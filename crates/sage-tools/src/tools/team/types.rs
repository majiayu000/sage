//! Type definitions for team collaboration

use serde::{Deserialize, Serialize};

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
