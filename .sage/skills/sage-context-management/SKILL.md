---
name: sage-context-management
description: Sage 上下文管理开发指南，涵盖 Token 估算、消息裁剪、自动压缩、摘要生成
when_to_use: 当涉及上下文窗口管理、Token 计数、消息裁剪、Auto-Compact 时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 89
---

# Sage 上下文管理开发指南

## 模块概览

上下文模块管理 LLM 对话的上下文窗口，代码量 **4029 行**，包含：

```
crates/sage-core/src/context/
├── mod.rs              # 公开接口 (68行)
├── config.rs           # 配置 (263行)
├── estimator.rs        # Token 估算器 (223行)
├── pruner.rs           # 消息裁剪器 (409行)
├── summarizer.rs       # 摘要生成器 (359行)
├── compact.rs          # 压缩操作 (459行)
├── manager/            # 上下文管理器
│   ├── mod.rs          # 入口
│   ├── core.rs         # 核心实现 (148行)
│   ├── operations.rs   # 操作方法 (180行)
│   ├── types.rs        # 类型定义 (67行)
│   └── tests.rs        # 测试
├── auto_compact/       # 自动压缩（学习自 Claude Code）
│   ├── mod.rs          # 入口
│   ├── config.rs       # 配置
│   ├── manager.rs      # 管理器
│   ├── operations.rs   # 操作
│   ├── partition.rs    # 分区
│   ├── result.rs       # 结果
│   ├── stats.rs        # 统计
│   ├── summary.rs      # 摘要 (142行)
│   └── tests.rs        # 测试
└── streaming/          # 流式计数
    ├── mod.rs          # 入口
    ├── counter.rs      # 计数器 (157行)
    ├── metrics.rs      # 指标 (163行)
    ├── types.rs        # 类型 (79行)
    └── tests.rs        # 测试
```

---

## 一、核心架构

### 1.1 ContextManager

```rust
// crates/sage-core/src/context/manager/core.rs
#[derive(Clone)]
pub struct ContextManager {
    /// 配置
    pub(super) config: ContextConfig,
    /// Token 估算器
    pub(super) estimator: TokenEstimator,
    /// 消息裁剪器
    pub(super) pruner: MessagePruner,
    /// 摘要生成器
    pub(super) summarizer: ConversationSummarizer,
}

impl ContextManager {
    /// 创建管理器
    pub fn new(config: ContextConfig) -> Self {
        let estimator = TokenEstimator::new();
        let pruner = MessagePruner::new(config.clone());
        let summarizer = ConversationSummarizer::new();

        Self { config, estimator, pruner, summarizer }
    }

    /// 针对特定 Provider 优化
    pub fn for_provider(provider: &str, model: &str) -> Self {
        let config = ContextConfig::for_provider(provider, model);
        let estimator = TokenEstimator::for_provider(provider);
        // ...
    }

    /// 估算 Token 数
    pub fn estimate_tokens(&self, messages: &[LlmMessage]) -> usize {
        self.estimator.estimate_conversation(messages)
    }

    /// 检查是否接近限制
    pub fn is_approaching_limit(&self, messages: &[LlmMessage]) -> bool {
        let current_tokens = self.estimator.estimate_conversation(messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// 获取使用统计
    pub fn get_usage_stats(&self, messages: &[LlmMessage]) -> ContextUsageStats {
        let current_tokens = self.estimator.estimate_conversation(messages);
        let max_tokens = self.config.max_context_tokens;

        ContextUsageStats {
            current_tokens,
            max_tokens,
            usage_percentage: (current_tokens as f32 / max_tokens as f32) * 100.0,
            messages_count: messages.len(),
            // ...
        }
    }
}
```

### 1.2 配置结构

```rust
// crates/sage-core/src/context/config.rs
#[derive(Debug, Clone)]
pub struct ContextConfig {
    /// 最大上下文 Token 数
    pub max_context_tokens: usize,
    /// 触发裁剪的阈值比例 (0.0-1.0)
    pub threshold_ratio: f32,
    /// 溢出处理策略
    pub overflow_strategy: OverflowStrategy,
    /// 保留的系统消息数
    pub preserve_system_messages: bool,
    /// 保留的最近消息数
    pub preserve_recent_count: usize,
    /// 为响应预留的 Token 数
    pub reserved_for_response: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum OverflowStrategy {
    /// 裁剪旧消息
    Prune,
    /// 生成摘要
    Summarize,
    /// 自动压缩（Claude Code 风格）
    AutoCompact,
    /// 返回错误
    Error,
}

impl ContextConfig {
    /// 获取特定 Provider/Model 的配置
    pub fn for_provider(provider: &str, model: &str) -> Self {
        match provider {
            "anthropic" => match model {
                m if m.contains("opus") => Self::claude_opus(),
                m if m.contains("sonnet") => Self::claude_sonnet(),
                _ => Self::default(),
            },
            "openai" => match model {
                m if m.contains("gpt-4o") => Self::gpt4o(),
                m if m.contains("gpt-4") => Self::gpt4(),
                _ => Self::default(),
            },
            _ => Self::default(),
        }
    }

    /// 阈值 Token 数
    pub fn threshold_tokens(&self) -> usize {
        (self.max_context_tokens as f32 * self.threshold_ratio) as usize
    }
}
```

