//! Running agent management methods

use super::super::types::{AgentProgress, AgentStatus, AgentType, RunningAgent, SubAgentConfig};
use super::types::AgentRegistry;
use crate::error::{SageError, SageResult};
use tokio_util::sync::CancellationToken;

impl AgentRegistry {
    /// Create a new running agent entry and return the agent ID
    pub fn create_running_agent(&self, config: SubAgentConfig) -> String {
        let agent_id = uuid::Uuid::new_v4().to_string();
        let agent_type = config.agent_type;
        let agent = RunningAgent::new(agent_id.clone(), agent_type, config);

        self.running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(agent_id.clone(), agent);

        agent_id
    }

    /// Update agent status
    pub fn update_status(&self, agent_id: &str, status: AgentStatus) {
        if let Some(agent) = self
            .running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(agent_id)
        {
            agent.status = status;
        }
    }

    /// Update agent progress
    pub fn update_progress(&self, agent_id: &str, progress: AgentProgress) {
        if let Some(agent) = self
            .running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(agent_id)
        {
            agent.status = AgentStatus::Running(progress);
        }
    }

    /// Get agent status
    pub fn get_status(&self, agent_id: &str) -> Option<AgentStatus> {
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(agent_id)
            .map(|agent| agent.status.clone())
    }

    /// Get agent progress
    pub fn get_progress(&self, agent_id: &str) -> Option<AgentProgress> {
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(agent_id)
            .and_then(|agent| match &agent.status {
                AgentStatus::Running(progress) => Some(progress.clone()),
                _ => None,
            })
    }

    /// Kill a running agent by cancelling it
    pub fn kill(&self, agent_id: &str) -> SageResult<()> {
        let running = self
            .running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(agent) = running.get(agent_id) {
            agent.cancel_token.cancel();
            Ok(())
        } else {
            Err(SageError::agent(format!(
                "Agent with ID {} not found",
                agent_id
            )))
        }
    }

    /// List all running agents (returns tuples of ID, type, and status)
    pub fn list_running(&self) -> Vec<(String, AgentType, AgentStatus)> {
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(|(id, agent)| (id.clone(), agent.agent_type, agent.status.clone()))
            .collect()
    }

    /// Remove a completed/failed agent from the running agents list
    pub fn remove(&self, agent_id: &str) {
        self.running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(agent_id);
    }

    /// Get the cancellation token for an agent
    pub fn get_cancel_token(&self, agent_id: &str) -> Option<CancellationToken> {
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(agent_id)
            .map(|agent| agent.cancel_token.clone())
    }

    /// Clear all running agents
    pub fn clear_running(&self) {
        self.running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}
