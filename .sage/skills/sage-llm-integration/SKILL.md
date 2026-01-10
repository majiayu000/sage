---
name: sage-llm-integration
description: Sage LLM 客户端集成开发指南，涵盖多 Provider 支持、Fallback、Rate Limiting、Streaming
when_to_use: 当涉及 LLM 客户端、Provider 实现、流式响应、限流策略时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 94
---

# Sage LLM 集成开发指南

## 模块概览

LLM 模块是 Sage 与大语言模型交互的核心层，代码量 **8722 行**，包含：

```
crates/sage-core/src/llm/
├── mod.rs              # 公开接口
├── messages.rs         # 消息类型定义
├── model_capabilities.rs # 模型能力定义
├── provider_types.rs   # Provider 类型 (320行)
├── provider_fallback.rs # Provider 级别 Fallback (83行)
├── sse_decoder.rs      # SSE 解码器 (609行)
├── streaming.rs        # 流式响应 (385行)
├── client/             # LLM 客户端
│   ├── mod.rs          # 入口
│   ├── types.rs        # LlmClient 定义
│   ├── chat.rs         # Chat 请求
│   ├── streaming.rs    # 流式请求
│   ├── retry.rs        # 重试逻辑
│   ├── constructor.rs  # 构造器
│   ├── accessors.rs    # 访问器
│   └── error_check.rs  # 错误检查
├── converters/         # 消息转换
│   ├── messages.rs     # 消息格式转换
│   └── tools.rs        # 工具格式转换
├── fallback/           # Fallback 机制
│   ├── manager.rs      # FallbackChain (159行)
│   ├── builder.rs      # 构建器
│   ├── state.rs        # 模型状态
│   ├── types.rs        # 类型定义
│   ├── operations.rs   # 操作方法
│   └── tests/          # 测试
├── parsers/            # 响应解析
│   ├── responses.rs    # 响应解析器
│   └── responses_tests.rs
├── providers/          # Provider 实现
│   ├── provider_trait.rs # 统一 trait (74行)
│   ├── openai.rs       # OpenAI (204行)
│   ├── anthropic.rs    # Anthropic (443行)
│   ├── google.rs       # Google (141行)
│   ├── azure.rs        # Azure (233行)
│   ├── openrouter.rs   # OpenRouter (239行)
│   ├── ollama.rs       # Ollama (219行)
│   ├── doubao.rs       # Doubao (216行)
│   └── glm.rs          # GLM/智谱 (420行)
└── rate_limiter/       # 限流器
    ├── bucket.rs       # Token Bucket (141行)
    ├── limiter.rs      # 限流器 (51行)
    ├── types.rs        # 类型定义 (88行)
    └── tests.rs        # 测试 (287行)
```

---

## 一、核心架构：LlmClient

### 1.1 设计理念

Sage 采用**统一客户端 + 多 Provider 实例**的设计模式：

```rust
// crates/sage-core/src/llm/client/types.rs
pub struct LlmClient {
    /// Provider 类型
    pub(super) provider: LlmProvider,

    /// Provider 配置
    pub(super) config: ProviderConfig,

    /// 模型参数
    pub(super) model_params: ModelParameters,

    /// Provider 实例（实际执行请求）
    pub(super) provider_instance: ProviderInstance,
}
```

### 1.2 Provider Trait 设计

所有 Provider 实现统一的 trait：

```rust
// crates/sage-core/src/llm/providers/provider_trait.rs
#[async_trait]
pub trait LlmProviderTrait: Send + Sync {
    /// 发送 Chat 请求
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse>;

    /// 发送流式 Chat 请求
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream>;
}
```

### 1.3 Provider 实例枚举

```rust
// crates/sage-core/src/llm/providers/provider_trait.rs
pub enum ProviderInstance {
    OpenAI(OpenAiProvider),
    Anthropic(AnthropicProvider),
    Google(GoogleProvider),
    Azure(AzureProvider),
    OpenRouter(OpenRouterProvider),
    Ollama(OllamaProvider),
    Doubao(DoubaoProvider),
    Glm(GlmProvider),
}

#[async_trait]
impl LlmProviderTrait for ProviderInstance {
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        match self {
            Self::OpenAI(p) => p.chat(messages, tools).await,
            Self::Anthropic(p) => p.chat(messages, tools).await,
            // ... 其他 provider
        }
    }
}
```

