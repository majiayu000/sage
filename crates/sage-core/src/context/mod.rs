//! Context window management for LLM conversations
//!
//! This module provides functionality for managing the context window of LLM conversations,
//! including token estimation, message pruning, and automatic summarization.
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
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::context::{ContextConfig, ContextManager, OverflowStrategy};
//! use sage_core::llm::LLMMessage;
//!
//! // Create context manager with default config
//! let config = ContextConfig::for_provider("anthropic", "claude-3.5-sonnet");
//! let manager = ContextManager::new(config);
//!
//! // Prepare messages for LLM call
//! let messages = vec![
//!     LLMMessage::system("You are a helpful assistant"),
//!     LLMMessage::user("Hello!"),
//!     LLMMessage::assistant("Hi there!"),
//! ];
//!
//! let managed_messages = manager.prepare_messages(messages, None, "claude-3.5-sonnet").await?;
//! ```

pub mod config;
pub mod estimator;
pub mod manager;
pub mod pruner;
pub mod streaming;
pub mod summarizer;

pub use config::{ContextConfig, OverflowStrategy};
pub use estimator::TokenEstimator;
pub use manager::{ContextManager, ContextUsageStats, PrepareResult};
pub use pruner::{MessagePruner, PruneResult};
pub use streaming::{
    AggregatedStats, SharedStreamingMetrics, StreamingMetrics, StreamingStats,
    StreamingTokenCounter,
};
pub use summarizer::ConversationSummarizer;
