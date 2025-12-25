//! Auto-Compact feature for automatic context management
//!
//! This module implements automatic context compression similar to Claude Code.
//! When the conversation context exceeds a configurable threshold (default 95%),
//! it automatically summarizes the conversation history to reduce token usage.
//!
//! ## Features
//!
//! - Automatic activation when context exceeds capacity threshold
//! - Configurable threshold (default 95% of max context)
//! - Compact boundary markers for recovery and chaining
//! - Claude Code style 9-section summary prompt
//! - Optional custom summarization instructions
//! - Manual trigger via `/compact` equivalent
//!
//! ## Usage
//!
//! ```ignore
//! let auto_compact = AutoCompact::new(config, llm_client);
//!
//! // Check and auto-compact if needed
//! let result = auto_compact.check_and_compact(&mut messages).await?;
//! if result.was_compacted {
//!     println!("Compacted {} messages, saved {} tokens", result.messages_compacted, result.tokens_saved);
//! }
//!
//! // Manual compact with custom instructions
//! auto_compact.compact_with_instructions(&mut messages, "Focus on code samples").await?;
//! ```

mod config;
mod manager;
mod operations;
mod partition;
mod result;
mod stats;
mod summary;

#[cfg(test)]
mod tests;

// Re-export public types
pub use config::{AUTOCOMPACT_PCT_OVERRIDE_ENV, AutoCompactConfig, DEFAULT_RESERVED_FOR_RESPONSE};
pub use manager::AutoCompact;
pub use result::CompactResult;
pub use stats::AutoCompactStats;