---

## 二、多 Provider 支持

### 2.1 支持的 Provider 列表

| Provider | 模型示例 | 特殊功能 |
|----------|---------|---------|
| OpenAI | gpt-4o, gpt-4-turbo | Function calling |
| Anthropic | claude-3-5-sonnet | Tool use, Cache Control |
| Google | gemini-1.5-pro | 长上下文 |
| Azure | gpt-4 (部署) | 企业级 |
| OpenRouter | 多模型聚合 | 统一接口 |
| Ollama | llama3, codellama | 本地部署 |
| Doubao | doubao-pro | 字节跳动 |
| GLM | glm-4, glm-4-plus | 智谱 AI |

### 2.2 添加新 Provider 模式

1. 创建 Provider 实现文件：

```rust
// crates/sage-core/src/llm/providers/new_provider.rs
pub struct NewProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[async_trait]
impl LlmProviderTrait for NewProvider {
    async fn chat(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        // 1. 转换消息格式
        let request_body = self.build_request(messages, tools)?;

        // 2. 发送请求
        let response = self.client
            .post(&format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        // 3. 解析响应
        self.parse_response(response).await
    }
}
```

2. 在 `provider_trait.rs` 添加枚举变体：

```rust
pub enum ProviderInstance {
    // ... 现有
    NewProvider(super::NewProvider),
}
```

3. 在 `LlmProvider` 枚举添加：

```rust
// crates/sage-core/src/llm/provider_types.rs
pub enum LlmProvider {
    // ... 现有
    NewProvider,
}
```

---

## 三、Fallback 机制

### 3.1 模型级 Fallback（FallbackChain）

用于同一 Provider 内的模型降级：

```rust
// crates/sage-core/src/llm/fallback/manager.rs
pub struct FallbackChain {
    /// 模型链
    pub(super) models: Arc<RwLock<Vec<ModelState>>>,
    /// 当前模型索引
    pub(super) current_index: Arc<RwLock<usize>>,
    /// Fallback 历史
    pub(super) history: Arc<RwLock<Vec<FallbackEvent>>>,
    /// 最大历史记录数
    pub(super) max_history: usize,
}
```

**Fallback 触发逻辑：**

```rust
impl FallbackChain {
    /// 记录失败并触发 Fallback
    pub async fn record_failure(
        &self,
        model_id: &str,
        reason: FallbackReason
    ) -> Option<String> {
        let mut models = self.models.write().await;

        if let Some(index) = models.iter().position(|m| m.config.model_id == model_id) {
            models[index].record_failure();

            // 达到重试上限，切换到下一个模型
            if models[index].failure_count >= models[index].config.max_retries {
                drop(models);
                return self.next_available(None).await;
            }
        }

        None
    }
}
```

### 3.2 Provider 级 Fallback

用于跨 Provider 降级（quota/rate limit 错误）：

```rust
// crates/sage-core/src/llm/provider_fallback.rs
pub struct ProviderFallbackClient {
    clients: Vec<LlmClient>,
    current_index: usize,
}

impl ProviderFallbackClient {
    pub async fn chat(
        &mut self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmResponse> {
        let mut last_error = None;

        for attempt in 0..self.clients.len() {
            let client = &self.clients[self.current_index];

            match client.chat(messages, tools).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    last_error = Some(error.clone());

                    // 判断是否应该切换 Provider
                    if client.should_fallback_provider(&error) {
                        warn!("Provider {} 限流，切换到下一个...",
                            client.provider().name());
                        self.current_index = (self.current_index + 1) % self.clients.len();
                        continue;
                    }

                    return Err(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SageError::llm("所有 Provider 已耗尽")))
    }
}
```

### 3.3 Fallback 原因类型

