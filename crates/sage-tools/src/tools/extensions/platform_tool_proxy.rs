//! Platform Tool Proxy - Handles external platform tools (e.g., GLM built-in tools)
//!
//! This tool acts as a proxy/fallback for tools that are provided by the LLM platform
//! but not registered in Sage's tool registry. It allows the agent to gracefully handle
//! platform-specific tools like GLM's `claim_glm_camp_coupon`.

use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolResult, ToolSchema};
use serde_json::json;

/// Proxy tool that handles platform-provided tools
pub struct PlatformToolProxy {
    tool_name: String,
    description: String,
}

impl PlatformToolProxy {
    /// Create a new platform tool proxy with dynamic name
    pub fn new(tool_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            description: description.into(),
        }
    }

    /// Create a proxy for GLM's claim_glm_camp_coupon tool
    pub fn glm_claim_coupon() -> Self {
        Self::new(
            "claim_glm_camp_coupon",
            "GLM platform built-in tool for claiming promotional coupons. This tool is provided by the GLM platform and will be executed by the platform itself.",
        )
    }
}

#[async_trait]
impl Tool for PlatformToolProxy {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        // Create a flexible schema that accepts any parameters
        ToolSchema::new_flexible(
            self.name(),
            self.description(),
            json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true,
                "description": "Accepts any parameters from the platform"
            }),
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // Log the platform tool call
        tracing::info!(
            tool_name = %call.name,
            args = ?call.arguments,
            "Platform tool proxy handling external tool call"
        );

        // Format arguments for display
        let args_display = if call.arguments.is_empty() {
            "no arguments".to_string()
        } else {
            serde_json::to_string_pretty(&call.arguments)
                .unwrap_or_else(|_| format!("{:?}", call.arguments))
        };

        // Return a transparent message explaining platform tool behavior
        // The platform will execute this tool and return results in its next response
        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!(
                "Platform tool '{}' has been invoked with the following parameters:\n\n{}\n\n⚠️  Note: This is a platform-provided tool (e.g., GLM built-in tool).\nThe execution result will be provided by the platform in its next response.\nPlease wait for the platform to complete the operation.",
                call.name,
                args_display
            ),
        ))
    }

    fn validate(&self, _call: &ToolCall) -> Result<(), ToolError> {
        // Platform tools can accept any arguments, so validation always passes
        Ok(())
    }

    fn max_execution_duration(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(5)) // 5 seconds - this is just an acknowledgment
    }

    fn supports_parallel_execution(&self) -> bool {
        true // Platform tool proxies don't interfere with each other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(id: &str, name: &str, args: serde_json::Value) -> ToolCall {
        let arguments = if let serde_json::Value::Object(map) = args {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        ToolCall {
            id: id.to_string(),
            name: name.to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_glm_claim_coupon_proxy() {
        let tool = PlatformToolProxy::glm_claim_coupon();
        assert_eq!(tool.name(), "claim_glm_camp_coupon");
        assert!(tool.description().contains("GLM platform"));
    }

    #[tokio::test]
    async fn test_platform_tool_execution() {
        let tool = PlatformToolProxy::glm_claim_coupon();
        let call = create_tool_call(
            "test-1",
            "claim_glm_camp_coupon",
            json!({
                "campaign_id": "2024_winter_promo"
            }),
        );

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("Platform tool"));
        assert!(output.contains("invoked"));
    }

    #[tokio::test]
    async fn test_platform_tool_no_args() {
        let tool = PlatformToolProxy::new("test_platform_tool", "Test tool");
        let call = create_tool_call("test-2", "test_platform_tool", json!({}));

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        let output = result.output.as_ref().unwrap();
        assert!(output.contains("no arguments"));
    }

    #[tokio::test]
    async fn test_custom_platform_tool() {
        let tool = PlatformToolProxy::new(
            "custom_tool",
            "Custom platform tool for testing",
        );
        assert_eq!(tool.name(), "custom_tool");
        assert!(tool.description().contains("Custom platform tool"));
    }

    #[test]
    fn test_platform_tool_schema() {
        let tool = PlatformToolProxy::glm_claim_coupon();
        let schema = tool.schema();
        assert_eq!(schema.name, "claim_glm_camp_coupon");
        assert!(!schema.description.is_empty());
    }

    #[tokio::test]
    async fn test_validation_always_passes() {
        let tool = PlatformToolProxy::glm_claim_coupon();
        let call = create_tool_call(
            "test-3",
            "claim_glm_camp_coupon",
            json!({
                "any_param": "any_value",
                "number": 42,
                "nested": {
                    "key": "value"
                }
            }),
        );

        // Validation should always succeed for platform tools
        assert!(tool.validate(&call).is_ok());
    }
}
