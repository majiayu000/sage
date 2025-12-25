//! JSONL storage for enhanced messages
//!
//! This module provides JSONL-based storage for enhanced messages,
//! following the Claude Code pattern of storing one JSON object per line.
//!
//! # File Format
//!
//! Each session is stored as a directory containing:
//! - `messages.jsonl` - One enhanced message per line
//! - `snapshots.jsonl` - File history snapshots
//! - `metadata.json` - Session metadata
//!
//! # Example
//!
//! ```jsonl
//! {"type":"user","uuid":"...","parentUuid":null,...}
//! {"type":"assistant","uuid":"...","parentUuid":"...",...}
//! {"type":"tool_result","uuid":"...","parentUuid":"...",...}
//! ```

pub mod metadata;
pub mod storage;
mod tracker;

#[cfg(test)]
mod tests;

// Re-export public types
pub use metadata::SessionMetadata;
pub use storage::JsonlSessionStorage;
pub use tracker::MessageChainTracker;
