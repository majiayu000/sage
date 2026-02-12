//! Team manager for coordinating multiple agents
//!
//! Provides the core infrastructure for team-based collaboration.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Team configuration stored in ~/.claude/teams/{team-name}/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    /// Team name
    pub name: String,
    /// Team description
    pub description: Option<String>,
    /// Team leader agent ID
    pub lead_agent_id: String,
    /// Team members
    pub members: Vec<TeamMember>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    /// Human-readable name (used for messaging)
    pub name: String,
    /// Unique agent ID (UUID)
    pub agent_id: String,
    /// Agent type/role
    pub agent_type: Option<String>,
    /// Agent capabilities description
    pub capabilities: Option<String>,
    /// Display color for UI
    pub color: Option<String>,
    /// Join timestamp
    pub joined_at: chrono::DateTime<chrono::Utc>,
    /// Whether the agent is currently active
    pub active: bool,
}

/// Join request from an agent wanting to join a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// Unique request ID
    pub request_id: String,
    /// Proposed name for the agent
    pub proposed_name: String,
    /// Agent's capabilities
    pub capabilities: Option<String>,
    /// Requesting agent's temporary ID
    pub requester_id: String,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Message between teammates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    /// Message ID
    pub id: String,
    /// Sender name
    pub from: String,
    /// Recipient name (None for broadcast)
    pub to: Option<String>,
    /// Message content
    pub content: String,
    /// Message type
    pub message_type: MessageType,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Direct message
    Message,
    /// Broadcast to all
    Broadcast,
    /// Protocol request (shutdown, plan approval)
    Request,
    /// Protocol response
    Response,
    /// System notification
    System,
}

/// Team manager for coordinating agents
pub struct TeamManager {
    /// Base directory for team data (~/.claude/teams/)
    teams_dir: PathBuf,
    /// Base directory for task data (~/.claude/tasks/)
    tasks_dir: PathBuf,
    /// Current team (if joined)
    current_team: RwLock<Option<String>>,
    /// Current agent info
    current_agent: RwLock<Option<TeamMember>>,
    /// Message inbox
    inbox: RwLock<Vec<TeamMessage>>,
}

impl Default for TeamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamManager {
    /// Create a new team manager
    pub fn new() -> Self {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let teams_dir = home.join(".claude").join("teams");
        let tasks_dir = home.join(".claude").join("tasks");

        Self {
            teams_dir,
            tasks_dir,
            current_team: RwLock::new(None),
            current_agent: RwLock::new(None),
            inbox: RwLock::new(Vec::new()),
        }
    }

    /// Create a new team
    pub async fn spawn_team(
        &self,
        team_name: &str,
        description: Option<&str>,
        lead_agent_id: &str,
    ) -> Result<TeamConfig, String> {
        let team_dir = self.teams_dir.join(team_name);
        let task_dir = self.tasks_dir.join(team_name);

        // Check if team already exists
        if team_dir.exists() {
            return Err(format!("Team '{}' already exists", team_name));
        }

        // Create directories
        tokio::fs::create_dir_all(&team_dir)
            .await
            .map_err(|e| format!("Failed to create team directory: {}", e))?;
        tokio::fs::create_dir_all(&task_dir)
            .await
            .map_err(|e| format!("Failed to create task directory: {}", e))?;

        let now = chrono::Utc::now();
        let config = TeamConfig {
            name: team_name.to_string(),
            description: description.map(|s| s.to_string()),
            lead_agent_id: lead_agent_id.to_string(),
            members: vec![TeamMember {
                name: "team-lead".to_string(),
                agent_id: lead_agent_id.to_string(),
                agent_type: Some("leader".to_string()),
                capabilities: None,
                color: Some("#4CAF50".to_string()),
                joined_at: now,
                active: true,
            }],
            created_at: now,
            updated_at: now,
        };

        // Save config
        let config_path = team_dir.join("config.json");
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        tokio::fs::write(&config_path, config_json)
            .await
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Set current team
        *self.current_team.write().await = Some(team_name.to_string());
        *self.current_agent.write().await = Some(config.members[0].clone());

        Ok(config)
    }