---

## 二、Token 估算

### 2.1 TokenEstimator

```rust
// crates/sage-core/src/context/estimator.rs
pub struct TokenEstimator {
    /// 每字符平均 Token 数
    chars_per_token: f32,
    /// Provider 特定调整
    provider_overhead: usize,
}

impl TokenEstimator {
    /// 估算消息 Token 数
    pub fn estimate_message(&self, message: &LlmMessage) -> usize {
        let content_tokens = self.estimate_text(&message.content);
        let role_tokens = 4; // 角色标记开销

        content_tokens + role_tokens + self.provider_overhead
    }

    /// 估算对话总 Token 数
    pub fn estimate_conversation(&self, messages: &[LlmMessage]) -> usize {
        messages.iter().map(|m| self.estimate_message(m)).sum()
    }

    /// 估算请求总 Token 数（含工具）
    pub fn estimate_request(
        &self,
        messages: &[LlmMessage],
        tools: Option<&[ToolSchema]>,
    ) -> usize {
        let conversation_tokens = self.estimate_conversation(messages);
        let tool_tokens = tools
            .map(|t| self.estimate_tools(t))
            .unwrap_or(0);

        conversation_tokens + tool_tokens
    }

    /// 估算文本 Token 数
    fn estimate_text(&self, text: &str) -> usize {
        (text.len() as f32 / self.chars_per_token).ceil() as usize
    }
}
```

### 2.2 Provider 特定估算

```rust
impl TokenEstimator {
    pub fn for_provider(provider: &str) -> Self {
        match provider {
            "anthropic" => Self {
                chars_per_token: 4.0,
                provider_overhead: 8,
            },
            "openai" => Self {
                chars_per_token: 4.0,
                provider_overhead: 4,
            },
            "google" => Self {
                chars_per_token: 4.5,
                provider_overhead: 6,
            },
            _ => Self::default(),
        }
    }
}
```

---

## 三、消息裁剪

### 3.1 MessagePruner

```rust
// crates/sage-core/src/context/pruner.rs
pub struct MessagePruner {
    config: ContextConfig,
}

impl MessagePruner {
    /// 裁剪消息到目标 Token 数
    pub fn prune(&self, messages: Vec<LlmMessage>, target_tokens: usize) -> PruneResult {
        let mut result = messages.clone();
        let mut removed_count = 0;

        // 1. 保留系统消息
        let system_messages: Vec<_> = result
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .cloned()
            .collect();

        // 2. 保留最近消息
        let recent_count = self.config.preserve_recent_count;
        let recent_messages: Vec<_> = result
            .iter()
            .rev()
            .take(recent_count)
            .cloned()
            .collect();

        // 3. 从中间裁剪
        while self.estimate_tokens(&result) > target_tokens {
            if result.len() <= recent_count + system_messages.len() {
                break;
            }

            // 移除最早的非系统消息
            let idx = result.iter().position(|m| m.role != MessageRole::System);
            if let Some(i) = idx {
                result.remove(i);
                removed_count += 1;
            }
        }

        PruneResult {
            messages: result,
            removed_count,
            tokens_saved: self.estimate_tokens(&messages) - self.estimate_tokens(&result),
        }
    }
}

#[derive(Debug)]
pub struct PruneResult {
    pub messages: Vec<LlmMessage>,
    pub removed_count: usize,
    pub tokens_saved: usize,
}
```

---

## 四、自动压缩（Auto-Compact）

### 4.1 设计理念（学习自 Claude Code）

当上下文接近限制时，自动压缩历史对话为摘要：

```
原始对话:
[System] You are a helpful assistant
[User] Hello
[Assistant] Hi there!
[User] What is Rust?
[Assistant] Rust is a systems programming language...
[User] Show me an example
[Assistant] Here's a simple example...
... (更多消息) ...

压缩后:
[System] You are a helpful assistant
[Compact Boundary] <summary of previous conversation>
[User] (最近的消息)
[Assistant] (最近的响应)
```

