//! LLM client and message types

pub mod client;
pub mod converters;
pub mod fallback;
pub mod messages;
pub mod parsers;
pub mod provider_fallback;
pub mod provider_types;
pub mod providers;
pub mod rate_limiter;
pub mod sse_decoder;
pub mod streaming;

#[cfg(test)]
mod client_tests;

#[cfg(test)]
mod provider_fallback_tests;

pub use client::LlmClient;
pub use fallback::{
    FallbackChain, FallbackChainBuilder, FallbackEvent, FallbackReason, ModelConfig,
    ModelStats as FallbackModelStats, anthropic_fallback_chain, openai_fallback_chain,
};
pub use messages::{CacheControl, LlmMessage, LlmResponse, MessageRole};
pub use provider_fallback::ProviderFallbackClient;
pub use provider_types::{LlmProvider, TimeoutConfig};
pub use rate_limiter::{RateLimitConfig, RateLimiter};
pub use sse_decoder::{SseDecoder, SseEvent};
pub use streaming::{LlmStream, StreamChunk, StreamingLlmClient};
