//! LLM client and message types

pub mod client;
pub mod converters;
pub mod fallback;
pub mod messages;
pub mod parsers;
pub mod providers;
pub mod rate_limiter;
pub mod sse_decoder;
pub mod streaming;

#[cfg(test)]
mod client_tests;

pub use client::LLMClient;
pub use fallback::{
    FallbackChain, FallbackChainBuilder, FallbackEvent, FallbackReason, ModelConfig,
    ModelStats as FallbackModelStats, anthropic_fallback_chain, openai_fallback_chain,
};
pub use messages::{CacheControl, LLMMessage, LLMResponse, MessageRole};
pub use providers::{LLMProvider, TimeoutConfig};
pub use rate_limiter::{RateLimitConfig, RateLimiter};
pub use sse_decoder::{SSEDecoder, SSEEvent};
pub use streaming::{LLMStream, StreamChunk, StreamingLLMClient};
