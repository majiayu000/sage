//! Shared API types for HTTP/RPC communication
//!
//! Note: These types are for future API server support.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Chat request from UI clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The user's message
    pub message: String,
    /// Path to the configuration file
    pub config_file: String,
    /// Optional working directory
    pub working_dir: Option<String>,
}

/// Chat response to UI clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Message role (user/assistant)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp of the message
    pub timestamp: String,
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// List of tool calls made
    pub tool_calls: Vec<ToolCallStatus>,
}

/// Status of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStatus {
    /// Unique identifier for the tool call
    pub id: String,
    /// Name of the tool
    pub name: String,
    /// Tool call arguments
    pub args: serde_json::Value,
    /// Current status (pending/running/completed/failed)
    pub status: String,
    /// Start time in milliseconds
    pub start_time: Option<u64>,
    /// End time in milliseconds
    pub end_time: Option<u64>,
    /// Result output if successful
    pub result: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}
