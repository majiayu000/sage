---
name: sage-recovery-patterns
description: Sage 独有的恢复模式设计，包含熔断器、限流器、重试策略的最佳实践
when_to_use: 当需要实现容错机制、处理 LLM API 故障、或设计弹性系统时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
user_invocable: true
priority: 80
---

# Sage 恢复模式指南

## 概述

Sage 的 `recovery/` 模块是**独有的竞争优势**，提供生产级容错能力：

- **熔断器 (Circuit Breaker)**: 防止级联故障
- **限流器 (Rate Limiter)**: 控制 API 调用频率
- **重试策略 (Retry Policy)**: 智能重试失败操作
- **任务监督 (Supervisor)**: 管理长时间运行任务

## 熔断器模式

### 状态机

```
     成功调用
    ┌────────┐
    │        ▼
┌───────┐  失败次数 < 阈值  ┌───────┐
│Closed │ ─────────────────▶│Closed │
└───────┘                   └───┬───┘
    │                           │
    │ 失败次数 >= 阈值          │
    ▼                           │
┌───────┐                       │
│ Open  │ ◀────────────────────┘
└───┬───┘
    │ 超时后
    ▼
┌──────────┐
│Half-Open │
└────┬─────┘
     │
     ├── 成功 ──▶ Closed
     │
     └── 失败 ──▶ Open
```

### 配置

```rust
use sage_core::recovery::{CircuitBreaker, CircuitBreakerConfig};

let config = CircuitBreakerConfig {
    // 触发熔断的失败阈值
    failure_threshold: 5,

    // 熔断恢复超时（Open -> Half-Open）
    recovery_timeout: Duration::from_secs(30),

    // Half-Open 状态允许的探测请求数
    half_open_max_calls: 3,

    // 成功率阈值（低于此值触发熔断）
    success_rate_threshold: 0.5,

    // 统计窗口大小
    window_size: 10,
};

let breaker = CircuitBreaker::new("llm-api", config);
```

### 使用

```rust
// 基本使用
let result = breaker.call(|| async {
    llm_client.chat(request).await
}).await;

match result {
    Ok(response) => handle_success(response),
    Err(CircuitBreakerError::Open) => {
        // 熔断器开启，快速失败
        return fallback_response();
    }
    Err(CircuitBreakerError::Rejected(e)) => {
        // 被拒绝（Half-Open 状态超出限制）
        return Err(e);
    }
    Err(CircuitBreakerError::Failed(e)) => {
        // 调用失败
        return Err(e);
    }
}

// 检查状态
if breaker.is_open() {
    log::warn!("Circuit breaker is open, using fallback");
    return fallback();
}
```

### 与 LLM 提供者集成

```rust
pub struct ResilientLlmClient {
    client: Box<dyn LlmClient>,
    breaker: CircuitBreaker,
}

impl ResilientLlmClient {
    pub async fn chat(&self, request: ChatRequest) -> Result<LlmResponse> {
        self.breaker.call(|| async {
            self.client.chat(request.clone()).await
        }).await
        .map_err(|e| match e {
            CircuitBreakerError::Open => {
                LlmError::ServiceUnavailable("Circuit breaker open".into())
            }
            CircuitBreakerError::Failed(inner) => inner,
            _ => LlmError::Unknown(e.to_string()),
        })
    }
}
```

## 限流器模式

### 滑动窗口限流

```rust
use sage_core::recovery::SlidingWindowRateLimiter;

// 每分钟最多 100 次请求
let limiter = SlidingWindowRateLimiter::new(
    100,                        // 窗口内最大请求数
    Duration::from_secs(60),    // 窗口大小
);

// 获取令牌（阻塞直到可用）
limiter.acquire().await?;

// 非阻塞检查
if limiter.try_acquire() {
    // 有令牌，执行请求
} else {
    // 无令牌，返回 429
    return Err(RateLimitError::TooManyRequests);
}
```

### 令牌桶限流

```rust
use sage_core::recovery::TokenBucketRateLimiter;

let limiter = TokenBucketRateLimiter::new(
    10,                         // 桶容量
    Duration::from_millis(100), // 令牌生成间隔
);

// 消耗多个令牌（批量请求）
limiter.acquire_n(5).await?;
```

### 按提供者限流

```rust
use sage_core::recovery::RateLimiterRegistry;

let registry = RateLimiterRegistry::new();

// 为不同提供者设置不同限制
registry.register("anthropic", SlidingWindowRateLimiter::new(60, Duration::from_secs(60)));
registry.register("openai", SlidingWindowRateLimiter::new(100, Duration::from_secs(60)));
registry.register("google", SlidingWindowRateLimiter::new(30, Duration::from_secs(60)));

// 使用
let limiter = registry.get("anthropic")?;
limiter.acquire().await?;
```

## 重试策略

### 指数退避

```rust
use sage_core::recovery::{RetryPolicy, BackoffConfig};

let policy = RetryPolicy::exponential(BackoffConfig {
    initial_delay: Duration::from_millis(100),
    max_delay: Duration::from_secs(30),
    multiplier: 2.0,
    max_retries: 5,
    jitter: true,  // 添加随机抖动
});

let result = policy.retry(|| async {
    llm_client.chat(request.clone()).await
}).await?;
```

