//! SendMessageTool for sending messages between teammates
//!
//! This tool provides messaging capabilities in a swarm:
//! - message: Send a direct message to a specific teammate
//! - broadcast: Send a message to all teammates (use sparingly)
//! - request: Send a protocol request (shutdown, plan approval)
//! - response: Respond to a protocol request

use super::team_manager::{MessageType, SharedTeamManager, TeamManager, TeamMessage};
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::sync::Arc;

/// Tool for sending messages to teammates
pub struct SendMessageTool {
    manager: SharedTeamManager,
}

impl Default for SendMessageTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SendMessageTool {
    /// Create a new SendMessageTool
    pub fn new() -> Self {
        Self {
            manager: Arc::new(TeamManager::new()),
        }
    }

    /// Create with an existing team manager
    pub fn with_manager(manager: SharedTeamManager) -> Self {
        Self { manager }
    }

    /// Get current agent name from environment
    fn get_current_agent(&self) -> Result<String, ToolError> {
        std::env::var("CLAUDE_CODE_AGENT_ID")
            .map_err(|_| ToolError::ExecutionFailed("Not in a team context".to_string()))
    }

    /// Send a direct message
    async fn send_direct_message(&self, call: &ToolCall) -> Result<String, ToolError> {
        let recipient = call.get_string("recipient").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: recipient".to_string())
        })?;

        let content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: content".to_string())
        })?;

        let from = self.get_current_agent()?;

        let message = TeamMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.clone(),
            to: Some(recipient.clone()),
            content,
            message_type: MessageType::Message,
            timestamp: chrono::Utc::now(),
        };

        self.manager
            .send_message(message)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!("Message sent to '{}'.", recipient))
    }

    /// Broadcast a message to all teammates
    async fn broadcast(&self, call: &ToolCall) -> Result<String, ToolError> {
        let content = call.get_string("content").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: content".to_string())
        })?;

        let from = self.get_current_agent()?;

        let message = TeamMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.clone(),
            to: None, // None means broadcast
            content,
            message_type: MessageType::Broadcast,
            timestamp: chrono::Utc::now(),
        };

        self.manager
            .send_message(message)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok("Message broadcast to all teammates.\n\n\
            WARNING: Broadcasting is expensive. Each broadcast sends a separate message \
            to every teammate. Use direct messages when possible."
            .to_string())
    }

    /// Send a protocol request
    async fn send_request(&self, call: &ToolCall) -> Result<String, ToolError> {
        let subtype = call.get_string("subtype").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: subtype".to_string())
        })?;

        let recipient = call.get_string("recipient").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: recipient".to_string())
        })?;

        let content = call.get_string("content").unwrap_or_default();
        let from = self.get_current_agent()?;

        let request_id = uuid::Uuid::new_v4().to_string();

        // Create the request message with embedded metadata
        let request_content = serde_json::json!({
            "type": format!("{}_request", subtype),
            "requestId": request_id,
            "from": from,
            "content": content,
        });

        let message = TeamMessage {
            id: request_id.clone(),
            from: from.clone(),
            to: Some(recipient.clone()),
            content: serde_json::to_string(&request_content).unwrap_or(content),
            message_type: MessageType::Request,
            timestamp: chrono::Utc::now(),
        };

        self.manager
            .send_message(message)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "{} request sent to '{}'.\n\
             Request ID: {}\n\n\
             Waiting for response...",
            subtype, recipient, request_id
        ))
    }

    /// Send a protocol response
    async fn send_response(&self, call: &ToolCall) -> Result<String, ToolError> {
        let subtype = call.get_string("subtype").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: subtype".to_string())
        })?;

        let request_id = call.get_string("request_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: request_id".to_string())
        })?;

        let approve = call
            .get_bool("approve")
            .ok_or_else(|| {
                ToolError::InvalidArguments("Missing required parameter: approve".to_string())
            })?;

        let content = call.get_string("content");
        let recipient = call.get_string("recipient");
        let from = self.get_current_agent()?;

        // Create the response message
        let response_content = serde_json::json!({
            "type": format!("{}_response", subtype),
            "requestId": request_id,
            "approved": approve,
            "content": content,
        });

        let message = TeamMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.clone(),
            to: recipient,
            content: serde_json::to_string(&response_content).unwrap_or_default(),
            message_type: MessageType::Response,
            timestamp: chrono::Utc::now(),
        };

        self.manager
            .send_message(message)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        let status = if approve { "approved" } else { "rejected" };

        // Handle shutdown approval specially
        if subtype == "shutdown" && approve {
            Ok(format!(
                "Shutdown {} and confirmed. Process will terminate.",
                status
            ))
        } else {
            Ok(format!(
                "{} request {} (request_id: {}).",
                subtype, status, request_id
            ))
        }
    }
}

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "SendMessageTool"
    }

    fn description(&self) -> &str {
        r#"Send messages to teammates and handle protocol requests/responses in a swarm.

Message Types:
- message: Send a direct message to a specific teammate (requires: recipient, content)
- broadcast: Send to ALL teammates - USE SPARINGLY (requires: content)
- request: Send a protocol request like shutdown (requires: subtype, recipient, optional: content)
- response: Respond to a protocol request (requires: subtype, request_id, approve, optional: content, recipient)

IMPORTANT: Your plain text output is NOT visible to teammates. You MUST use this tool to communicate."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "type",
                    "Message type: message, broadcast, request, response",
                ),
                ToolParameter::string("recipient", "Recipient teammate name (for message, request)")
                    .optional(),
                ToolParameter::string("content", "Message content").optional(),
                ToolParameter::string(
                    "subtype",
                    "Request/response subtype: shutdown, plan_approval (for request, response)",
                )
                .optional(),
                ToolParameter::string("request_id", "Request ID to respond to (for response)")
                    .optional(),
                ToolParameter::boolean("approve", "Whether to approve the request (for response)")
                    .optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let msg_type = call.get_string("type").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: type".to_string())
        })?;

        let result = match msg_type.as_str() {
            "message" => self.send_direct_message(call).await?,
            "broadcast" => self.broadcast(call).await?,
            "request" => self.send_request(call).await?,
            "response" => self.send_response(call).await?,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown message type: {}. Valid types: message, broadcast, request, response",
                    msg_type
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let msg_type = call.get_string("type").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: type".to_string())
        })?;

        match msg_type.as_str() {
            "message" => {
                call.get_string("recipient").ok_or_else(|| {
                    ToolError::InvalidArguments("message requires recipient parameter".to_string())
                })?;
                call.get_string("content").ok_or_else(|| {
                    ToolError::InvalidArguments("message requires content parameter".to_string())
                })?;
            }
            "broadcast" => {
                call.get_string("content").ok_or_else(|| {
                    ToolError::InvalidArguments("broadcast requires content parameter".to_string())
                })?;
            }
            "request" => {
                call.get_string("subtype").ok_or_else(|| {
                    ToolError::InvalidArguments("request requires subtype parameter".to_string())
                })?;
                call.get_string("recipient").ok_or_else(|| {
                    ToolError::InvalidArguments("request requires recipient parameter".to_string())
                })?;
            }
            "response" => {
                call.get_string("subtype").ok_or_else(|| {
                    ToolError::InvalidArguments("response requires subtype parameter".to_string())
                })?;
                call.get_string("request_id").ok_or_else(|| {
                    ToolError::InvalidArguments("response requires request_id parameter".to_string())
                })?;
                call.get_bool("approve").ok_or_else(|| {
                    ToolError::InvalidArguments("response requires approve parameter".to_string())
                })?;
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown message type: {}",
                    msg_type
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn create_tool_call(msg_type: &str, args: Vec<(&str, serde_json::Value)>) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("type".to_string(), json!(msg_type));
        for (key, value) in args {
            arguments.insert(key.to_string(), value);
        }

        ToolCall {
            id: "test-1".to_string(),
            name: "SendMessageTool".to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_invalid_type() {
        let tool = SendMessageTool::new();
        let call = create_tool_call("invalid", vec![]);

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_missing_recipient() {
        let tool = SendMessageTool::new();
        let call = create_tool_call("message", vec![("content", json!("Hello"))]);

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_broadcast_missing_content() {
        let tool = SendMessageTool::new();
        let call = create_tool_call("broadcast", vec![]);

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_response_validation() {
        let tool = SendMessageTool::new();

        // Missing subtype
        let call = create_tool_call(
            "response",
            vec![("request_id", json!("123")), ("approve", json!(true))],
        );
        assert!(tool.validate(&call).is_err());

        // Missing request_id
        let call = create_tool_call(
            "response",
            vec![("subtype", json!("shutdown")), ("approve", json!(true))],
        );
        assert!(tool.validate(&call).is_err());

        // Missing approve
        let call = create_tool_call(
            "response",
            vec![
                ("subtype", json!("shutdown")),
                ("request_id", json!("123")),
            ],
        );
        assert!(tool.validate(&call).is_err());
    }

    #[test]
    fn test_schema() {
        let tool = SendMessageTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "SendMessageTool");
        assert!(!schema.description.is_empty());
    }
}