```rust
// crates/sage-core/src/llm/fallback/types.rs
#[derive(Debug, Clone)]
pub enum FallbackReason {
    /// API 错误
    ApiError(String),
    /// 超时
    Timeout,
    /// 速率限制
    RateLimited,
    /// 配额超限
    QuotaExceeded,
    /// 上下文超长
    ContextTooLong,
    /// 模型不可用
    ModelUnavailable,
    /// 手动触发
    Manual,
}
```

---

## 四、Rate Limiting

### 4.1 Token Bucket 算法

```rust
// crates/sage-core/src/llm/rate_limiter/types.rs
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 每分钟请求数
    pub requests_per_minute: u32,
    /// 突发大小（允许短期超过持续速率）
    pub burst_size: u32,
    /// 是否启用
    pub enabled: bool,
}

impl RateLimitConfig {
    /// 获取 Provider 特定配置
    pub fn for_provider(provider: &str) -> Self {
        match provider.to_lowercase().as_str() {
            "openai" => Self::new(60, 20),
            "anthropic" => Self::new(60, 10),
            "google" => Self::new(60, 15),
            "azure" => Self::new(60, 20),
            "ollama" => Self::new(120, 30),  // 本地，可更宽松
            "glm" => Self::new(60, 15),
            _ => Self::default(),
        }
    }
}
```

### 4.2 限流器实现

```rust
// crates/sage-core/src/llm/rate_limiter/limiter.rs
pub struct RateLimiter {
    config: RateLimitConfig,
    state: Arc<Mutex<RateLimiterState>>,
}

impl RateLimiter {
    /// 等待获取令牌
    pub async fn acquire(&self) -> SageResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        loop {
            let mut state = self.state.lock().await;

            // 补充令牌
            self.refill_tokens(&mut state);

            if state.tokens >= 1.0 {
                state.tokens -= 1.0;
                return Ok(());
            }

            // 计算等待时间
            let wait_time = self.time_until_token(&state);
            drop(state);

            tokio::time::sleep(wait_time).await;
        }
    }
}
```

---

## 五、流式响应

### 5.1 StreamChunk 设计

```rust
// crates/sage-core/src/llm/streaming.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// 增量内容
    pub content: Option<String>,
    /// 工具调用
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 使用统计（通常只在最后一个 chunk）
    pub usage: Option<LlmUsage>,
    /// 是否为最终 chunk
    pub is_final: bool,
    /// 结束原因
    pub finish_reason: Option<String>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 流式响应类型
pub type LlmStream = Pin<Box<dyn Stream<Item = SageResult<StreamChunk>> + Send>>;
```

### 5.2 StreamingLlmClient Trait

```rust
#[async_trait]
pub trait StreamingLlmClient {
    async fn chat_stream(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> SageResult<LlmStream>;
}
```

### 5.3 流工具函数

```rust
pub mod stream_utils {
    /// 收集流到完整响应
    pub async fn collect_stream(mut stream: LlmStream) -> SageResult<LlmResponse> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        let mut usage = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;

            if let Some(chunk_content) = chunk.content {
                content.push_str(&chunk_content);
            }

            if let Some(chunk_tool_calls) = chunk.tool_calls {
                tool_calls.extend(chunk_tool_calls);
            }

            if chunk.is_final {
                usage = chunk.usage;
            }
        }

        Ok(LlmResponse { content, tool_calls, usage, .. })
    }

    /// 带取消支持的流收集
    pub async fn collect_stream_with_cancel(
        mut stream: LlmStream,
        cancel_token: &CancellationToken,
    ) -> SageResult<LlmResponse> {
        while let Some(chunk_result) = stream.next().await {
            // 每个 chunk 后检查取消
            if cancel_token.is_cancelled() {
                return Err(SageError::Cancelled);
            }
            // ... 处理 chunk
        }
        // ...
    }

    /// 只保留内容 chunk
    pub fn content_only(stream: LlmStream) -> LlmStream;

    /// 批量缓冲 chunk
    pub fn buffer_chunks(stream: LlmStream, buffer_size: usize) -> LlmStream;

    /// 添加时间信息
    pub fn with_timing(stream: LlmStream) -> LlmStream;
}
```

### 5.4 SSE 支持

