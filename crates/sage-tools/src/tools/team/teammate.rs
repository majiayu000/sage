//! TeammateTool for managing teams and coordinating teammates
//!
//! This tool provides operations for team management in a swarm:
//! - spawnTeam: Create a new team
//! - discoverTeams: Find available teams
//! - requestJoin: Request to join a team
//! - approveJoin: Approve a join request (leader only)
//! - rejectJoin: Reject a join request (leader only)
//! - cleanup: Clean up team resources

use super::team_manager::{SharedTeamManager, TeamManager};
use async_trait::async_trait;
use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};
use std::sync::Arc;

/// Tool for managing teams and coordinating teammates
pub struct TeammateTool {
    manager: SharedTeamManager,
}

impl Default for TeammateTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TeammateTool {
    /// Create a new TeammateTool
    pub fn new() -> Self {
        Self {
            manager: Arc::new(TeamManager::new()),
        }
    }

    /// Create with an existing team manager
    pub fn with_manager(manager: SharedTeamManager) -> Self {
        Self { manager }
    }

    /// Get the team manager
    pub fn manager(&self) -> SharedTeamManager {
        Arc::clone(&self.manager)
    }

    /// Execute spawn team operation
    async fn spawn_team(&self, call: &ToolCall) -> Result<String, ToolError> {
        let team_name = call.get_string("team_name").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: team_name".to_string())
        })?;

        let description = call.get_string("description");

        // Get agent ID from environment or generate one
        let lead_agent_id = std::env::var("CLAUDE_CODE_AGENT_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        let config = self
            .manager
            .spawn_team(&team_name, description.as_deref(), &lead_agent_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "Team '{}' created successfully.\n\n\
             - Team directory: ~/.claude/teams/{}/\n\
             - Task directory: ~/.claude/tasks/{}/\n\
             - Leader: {} ({})\n\n\
             You can now spawn teammates using the Task tool with `team_name` and `name` parameters.",
            config.name, config.name, config.name, config.members[0].name, config.lead_agent_id
        ))
    }

    /// Execute discover teams operation
    async fn discover_teams(&self) -> Result<String, ToolError> {
        let teams = self
            .manager
            .discover_teams()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        if teams.is_empty() {
            return Ok("No teams found. Use `spawnTeam` to create a new team.".to_string());
        }

        let mut output = format!("Found {} team(s):\n\n", teams.len());

        for team in teams {
            output.push_str(&format!(
                "- **{}**\n  Description: {}\n  Leader: {}\n  Members: {}\n\n",
                team.name,
                team.description.as_deref().unwrap_or("(none)"),
                team.lead_agent_id,
                team.members.len()
            ));
        }

        Ok(output)
    }

    /// Execute request join operation
    async fn request_join(&self, call: &ToolCall) -> Result<String, ToolError> {
        let team_name = call.get_string("team_name").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: team_name".to_string())
        })?;

        let proposed_name = call.get_string("proposed_name");
        let capabilities = call.get_string("capabilities");

        let request = self
            .manager
            .request_join(
                &team_name,
                proposed_name.as_deref(),
                capabilities.as_deref(),
            )
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "Join request sent to team '{}'.\n\n\
             - Request ID: {}\n\
             - Proposed name: {}\n\n\
             Waiting for team leader to approve or reject the request.",
            team_name, request.request_id, request.proposed_name
        ))
    }

    /// Execute approve join operation
    async fn approve_join(&self, call: &ToolCall) -> Result<String, ToolError> {
        let target_agent_id = call.get_string("target_agent_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: target_agent_id".to_string())
        })?;

        let request_id = call.get_string("request_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: request_id".to_string())
        })?;

        // Get current team from environment
        let team_name = std::env::var("CLAUDE_CODE_TEAM_NAME")
            .map_err(|_| ToolError::ExecutionFailed("Not in a team context".to_string()))?;

        let member = self
            .manager
            .approve_join(&team_name, &request_id, &target_agent_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "Join request approved.\n\n\
             New team member:\n\
             - Name: {}\n\
             - Agent ID: {}\n\
             - Color: {}\n\n\
             The agent has been notified and can now participate in the team.",
            member.name,
            member.agent_id,
            member.color.as_deref().unwrap_or("default")
        ))
    }

    /// Execute reject join operation
    async fn reject_join(&self, call: &ToolCall) -> Result<String, ToolError> {
        let target_agent_id = call.get_string("target_agent_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: target_agent_id".to_string())
        })?;

        let request_id = call.get_string("request_id").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: request_id".to_string())
        })?;

        let reason = call.get_string("reason");

        // Get current team from environment
        let team_name = std::env::var("CLAUDE_CODE_TEAM_NAME")
            .map_err(|_| ToolError::ExecutionFailed("Not in a team context".to_string()))?;

        self.manager
            .reject_join(&team_name, &request_id, reason.as_deref())
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "Join request from '{}' rejected.\n\
             Reason: {}",
            target_agent_id,
            reason.as_deref().unwrap_or("No reason provided")
        ))
    }

    /// Execute cleanup operation
    async fn cleanup(&self) -> Result<String, ToolError> {
        // Get current team from environment
        let team_name = std::env::var("CLAUDE_CODE_TEAM_NAME")
            .map_err(|_| ToolError::ExecutionFailed("Not in a team context".to_string()))?;

        self.manager
            .cleanup(&team_name)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e))?;

        Ok(format!(
            "Team '{}' cleaned up successfully.\n\n\
             - Team directory removed\n\
             - Task directory removed\n\
             - Team context cleared",
            team_name
        ))
    }
}

