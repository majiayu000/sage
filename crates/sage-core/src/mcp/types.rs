//! MCP type definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Server information returned after initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Protocol version supported
    #[serde(default)]
    pub protocol_version: Option<String>,
}

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCapabilities {
    /// Tool capabilities
    #[serde(default)]
    pub tools: Option<ToolCapabilities>,
    /// Resource capabilities
    #[serde(default)]
    pub resources: Option<ResourceCapabilities>,
    /// Prompt capabilities
    #[serde(default)]
    pub prompts: Option<PromptCapabilities>,
}

/// Tool capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCapabilities {
    /// Whether the server supports tool listing changes
    #[serde(default)]
    pub list_changed: bool,
}

/// Resource capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilities {
    /// Whether resources can be subscribed to
    #[serde(default)]
    pub subscribe: bool,
    /// Whether resource list changes are notified
    #[serde(default)]
    pub list_changed: bool,
}

/// Prompt capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptCapabilities {
    /// Whether prompt list changes are notified
    #[serde(default)]
    pub list_changed: bool,
}

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(default)]
    pub description: Option<String>,
    /// Input schema (JSON Schema)
    #[serde(default)]
    pub input_schema: Value,
}

impl McpTool {
    /// Create a new MCP tool
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema: Value::Object(serde_json::Map::new()),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set input schema
    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }
}

/// MCP tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    /// Result content
    pub content: Vec<McpContent>,
    /// Whether the execution produced an error
    #[serde(default)]
    pub is_error: bool,
}

/// Content types in MCP responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum McpContent {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Image content
    #[serde(rename = "image")]
    Image {
        data: String,
        mime_type: String,
    },
    /// Resource reference
    #[serde(rename = "resource")]
    Resource {
        resource: McpResourceRef,
    },
}

impl McpContent {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create image content
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }
}

/// Reference to a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceRef {
    /// Resource URI
    pub uri: String,
    /// Resource text content
    #[serde(default)]
    pub text: Option<String>,
    /// Resource blob content (base64)
    #[serde(default)]
    pub blob: Option<String>,
}

/// MCP resource definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(default)]
    pub description: Option<String>,
    /// MIME type
    #[serde(default)]
    pub mime_type: Option<String>,
}

impl McpResource {
    /// Create a new resource
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResourceContent {
    /// Resource URI
    pub uri: String,
    /// MIME type
    #[serde(default)]
    pub mime_type: Option<String>,
    /// Text content
    #[serde(default)]
    pub text: Option<String>,
    /// Blob content (base64 encoded)
    #[serde(default)]
    pub blob: Option<String>,
}

/// MCP prompt definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPrompt {
    /// Prompt name
    pub name: String,
    /// Prompt description
    #[serde(default)]
    pub description: Option<String>,
    /// Prompt arguments
    #[serde(default)]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

/// Prompt argument definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether the argument is required
    #[serde(default)]
    pub required: bool,
}

/// Prompt message returned by get_prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpPromptMessage {
    /// Message role
    pub role: PromptRole,
    /// Message content
    pub content: McpContent,
}

/// Prompt message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client info
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Roots capability (for workspace roots)
    #[serde(default)]
    pub roots: Option<HashMap<String, Value>>,
    /// Sampling capability
    #[serde(default)]
    pub sampling: Option<HashMap<String, Value>>,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "sage-agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: McpCapabilities,
    /// Server info
    pub server_info: McpServerInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_serialization() {
        let tool = McpTool::new("read_file")
            .with_description("Read a file")
            .with_input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }));

        let json = serde_json::to_string(&tool).unwrap();
        let parsed: McpTool = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "read_file");
        assert_eq!(parsed.description, Some("Read a file".to_string()));
    }

    #[test]
    fn test_mcp_content_text() {
        let content = McpContent::text("Hello, world!");

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("Hello, world!"));
    }

    #[test]
    fn test_mcp_resource() {
        let resource = McpResource::new("file:///tmp/test.txt", "test.txt");

        let json = serde_json::to_string(&resource).unwrap();
        let parsed: McpResource = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.uri, "file:///tmp/test.txt");
        assert_eq!(parsed.name, "test.txt");
    }
}
