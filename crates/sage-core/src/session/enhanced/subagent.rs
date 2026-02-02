//! Subagent data types for tracking child agent execution

use super::message::EnhancedTokenUsage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Subagent execution data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentData {
    /// Unique subagent ID
    pub id: String,

    /// Parent agent ID
    #[serde(rename = "parentAgentId")]
    pub parent_agent_id: String,

    /// Agent type (explore, plan, bash, etc.)
    #[serde(rename = "agentType")]
    pub agent_type: String,

    /// Task description
    pub task: String,

    /// Model used by this subagent
    pub model: String,

    /// Maximum turns allowed
    #[serde(rename = "maxTurns")]
    pub max_turns: u32,

    /// Start timestamp
    #[serde(rename = "startTime")]
    pub start_time: DateTime<Utc>,

    /// End timestamp (filled when subagent completes)
    #[serde(rename = "endTime")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<DateTime<Utc>>,

    /// Whether execution succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,

    /// Result summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,

    /// Number of turns used
    #[serde(rename = "turnsUsed")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turns_used: Option<u32>,

    /// Token usage for this subagent
    #[serde(rename = "tokenUsage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<EnhancedTokenUsage>,

    /// Total duration in milliseconds
    #[serde(rename = "durationMs")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl SubagentData {
    /// Create a new subagent data entry (for SubagentStart)
    pub fn new(
        id: impl Into<String>,
        parent_agent_id: impl Into<String>,
        agent_type: impl Into<String>,
        task: impl Into<String>,
        model: impl Into<String>,
        max_turns: u32,
    ) -> Self {
        Self {
            id: id.into(),
            parent_agent_id: parent_agent_id.into(),
            agent_type: agent_type.into(),
            task: task.into(),
            model: model.into(),
            max_turns,
            start_time: Utc::now(),
            end_time: None,
            success: None,
            result: None,
            turns_used: None,
            token_usage: None,
            duration_ms: None,
        }
    }

    /// Mark subagent as completed successfully
    pub fn complete_success(mut self, result: impl Into<String>, turns_used: u32) -> Self {
        self.end_time = Some(Utc::now());
        self.success = Some(true);
        self.result = Some(result.into());
        self.turns_used = Some(turns_used);
        self.duration_ms = Some(
            self.end_time
                .unwrap()
                .signed_duration_since(self.start_time)
                .num_milliseconds() as u64,
        );
        self
    }

    /// Mark subagent as failed
    pub fn complete_failure(mut self, error: impl Into<String>, turns_used: u32) -> Self {
        self.end_time = Some(Utc::now());
        self.success = Some(false);
        self.result = Some(error.into());
        self.turns_used = Some(turns_used);
        self.duration_ms = Some(
            self.end_time
                .unwrap()
                .signed_duration_since(self.start_time)
                .num_milliseconds() as u64,
        );
        self
    }

    /// Set token usage
    pub fn with_token_usage(mut self, usage: EnhancedTokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    /// Check if subagent has completed
    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_data_creation() {
        let subagent = SubagentData::new(
            "sub-001",
            "parent-001",
            "explore",
            "Find all test files",
            "claude-3-haiku",
            10,
        );

        assert_eq!(subagent.id, "sub-001");
        assert_eq!(subagent.parent_agent_id, "parent-001");
        assert_eq!(subagent.agent_type, "explore");
        assert_eq!(subagent.max_turns, 10);
        assert!(!subagent.is_completed());
    }

    #[test]
    fn test_subagent_completion() {
        let subagent = SubagentData::new(
            "sub-002",
            "parent-001",
            "bash",
            "Run tests",
            "claude-3-sonnet",
            5,
        )
        .complete_success("All tests passed", 3);

        assert!(subagent.is_completed());
        assert_eq!(subagent.success, Some(true));
        assert_eq!(subagent.turns_used, Some(3));
        assert!(subagent.duration_ms.is_some());
    }

    #[test]
    fn test_subagent_failure() {
        let subagent = SubagentData::new(
            "sub-003",
            "parent-001",
            "plan",
            "Design architecture",
            "claude-3-opus",
            20,
        )
        .complete_failure("Max turns exceeded", 20);

        assert!(subagent.is_completed());
        assert_eq!(subagent.success, Some(false));
        assert_eq!(subagent.turns_used, Some(20));
    }

    #[test]
    fn test_serialization() {
        let subagent = SubagentData::new("sub-001", "parent-001", "explore", "Test", "model", 5);
        let json = serde_json::to_string(&subagent).unwrap();
        assert!(json.contains("parentAgentId"));
        assert!(json.contains("agentType"));
        assert!(json.contains("maxTurns"));

        let deserialized: SubagentData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "sub-001");
    }
}