    /// Discover available teams
    pub async fn discover_teams(&self) -> Result<Vec<TeamConfig>, String> {
        let mut teams = Vec::new();

        if !self.teams_dir.exists() {
            return Ok(teams);
        }

        let mut entries = tokio::fs::read_dir(&self.teams_dir)
            .await
            .map_err(|e| format!("Failed to read teams directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let path = entry.path();
            if path.is_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&config_path).await {
                        if let Ok(config) = serde_json::from_str::<TeamConfig>(&content) {
                            teams.push(config);
                        }
                    }
                }
            }
        }

        Ok(teams)
    }

    /// Request to join a team
    pub async fn request_join(
        &self,
        team_name: &str,
        proposed_name: Option<&str>,
        capabilities: Option<&str>,
    ) -> Result<JoinRequest, String> {
        let team_dir = self.teams_dir.join(team_name);
        if !team_dir.exists() {
            return Err(format!("Team '{}' not found", team_name));
        }

        let request = JoinRequest {
            request_id: format!("join-{}", uuid::Uuid::new_v4()),
            proposed_name: proposed_name
                .unwrap_or(&format!("agent-{}", &uuid::Uuid::new_v4().to_string()[..8]))
                .to_string(),
            capabilities: capabilities.map(|s| s.to_string()),
            requester_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
        };

        // Save request to team's pending requests
        let requests_path = team_dir.join("pending_requests.json");
        let mut requests: Vec<JoinRequest> = if requests_path.exists() {
            let content = tokio::fs::read_to_string(&requests_path)
                .await
                .unwrap_or_else(|_| "[]".to_string());
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        requests.push(request.clone());

        let requests_json = serde_json::to_string_pretty(&requests)
            .map_err(|e| format!("Failed to serialize requests: {}", e))?;
        tokio::fs::write(&requests_path, requests_json)
            .await
            .map_err(|e| format!("Failed to write requests: {}", e))?;

        Ok(request)
    }

    /// Approve a join request (leader only)
    pub async fn approve_join(
        &self,
        team_name: &str,
        request_id: &str,
        target_agent_id: &str,
    ) -> Result<TeamMember, String> {
        let team_dir = self.teams_dir.join(team_name);
        let config_path = team_dir.join("config.json");

        // Load config
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("Failed to read config: {}", e))?;
        let mut config: TeamConfig =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

        // Load and find request
        let requests_path = team_dir.join("pending_requests.json");
        let requests_content = tokio::fs::read_to_string(&requests_path)
            .await
            .unwrap_or_else(|_| "[]".to_string());
        let mut requests: Vec<JoinRequest> =
            serde_json::from_str(&requests_content).unwrap_or_default();

        let request_idx = requests
            .iter()
            .position(|r| r.request_id == request_id)
            .ok_or_else(|| format!("Request '{}' not found", request_id))?;

        let request = requests.remove(request_idx);

        // Create new member
        let member = TeamMember {
            name: target_agent_id.to_string(),
            agent_id: request.requester_id.clone(),
            agent_type: None,
            capabilities: request.capabilities,
            color: Some(self.generate_color(config.members.len())),
            joined_at: chrono::Utc::now(),
            active: true,
        };

        config.members.push(member.clone());
        config.updated_at = chrono::Utc::now();

        // Save updated config
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        tokio::fs::write(&config_path, config_json)
            .await
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Save updated requests
        let requests_json = serde_json::to_string_pretty(&requests)
            .map_err(|e| format!("Failed to serialize requests: {}", e))?;
        tokio::fs::write(&requests_path, requests_json)
            .await
            .map_err(|e| format!("Failed to write requests: {}", e))?;

        Ok(member)
    }

    /// Reject a join request (leader only)
    pub async fn reject_join(
        &self,
        team_name: &str,
        request_id: &str,
        _reason: Option<&str>,
    ) -> Result<(), String> {
        let team_dir = self.teams_dir.join(team_name);
        let requests_path = team_dir.join("pending_requests.json");

        let requests_content = tokio::fs::read_to_string(&requests_path)
            .await
            .unwrap_or_else(|_| "[]".to_string());
        let mut requests: Vec<JoinRequest> =
            serde_json::from_str(&requests_content).unwrap_or_default();

        let request_idx = requests
            .iter()
            .position(|r| r.request_id == request_id)
            .ok_or_else(|| format!("Request '{}' not found", request_id))?;

        requests.remove(request_idx);

        let requests_json = serde_json::to_string_pretty(&requests)
            .map_err(|e| format!("Failed to serialize requests: {}", e))?;
        tokio::fs::write(&requests_path, requests_json)
            .await
            .map_err(|e| format!("Failed to write requests: {}", e))?;

        Ok(())
    }

    /// Clean up team resources
    pub async fn cleanup(&self, team_name: &str) -> Result<(), String> {
        let team_dir = self.teams_dir.join(team_name);
        let task_dir = self.tasks_dir.join(team_name);

        // Check for active members
        let config_path = team_dir.join("config.json");
        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path)
                .await
                .map_err(|e| format!("Failed to read config: {}", e))?;
            let config: TeamConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {}", e))?;

            let active_count = config.members.iter().filter(|m| m.active).count();
            if active_count > 1 {
                return Err(format!(
                    "Cannot cleanup: {} active members remaining. Terminate teammates first.",
                    active_count
                ));
            }
        }

        // Remove directories
        if team_dir.exists() {
            tokio::fs::remove_dir_all(&team_dir)
                .await
                .map_err(|e| format!("Failed to remove team directory: {}", e))?;
        }

        if task_dir.exists() {
            tokio::fs::remove_dir_all(&task_dir)
                .await
                .map_err(|e| format!("Failed to remove task directory: {}", e))?;
        }

        // Clear current team
        *self.current_team.write().await = None;
        *self.current_agent.write().await = None;

        Ok(())
    }

    /// Get team config
    pub async fn get_team(&self, team_name: &str) -> Result<TeamConfig, String> {
        let config_path = self.teams_dir.join(team_name).join("config.json");
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .map_err(|e| format!("Failed to read config: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Send a message
    pub async fn send_message(&self, message: TeamMessage) -> Result<(), String> {
        // In a real implementation, this would use a message bus or file-based queue
        // For now, we just store it in the inbox
        self.inbox.write().await.push(message);
        Ok(())
    }

    /// Get pending messages
    pub async fn get_messages(&self) -> Vec<TeamMessage> {
        self.inbox.read().await.clone()
    }

    /// Generate a color for a new member
    fn generate_color(&self, index: usize) -> String {
        let colors = [
            "#4CAF50", "#2196F3", "#FF9800", "#9C27B0", "#E91E63", "#00BCD4", "#FFEB3B", "#795548",
        ];
        colors[index % colors.len()].to_string()
    }
}

/// Shared team manager instance
pub type SharedTeamManager = Arc<TeamManager>;

/// Create a shared team manager
pub fn create_team_manager() -> SharedTeamManager {
    Arc::new(TeamManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (TeamManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let teams_dir = temp_dir.path().join("teams");
        let tasks_dir = temp_dir.path().join("tasks");

        let manager = TeamManager {
            teams_dir,
            tasks_dir,
            current_team: RwLock::new(None),
            current_agent: RwLock::new(None),
            inbox: RwLock::new(Vec::new()),
        };

        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_spawn_team() {
        let (manager, _temp) = create_test_manager().await;

        let config = manager
            .spawn_team("test-team", Some("Test description"), "leader-123")
            .await
            .unwrap();

        assert_eq!(config.name, "test-team");
        assert_eq!(config.lead_agent_id, "leader-123");
        assert_eq!(config.members.len(), 1);
        assert_eq!(config.members[0].name, "team-lead");
    }

    #[tokio::test]
    async fn test_discover_teams() {
        let (manager, _temp) = create_test_manager().await;

        // Create a team first
        manager
            .spawn_team("team-1", None, "leader-1")
            .await
            .unwrap();
        manager
            .spawn_team("team-2", None, "leader-2")
            .await
            .unwrap();

        let teams = manager.discover_teams().await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let (manager, _temp) = create_test_manager().await;

        manager
            .spawn_team("cleanup-test", None, "leader")
            .await
            .unwrap();

        // Should succeed with only leader
        manager.cleanup("cleanup-test").await.unwrap();

        // Team should be gone
        let teams = manager.discover_teams().await.unwrap();
        assert!(teams.is_empty());
    }
}
