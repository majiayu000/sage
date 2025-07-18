//! Sage Agent Core Library
//! 
//! This crate provides the core functionality for the Sage Agent system,
//! including agent execution, LLM integration, tool management, and configuration.

pub mod agent;
pub mod cache;
pub mod config;
pub mod error;
pub mod interrupt;
pub mod llm;
pub mod tools;
pub mod trajectory;
pub mod types;
pub mod ui;

// TODO: Add MCP (Model Context Protocol) support
// pub mod mcp;  // Uncomment when implementing MCP integration
// - Add MCP client and server implementations
// - Support multiple transport layers (stdio, HTTP, WebSocket)
// - Implement MCP tool discovery and registration
// - Add MCP resource management capabilities

// TODO: Add plugin system
// pub mod plugins;  // Uncomment when implementing plugin system
// - Design plugin API and lifecycle management
// - Add plugin security validation and sandboxing
// - Support dynamic plugin loading and unloading
// - Implement plugin marketplace integration

// Re-export commonly used types
pub use agent::{Agent, AgentExecution, AgentStep, AgentState};
pub use cache::{CacheManager, LLMCache, CacheKey, CacheEntry, CacheConfig};
pub use config::{Config, ModelParameters, LakeviewConfig};
pub use error::{SageError, SageResult};
pub use interrupt::{InterruptManager, InterruptReason, TaskScope};
pub use llm::{LLMClient, LLMMessage, LLMResponse, LLMProvider};
pub use tools::{Tool, ToolCall, ToolExecutor, ToolResult};
pub use trajectory::TrajectoryRecorder;
pub use types::*;
