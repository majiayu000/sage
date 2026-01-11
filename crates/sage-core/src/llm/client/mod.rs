//! LLM client implementation
//!
//! Provides a unified interface for interacting with multiple LLM providers
//! (OpenAI, Anthropic, Google, Azure, etc.) with automatic retry logic,
//! rate limiting, circuit breaker protection, and streaming support.

mod accessors;
mod chat;
mod constructor;
mod error_check;
mod retry;
mod streaming;
#[cfg(test)]
mod tests;
mod types;

pub use types::LlmClient;