### 错误分类重试

```rust
use sage_core::recovery::{RetryPolicy, ErrorClass};

let policy = RetryPolicy::with_classifier(|error: &LlmError| {
    match error {
        // 可重试错误
        LlmError::RateLimit(_) => ErrorClass::Transient,
        LlmError::Timeout(_) => ErrorClass::Transient,
        LlmError::ServerError(_) => ErrorClass::Transient,

        // 不可重试错误
        LlmError::InvalidRequest(_) => ErrorClass::Permanent,
        LlmError::AuthError(_) => ErrorClass::Permanent,

        // 未知错误，谨慎重试
        _ => ErrorClass::Unknown,
    }
});
```

### 组合策略

```rust
// 熔断 + 限流 + 重试
pub async fn resilient_call<F, T, E>(
    breaker: &CircuitBreaker,
    limiter: &RateLimiter,
    retry_policy: &RetryPolicy,
    f: F,
) -> Result<T, E>
where
    F: Fn() -> Future<Output = Result<T, E>>,
    E: Into<RecoverableError>,
{
    // 1. 检查熔断器
    if breaker.is_open() {
        return Err(Error::CircuitOpen);
    }

    // 2. 获取限流令牌
    limiter.acquire().await?;

    // 3. 带重试执行
    let result = retry_policy.retry(|| async {
        breaker.call(|| f()).await
    }).await;

    result
}
```

## 任务监督

### 监督策略

```rust
use sage_core::recovery::{TaskSupervisor, SupervisionPolicy};

let supervisor = TaskSupervisor::new(SupervisionPolicy {
    // 最大重启次数
    max_restarts: 3,

    // 重启窗口（在此时间内超过最大重启次数则放弃）
    restart_window: Duration::from_secs(60),

    // 重启延迟策略
    restart_delay: BackoffConfig::exponential(
        Duration::from_secs(1),
        Duration::from_secs(30),
    ),

    // 失败处理
    on_max_restarts: SupervisionAction::Stop,
});
```

### 监督长时间任务

```rust
supervisor.supervise("agent-execution", || async {
    agent.execute(task).await
}).await?;
```

## 最佳实践

### 1. 分层恢复

```rust
// 内层：重试瞬时错误
let retry = RetryPolicy::exponential(config);

// 中层：熔断保护
let breaker = CircuitBreaker::new(config);

// 外层：限流控制
let limiter = RateLimiter::new(config);

// 组合使用
limiter.acquire().await?;
breaker.call(|| retry.retry(|| api_call())).await?;
```

### 2. 监控指标

```rust
// 记录熔断器状态
telemetry.gauge("circuit_breaker.state", breaker.state() as i64);
telemetry.counter("circuit_breaker.failures", breaker.failure_count());

// 记录限流
telemetry.counter("rate_limiter.rejected", limiter.rejected_count());
telemetry.histogram("rate_limiter.wait_time", limiter.avg_wait_time());

// 记录重试
telemetry.histogram("retry.attempts", retry_count);
telemetry.counter("retry.exhausted", exhausted_count);
```

### 3. 降级策略

```rust
async fn with_fallback<T>(
    primary: impl Future<Output = Result<T>>,
    fallback: impl Future<Output = Result<T>>,
) -> Result<T> {
    match primary.await {
        Ok(result) => Ok(result),
        Err(e) if is_recoverable(&e) => {
            log::warn!("Primary failed, using fallback: {}", e);
            fallback.await
        }
        Err(e) => Err(e),
    }
}

// 使用
let response = with_fallback(
    claude_client.chat(request.clone()),
    openai_client.chat(request.clone()),  // 降级到 OpenAI
).await?;
```

### 4. 健康检查

```rust
pub struct HealthChecker {
    breakers: HashMap<String, CircuitBreaker>,
}

impl HealthChecker {
    pub fn is_healthy(&self) -> bool {
        self.breakers.values().all(|b| !b.is_open())
    }

    pub fn health_report(&self) -> HealthReport {
        HealthReport {
            services: self.breakers.iter().map(|(name, breaker)| {
                (name.clone(), ServiceHealth {
                    state: breaker.state(),
                    failure_rate: breaker.failure_rate(),
                    last_failure: breaker.last_failure_time(),
                })
            }).collect(),
        }
    }
}
```

## 配置建议

### LLM API 熔断器

```rust
CircuitBreakerConfig {
    failure_threshold: 5,           // 5 次失败触发熔断
    recovery_timeout: 30.seconds(), // 30 秒后尝试恢复
    success_rate_threshold: 0.5,    // 50% 成功率阈值
    window_size: 10,                // 统计最近 10 次调用
}
```

### LLM API 限流器

```rust
// Anthropic
SlidingWindowRateLimiter::new(60, 60.seconds())   // 60 RPM

// OpenAI
SlidingWindowRateLimiter::new(100, 60.seconds())  // 100 RPM

// Google
SlidingWindowRateLimiter::new(30, 60.seconds())   // 30 RPM
```

### 重试策略

```rust
RetryPolicy::exponential(BackoffConfig {
    initial_delay: 100.milliseconds(),
    max_delay: 30.seconds(),
    multiplier: 2.0,
    max_retries: 5,
    jitter: true,
})
```
