//! Context window management for LLM conversations
//!
//! This module provides functionality for managing the context window of LLM conversations,
//! including token estimation, message pruning, automatic summarization, and auto-compaction.
//!
//! # Overview
//!
//! When conversations grow long, they can exceed the context window limits of LLMs.
//! This module provides tools to:
//!
//! - Estimate token usage before sending messages
//! - Prune old messages while preserving important context
//! - Automatically summarize conversation history
//! - Configure overflow strategies per provider/model
//! - Auto-compact when context exceeds threshold (like Claude Code)
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::context::{ContextConfig, ContextManager, OverflowStrategy, AutoCompact};
//! use sage_core::llm::LlmMessage;
//!
//! // Create context manager with default config
//! let config = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
//! let manager = ContextManager::new(config);
//!
//! // Prepare messages for LLM call
//! let messages = vec![
//!     LlmMessage::system("You are a helpful assistant"),
//!     LlmMessage::user("Hello!"),
//!     LlmMessage::assistant("Hi there!"),
//! ];
//!
//! let managed_messages = manager.prepare_messages(messages, None, "claude-3.5-sonnet").await?;
//!
//! // Or use auto-compact for automatic context management
//! let mut auto_compact = AutoCompact::default();
//! let result = auto_compact.check_and_compact(&mut messages).await?;
//! ```

pub mod auto_compact;
pub mod compact;
pub mod config;
pub mod estimator;
pub mod manager;
pub mod pruner;
pub mod streaming;
pub mod summarizer;

pub use auto_compact::{
    AUTOCOMPACT_PCT_OVERRIDE_ENV, AutoCompact, AutoCompactConfig, AutoCompactStats, CompactResult,
    DEFAULT_RESERVED_FOR_RESPONSE,
};
pub use compact::{
    COMPACT_BOUNDARY_KEY, COMPACT_ID_KEY, COMPACT_SUMMARY_KEY, COMPACT_TIMESTAMP_KEY,
    CompactOperationResult, SummaryPromptConfig, build_summary_prompt, create_compact_boundary,
    create_compact_summary, find_last_compact_boundary_index, is_compact_boundary,
    slice_from_last_compact_boundary,
};
pub use config::{ContextConfig, OverflowStrategy};
pub use estimator::TokenEstimator;
pub use manager::{ContextManager, ContextUsageStats, PrepareResult};
pub use pruner::{MessagePruner, PruneResult};
pub use streaming::{
    AggregatedStats, SharedStreamingMetrics, StreamingMetrics, StreamingStats,
    StreamingTokenCounter,
};
pub use summarizer::ConversationSummarizer;