### 4.2 AutoCompact 配置

```rust
// crates/sage-core/src/context/auto_compact/config.rs
pub struct AutoCompactConfig {
    /// 是否启用
    pub enabled: bool,
    /// 最大上下文 Token 数
    pub max_context_tokens: usize,
    /// 触发压缩的比例
    pub compact_threshold_ratio: f32,
    /// 为响应预留的 Token 数
    pub reserved_for_response: usize,
    /// 目标压缩后大小比例
    pub target_ratio: f32,
}

impl AutoCompactConfig {
    /// 从环境变量覆盖
    pub fn with_env_override(mut self) -> Self {
        if let Ok(val) = std::env::var(AUTOCOMPACT_PCT_OVERRIDE_ENV) {
            if let Ok(pct) = val.parse::<f32>() {
                self.compact_threshold_ratio = pct / 100.0;
            }
        }
        self
    }
}
```

### 4.3 AutoCompact 管理器

```rust
// crates/sage-core/src/context/auto_compact/manager.rs
pub struct AutoCompact {
    config: AutoCompactConfig,
    llm_client: Option<Arc<LlmClient>>,
    stats: AutoCompactStats,
}

impl AutoCompact {
    /// 检查是否需要压缩
    pub fn needs_compaction(&self, messages: &[LlmMessage]) -> bool {
        if !self.config.enabled {
            return false;
        }

        // 只考虑上次压缩边界之后的消息
        let active_messages = slice_from_last_compact_boundary(messages);
        let current_tokens = self.estimate_tokens(&active_messages);
        current_tokens >= self.config.threshold_tokens()
    }

    /// 检查并自动压缩
    pub async fn check_and_compact(
        &mut self,
        messages: &mut Vec<LlmMessage>,
    ) -> SageResult<CompactResult> {
        if !self.needs_compaction(messages) {
            self.stats.skipped_count += 1;
            return Ok(CompactResult::Skipped);
        }

        // 执行压缩
        let result = self.compact(messages).await?;
        self.stats.compact_count += 1;
        self.stats.tokens_saved += result.tokens_saved;

        Ok(result)
    }

    /// 执行压缩
    async fn compact(&self, messages: &mut Vec<LlmMessage>) -> SageResult<CompactResult> {
        // 1. 获取活跃消息（边界之后）
        let boundary_idx = find_last_compact_boundary_index(messages);
        let active_start = boundary_idx.map(|i| i + 1).unwrap_or(0);

        // 2. 分区：保留部分 + 压缩部分
        let (to_compact, to_keep) = partition::partition_messages(
            &messages[active_start..],
            self.config.preserve_recent_count,
        );

        // 3. 生成摘要
        let summary = if let Some(client) = &self.llm_client {
            summary::generate_summary(client, &to_compact).await?
        } else {
            summary::simple_summary(&to_compact)
        };

        // 4. 创建压缩边界
        let compact_boundary = create_compact_boundary(&summary);

        // 5. 重建消息列表
        let preserved = &messages[..active_start];
        let new_messages = [
            preserved,
            &[compact_boundary],
            &to_keep,
        ].concat();

        let tokens_before = self.estimate_tokens(messages);
        let tokens_after = self.estimate_tokens(&new_messages);

        *messages = new_messages;

        Ok(CompactResult::Compacted {
            messages_compacted: to_compact.len(),
            tokens_saved: tokens_before - tokens_after,
        })
    }
}
```

### 4.4 压缩边界

```rust
// crates/sage-core/src/context/compact.rs
pub const COMPACT_BOUNDARY_KEY: &str = "compact_boundary";
pub const COMPACT_SUMMARY_KEY: &str = "compact_summary";
pub const COMPACT_TIMESTAMP_KEY: &str = "compact_timestamp";
pub const COMPACT_ID_KEY: &str = "compact_id";

/// 创建压缩边界消息
pub fn create_compact_boundary(summary: &str) -> LlmMessage {
    let mut metadata = HashMap::new();
    metadata.insert(COMPACT_BOUNDARY_KEY.to_string(), json!(true));
    metadata.insert(COMPACT_SUMMARY_KEY.to_string(), json!(summary));
    metadata.insert(COMPACT_TIMESTAMP_KEY.to_string(), json!(Utc::now().to_rfc3339()));
    metadata.insert(COMPACT_ID_KEY.to_string(), json!(Uuid::new_v4().to_string()));

    LlmMessage::assistant(format!(
        "[Previous conversation summary]\n\n{}\n\n[End of summary]",
        summary
    )).with_metadata(metadata)
}

/// 检查是否为压缩边界
pub fn is_compact_boundary(message: &LlmMessage) -> bool {
    message.metadata
        .get(COMPACT_BOUNDARY_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// 获取最后压缩边界索引
pub fn find_last_compact_boundary_index(messages: &[LlmMessage]) -> Option<usize> {
    messages.iter().rposition(is_compact_boundary)
}

/// 从最后边界切片
pub fn slice_from_last_compact_boundary(messages: &[LlmMessage]) -> &[LlmMessage] {
    match find_last_compact_boundary_index(messages) {
        Some(idx) => &messages[idx + 1..],
        None => messages,
    }
}
```

