//! LLM client implementation
//!
//! Provides a unified interface for interacting with multiple LLM providers
//! (OpenAI, Anthropic, Google, Azure, etc.) with automatic retry logic,
//! rate limiting, and streaming support.

mod accessors;
mod chat;
mod constructor;
mod error_check;
mod retry;
mod streaming;
mod types;

pub use types::LlmClient;
