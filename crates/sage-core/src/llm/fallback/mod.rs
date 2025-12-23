//! Model fallback chain
//!
//! This module provides automatic fallback to alternative models
//! when the primary model fails or is rate limited.

mod builder;
mod manager;
mod operations;
mod state;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types for backward compatibility
pub use builder::{anthropic_fallback_chain, openai_fallback_chain, FallbackChainBuilder};
pub use manager::FallbackChain;
pub use types::{FallbackEvent, FallbackReason, ModelConfig, ModelStats};
