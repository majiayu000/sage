//! Sage Agent Core Library
//! 
//! This crate provides the core functionality for the Sage Agent system,
//! including agent execution, LLM integration, tool management, and configuration.

pub mod agent;
pub mod config;
pub mod error;
pub mod llm;
pub mod tools;
pub mod trajectory;
pub mod types;
pub mod ui;

// Re-export commonly used types
pub use agent::{Agent, AgentExecution, AgentStep, AgentState};
pub use config::{Config, ModelParameters, LakeviewConfig};
pub use error::{SageError, SageResult};
pub use llm::{LLMClient, LLMMessage, LLMResponse, LLMProvider};
pub use tools::{Tool, ToolCall, ToolExecutor, ToolResult};
pub use trajectory::TrajectoryRecorder;
pub use types::*;