---

## 五、摘要生成

### 5.1 ConversationSummarizer

```rust
// crates/sage-core/src/context/summarizer.rs
pub struct ConversationSummarizer {
    llm_client: Option<Arc<LlmClient>>,
}

impl ConversationSummarizer {
    /// 使用 LLM 生成摘要
    pub async fn summarize(&self, messages: &[LlmMessage]) -> SageResult<String> {
        if let Some(client) = &self.llm_client {
            self.summarize_with_llm(client, messages).await
        } else {
            Ok(self.simple_summarize(messages))
        }
    }

    /// LLM 摘要
    async fn summarize_with_llm(
        &self,
        client: &LlmClient,
        messages: &[LlmMessage],
    ) -> SageResult<String> {
        let prompt = build_summary_prompt(messages);
        let response = client.chat(&[LlmMessage::user(prompt)], None).await?;
        Ok(response.content)
    }

    /// 简单摘要（无 LLM）
    fn simple_summarize(&self, messages: &[LlmMessage]) -> String {
        let user_count = messages.iter().filter(|m| m.role == MessageRole::User).count();
        let assistant_count = messages.iter().filter(|m| m.role == MessageRole::Assistant).count();

        format!(
            "Previous conversation with {} user messages and {} assistant responses.",
            user_count, assistant_count
        )
    }
}
```

---

## 六、流式 Token 计数

### 6.1 StreamingTokenCounter

```rust
// crates/sage-core/src/context/streaming/counter.rs
pub struct StreamingTokenCounter {
    estimator: TokenEstimator,
    input_tokens: AtomicUsize,
    output_tokens: AtomicUsize,
}

impl StreamingTokenCounter {
    /// 记录输入 Token
    pub fn record_input(&self, text: &str) {
        let tokens = self.estimator.estimate_text(text);
        self.input_tokens.fetch_add(tokens, Ordering::SeqCst);
    }

    /// 记录输出 Token
    pub fn record_output(&self, chunk: &str) {
        let tokens = self.estimator.estimate_text(chunk);
        self.output_tokens.fetch_add(tokens, Ordering::SeqCst);
    }

    /// 获取统计
    pub fn get_stats(&self) -> StreamingStats {
        StreamingStats {
            input_tokens: self.input_tokens.load(Ordering::SeqCst),
            output_tokens: self.output_tokens.load(Ordering::SeqCst),
        }
    }
}
```

---

## 七、使用示例

### 7.1 基本使用

```rust
use sage_core::context::{ContextManager, ContextConfig};

// 创建管理器
let config = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
let manager = ContextManager::new(config);

// 检查上下文使用
let stats = manager.get_usage_stats(&messages);
println!("Usage: {:.1}%", stats.usage_percentage);

// 检查是否需要处理
if manager.is_approaching_limit(&messages) {
    let pruned = manager.prune(messages.clone(), 8000);
    println!("Removed {} messages", pruned.removed_count);
}
```

### 7.2 Auto-Compact 使用

```rust
use sage_core::context::{AutoCompact, AutoCompactConfig};

let config = AutoCompactConfig::default()
    .with_env_override();

let mut auto_compact = AutoCompact::with_llm_client(config, llm_client);

// 每次 LLM 调用前检查
let result = auto_compact.check_and_compact(&mut messages).await?;

match result {
    CompactResult::Compacted { messages_compacted, tokens_saved } => {
        println!("Compacted {} messages, saved {} tokens", messages_compacted, tokens_saved);
    }
    CompactResult::Skipped => {
        // 不需要压缩
    }
}
```

---

## 八、相关模块

- `sage-llm-integration` - LLM 客户端（用于摘要生成）
- `sage-session-management` - 会话管理（消息存储）
- `sage-agent-execution` - Agent 执行（上下文集成）

---

*最后更新: 2026-01-10*
