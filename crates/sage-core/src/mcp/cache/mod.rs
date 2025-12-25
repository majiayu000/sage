//! MCP Resource Cache
//!
//! Provides caching for MCP resources, tools, and prompts to reduce
//! redundant requests to MCP servers.

mod cache;
mod eviction;
mod types;

#[cfg(test)]
mod tests;

pub use cache::McpCache;
pub use types::{CacheConfig, CacheSizeBreakdown, CacheStats};