```rust
pub mod sse {
    /// SSE 事件
    #[derive(Debug, Clone)]
    pub struct SseEvent {
        pub event: Option<String>,
        pub data: String,
        pub id: Option<String>,
        pub retry: Option<u64>,
    }

    /// 转换流到 SSE
    pub fn stream_to_sse(
        stream: LlmStream,
    ) -> Pin<Box<dyn Stream<Item = SageResult<SseEvent>> + Send>> {
        Box::pin(stream.map(|chunk_result|
            chunk_result.and_then(chunk_to_sse)
        ))
    }
}
```

---

## 六、SSE 解码器

### 6.1 解码器设计

```rust
// crates/sage-core/src/llm/sse_decoder.rs
pub struct SseDecoder {
    buffer: String,
    events: VecDeque<SseEvent>,
}

impl SseDecoder {
    /// 添加数据并解析事件
    pub fn decode(&mut self, data: &str) -> Vec<SseEvent> {
        self.buffer.push_str(data);
        let mut events = Vec::new();

        // 按双换行分割事件
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_str = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            if let Some(event) = self.parse_event(&event_str) {
                events.push(event);
            }
        }

        events
    }
}
```

---

## 七、消息类型

### 7.1 LlmMessage

```rust
// crates/sage-core/src/llm/messages.rs
pub struct LlmMessage {
    pub role: MessageRole,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub cache_control: Option<CacheControl>,  // Anthropic 特有
}

pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl LlmMessage {
    pub fn system(content: impl Into<String>) -> Self;
    pub fn user(content: impl Into<String>) -> Self;
    pub fn assistant(content: impl Into<String>) -> Self;
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self;
}
```

### 7.2 LlmResponse

```rust
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<LlmUsage>,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
    pub id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

---

## 八、开发指南

### 8.1 添加新 Provider 检查清单

- [ ] 创建 `providers/new_provider.rs`
- [ ] 实现 `LlmProviderTrait`
- [ ] 添加到 `ProviderInstance` 枚举
- [ ] 添加到 `LlmProvider` 枚举
- [ ] 在 `RateLimitConfig::for_provider` 添加配置
- [ ] 添加消息格式转换（如果 API 格式不同）
- [ ] 添加测试

### 8.2 流式响应处理模式

```rust
// 推荐模式：使用 collect_stream_with_cancel
let cancel_token = CancellationToken::new();
let stream = client.chat_stream(&messages, tools).await?;

tokio::select! {
    result = stream_utils::collect_stream_with_cancel(stream, &cancel_token) => {
        result?
    }
    _ = some_cancel_signal => {
        cancel_token.cancel();
        return Ok(());
    }
}
```

### 8.3 错误处理最佳实践

```rust
// 使用 should_fallback_provider 判断是否切换
match client.chat(&messages, tools).await {
    Ok(response) => response,
    Err(e) if client.should_fallback_provider(&e) => {
        // 切换到备用 Provider
        backup_client.chat(&messages, tools).await?
    }
    Err(e) => return Err(e),
}
```

---

## 九、与其他模块关系

```
┌─────────────────────────────────────────────────────────────┐
│                       Agent (执行器)                          │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                    LlmClient                         │    │
│  │                         │                            │    │
│  │    ┌────────────────────┼────────────────────┐      │    │
│  │    ▼                    ▼                    ▼      │    │
│  │ FallbackChain    RateLimiter          Streaming     │    │
│  │    │                    │                    │      │    │
│  │    └────────────────────┼────────────────────┘      │    │
│  │                         ▼                           │    │
│  │              ProviderInstance                       │    │
│  │    ┌────────────────────┼────────────────────┐      │    │
│  │    ▼         ▼          ▼         ▼          ▼      │    │
│  │ OpenAI  Anthropic   Google    Azure    Ollama  ...  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

---

## 十、相关模块

- `sage-agent-execution` - Agent 执行引擎
- `sage-config-system` - 配置系统（Provider 配置）
- `sage-recovery-patterns` - 恢复模式（熔断器/限流器）
- `sage-tool-development` - 工具开发（ToolSchema）

---

*最后更新: 2026-01-10*
