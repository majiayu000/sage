//! Running agent state container

use super::{AgentStatus, AgentType, SubAgentConfig};
use std::fmt;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// Running agent state container
pub struct RunningAgent {
    /// Unique agent identifier
    pub id: String,
    /// Type of agent
    pub agent_type: AgentType,
    /// Configuration used to spawn this agent
    pub config: SubAgentConfig,
    /// Current execution status
    pub status: AgentStatus,
    /// Start time of execution
    pub start_time: Instant,
    /// Cancellation token for stopping the agent
    pub cancel_token: CancellationToken,
}

impl fmt::Debug for RunningAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunningAgent")
            .field("id", &self.id)
            .field("agent_type", &self.agent_type)
            .field("config", &self.config)
            .field("status", &self.status)
            .field("elapsed", &self.start_time.elapsed())
            .finish()
    }
}

impl RunningAgent {
    /// Create a new running agent
    pub fn new(id: String, agent_type: AgentType, config: SubAgentConfig) -> Self {
        Self {
            id,
            agent_type,
            config,
            status: AgentStatus::Pending,
            start_time: Instant::now(),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.start_time.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Check if agent is still active
    pub fn is_active(&self) -> bool {
        !self.status.is_terminal()
    }

    /// Request cancellation of the agent
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// Check if cancellation was requested
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
}
