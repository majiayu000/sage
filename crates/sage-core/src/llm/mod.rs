//! LLM client and message types

pub mod client;
pub mod fallback;
pub mod messages;
pub mod providers;
pub mod sse_decoder;
pub mod streaming;

pub use client::LLMClient;
pub use fallback::{
    FallbackChain, FallbackChainBuilder, FallbackEvent, FallbackReason, ModelConfig,
    ModelStats as FallbackModelStats, anthropic_fallback_chain, openai_fallback_chain,
};
pub use messages::{CacheControl, LLMMessage, LLMResponse, MessageRole};
pub use providers::LLMProvider;
pub use sse_decoder::{SSEDecoder, SSEEvent};
pub use streaming::{LLMStream, StreamChunk, StreamingLLMClient};