#[async_trait]
impl Tool for TeammateTool {
    fn name(&self) -> &str {
        "TeammateTool"
    }

    fn description(&self) -> &str {
        r#"Manage teams and coordinate teammates in a swarm.

Operations:
- spawnTeam: Create a new team (requires: team_name, optional: description)
- discoverTeams: List available teams to join
- requestJoin: Request to join a team (requires: team_name, optional: proposed_name, capabilities)
- approveJoin: Approve a join request (requires: target_agent_id, request_id) [Leader only]
- rejectJoin: Reject a join request (requires: target_agent_id, request_id, optional: reason) [Leader only]
- cleanup: Clean up team resources [Leader only]

Note: To spawn new teammates, use the Task tool with `team_name` and `name` parameters."#
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string(
                    "operation",
                    "The operation to perform: spawnTeam, discoverTeams, requestJoin, approveJoin, rejectJoin, cleanup",
                ),
                ToolParameter::string("team_name", "Name of the team (for spawnTeam, requestJoin)")
                    .optional(),
                ToolParameter::string("description", "Team description (for spawnTeam)").optional(),
                ToolParameter::string(
                    "proposed_name",
                    "Proposed name for joining agent (for requestJoin)",
                )
                .optional(),
                ToolParameter::string(
                    "capabilities",
                    "Description of agent capabilities (for requestJoin)",
                )
                .optional(),
                ToolParameter::string(
                    "target_agent_id",
                    "Target agent name from join request (for approveJoin, rejectJoin)",
                )
                .optional(),
                ToolParameter::string(
                    "request_id",
                    "Join request ID (for approveJoin, rejectJoin)",
                )
                .optional(),
                ToolParameter::string("reason", "Rejection reason (for rejectJoin)").optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let operation = call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        let result = match operation.as_str() {
            "spawnTeam" => self.spawn_team(call).await?,
            "discoverTeams" => self.discover_teams().await?,
            "requestJoin" => self.request_join(call).await?,
            "approveJoin" => self.approve_join(call).await?,
            "rejectJoin" => self.reject_join(call).await?,
            "cleanup" => self.cleanup().await?,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown operation: {}. Valid operations: spawnTeam, discoverTeams, requestJoin, approveJoin, rejectJoin, cleanup",
                    operation
                )));
            }
        };

        Ok(ToolResult::success(&call.id, self.name(), result))
    }

    fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        let operation = call.get_string("operation").ok_or_else(|| {
            ToolError::InvalidArguments("Missing required parameter: operation".to_string())
        })?;

        match operation.as_str() {
            "spawnTeam" => {
                call.get_string("team_name").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "spawnTeam requires team_name parameter".to_string(),
                    )
                })?;
            }
            "requestJoin" => {
                call.get_string("team_name").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "requestJoin requires team_name parameter".to_string(),
                    )
                })?;
            }
            "approveJoin" | "rejectJoin" => {
                call.get_string("target_agent_id").ok_or_else(|| {
                    ToolError::InvalidArguments(format!(
                        "{} requires target_agent_id parameter",
                        operation
                    ))
                })?;
                call.get_string("request_id").ok_or_else(|| {
                    ToolError::InvalidArguments(format!(
                        "{} requires request_id parameter",
                        operation
                    ))
                })?;
            }
            "discoverTeams" | "cleanup" => {}
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown operation: {}",
                    operation
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

    fn create_tool_call(operation: &str, args: Vec<(&str, &str)>) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert("operation".to_string(), json!(operation));
        for (key, value) in args {
            arguments.insert(key.to_string(), json!(value));
        }

        ToolCall {
            id: "test-1".to_string(),
            name: "TeammateTool".to_string(),
            arguments,
            call_id: None,
        }
    }

    #[tokio::test]
    async fn test_discover_teams_empty() {
        let tool = TeammateTool::new();
        let call = create_tool_call("discoverTeams", vec![]);

        let result = tool.execute(&call).await.unwrap();
        assert!(result.success);
        assert!(result.output.as_ref().unwrap().contains("No teams found"));
    }

    #[tokio::test]
    async fn test_invalid_operation() {
        let tool = TeammateTool::new();
        let call = create_tool_call("invalidOp", vec![]);

        let result = tool.execute(&call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spawn_team_missing_name() {
        let tool = TeammateTool::new();
        let call = create_tool_call("spawnTeam", vec![]);

        let result = tool.validate(&call);
        assert!(result.is_err());
    }

    #[test]
    fn test_schema() {
        let tool = TeammateTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "TeammateTool");
        assert!(!schema.description.is_empty());
    }
}
