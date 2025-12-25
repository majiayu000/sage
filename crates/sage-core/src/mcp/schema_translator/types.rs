//! Type definitions and imports for schema translation

use crate::mcp::types::{McpContent, McpTool, McpToolResult};
use crate::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Re-export commonly used types for convenience
pub use crate::mcp::types::McpContent as Content;
pub use crate::tools::types::ToolCall as Call;
pub use crate::tools::types::ToolResult as Result;
pub use crate::tools::types::ToolSchema as Schema;
