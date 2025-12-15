//! Concurrency primitives for async code agent operations
//!
//! This module provides hierarchical cancellation and other concurrency utilities
//! for managing async operations across sessions, agents, and tool executions.

use dashmap::DashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Unique identifier for a session
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique identifier for a tool call
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolCallId(pub String);

impl ToolCallId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<String> for ToolCallId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ToolCallId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Hierarchical cancellation management for async operations
///
/// The hierarchy is: Root -> Sessions -> Agents -> Tools
///
/// Cancelling a parent automatically cancels all children:
/// - Cancelling root cancels all sessions, agents, and tools
/// - Cancelling a session cancels all agents and tools within it
/// - Cancelling an agent cancels all tool calls within it
///
/// # Example
///
/// ```rust
/// use sage_core::concurrency::{CancellationHierarchy, SessionId, AgentId, ToolCallId};
///
/// #[tokio::main]
/// async fn main() {
///     let hierarchy = CancellationHierarchy::new();
///
///     // Create a session
///     let session_id = SessionId::new("session-1");
///     let session_token = hierarchy.create_session_token(session_id.clone());
///
///     // Create an agent within the session
///     let agent_id = AgentId::new("agent-1");
///     let agent_token = hierarchy.create_agent_token(&session_id, agent_id.clone()).unwrap();
///
///     // Create a tool call within the agent
///     let tool_id = ToolCallId::new("tool-1");
///     let tool_token = hierarchy.create_tool_token(&agent_id, tool_id.clone()).unwrap();
///
///     // Cancelling the session cancels everything
///     hierarchy.cancel_session(&session_id);
///     assert!(session_token.is_cancelled());
///     assert!(agent_token.is_cancelled());
///     assert!(tool_token.is_cancelled());
/// }
/// ```
#[derive(Debug)]
pub struct CancellationHierarchy {
    /// Root cancellation token (cancels everything)
    root: CancellationToken,
    /// Session-level tokens (keyed by session ID)
    sessions: DashMap<SessionId, SessionState>,
    /// Agent-level tokens (keyed by agent ID, stores parent session ID)
    agents: DashMap<AgentId, AgentState>,
    /// Tool-level tokens (keyed by tool call ID)
    tools: DashMap<ToolCallId, CancellationToken>,
}

#[derive(Debug)]
struct SessionState {
    token: CancellationToken,
}

#[derive(Debug)]
struct AgentState {
    token: CancellationToken,
    session_id: SessionId,
}

impl Default for CancellationHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationHierarchy {
    /// Create a new cancellation hierarchy
    pub fn new() -> Self {
        Self {
            root: CancellationToken::new(),
            sessions: DashMap::new(),
            agents: DashMap::new(),
            tools: DashMap::new(),
        }
    }

    /// Get the root cancellation token
    pub fn root_token(&self) -> CancellationToken {
        self.root.clone()
    }

    /// Check if the root has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.root.is_cancelled()
    }

    /// Cancel everything (root cancellation)
    pub fn cancel_all(&self) {
        self.root.cancel();
    }

    /// Create a cancellation token for a new session
    ///
    /// The session token is a child of the root token, so cancelling the root
    /// will also cancel this session.
    pub fn create_session_token(&self, id: SessionId) -> CancellationToken {
        let token = self.root.child_token();
        self.sessions.insert(
            id,
            SessionState {
                token: token.clone(),
            },
        );
        token
    }

    /// Get an existing session token
    pub fn get_session_token(&self, id: &SessionId) -> Option<CancellationToken> {
        self.sessions.get(id).map(|s| s.token.clone())
    }

    /// Create a cancellation token for a new agent within a session
    ///
    /// The agent token is a child of the session token, so cancelling the session
    /// will also cancel this agent.
    pub fn create_agent_token(
        &self,
        session_id: &SessionId,
        agent_id: AgentId,
    ) -> Option<CancellationToken> {
        let session = self.sessions.get(session_id)?;
        let token = session.token.child_token();
        self.agents.insert(
            agent_id,
            AgentState {
                token: token.clone(),
                session_id: session_id.clone(),
            },
        );
        Some(token)
    }

    /// Get an existing agent token
    pub fn get_agent_token(&self, id: &AgentId) -> Option<CancellationToken> {
        self.agents.get(id).map(|a| a.token.clone())
    }

    /// Create a cancellation token for a tool call within an agent
    ///
    /// The tool token is a child of the agent token, so cancelling the agent
    /// will also cancel this tool call.
    pub fn create_tool_token(
        &self,
        agent_id: &AgentId,
        tool_id: ToolCallId,
    ) -> Option<CancellationToken> {
        let agent = self.agents.get(agent_id)?;
        let token = agent.token.child_token();
        self.tools.insert(tool_id, token.clone());
        Some(token)
    }

    /// Get an existing tool token
    pub fn get_tool_token(&self, id: &ToolCallId) -> Option<CancellationToken> {
        self.tools.get(id).map(|t| t.clone())
    }

    /// Cancel a specific session and all its children
    pub fn cancel_session(&self, id: &SessionId) {
        if let Some(session) = self.sessions.get(id) {
            session.token.cancel();
        }
    }

    /// Cancel a specific agent and all its tool calls
    pub fn cancel_agent(&self, id: &AgentId) {
        if let Some(agent) = self.agents.get(id) {
            agent.token.cancel();
        }
    }

    /// Cancel a specific tool call
    pub fn cancel_tool(&self, id: &ToolCallId) {
        if let Some(tool) = self.tools.get(id) {
            tool.cancel();
        }
    }

    /// Remove a completed session and clean up associated resources
    pub fn remove_session(&self, id: &SessionId) {
        // First, find and remove all agents belonging to this session
        let agents_to_remove: Vec<AgentId> = self
            .agents
            .iter()
            .filter(|entry| &entry.session_id == id)
            .map(|entry| entry.key().clone())
            .collect();

        for agent_id in agents_to_remove {
            self.remove_agent(&agent_id);
        }

        // Remove the session
        self.sessions.remove(id);
    }

    /// Remove a completed agent and clean up associated tool calls
    pub fn remove_agent(&self, id: &AgentId) {
        // Note: We don't track which tools belong to which agent currently
        // In a full implementation, we'd need to add this tracking
        self.agents.remove(id);
    }

    /// Remove a completed tool call
    pub fn remove_tool(&self, id: &ToolCallId) {
        self.tools.remove(id);
    }

    /// Get statistics about the current hierarchy
    pub fn stats(&self) -> HierarchyStats {
        HierarchyStats {
            active_sessions: self.sessions.len(),
            active_agents: self.agents.len(),
            active_tools: self.tools.len(),
            is_cancelled: self.root.is_cancelled(),
        }
    }
}

/// Statistics about the cancellation hierarchy
#[derive(Debug, Clone)]
pub struct HierarchyStats {
    pub active_sessions: usize,
    pub active_agents: usize,
    pub active_tools: usize,
    pub is_cancelled: bool,
}

/// Thread-safe wrapper around CancellationHierarchy
pub type SharedCancellationHierarchy = Arc<CancellationHierarchy>;

/// Create a new shared cancellation hierarchy
pub fn shared_hierarchy() -> SharedCancellationHierarchy {
    Arc::new(CancellationHierarchy::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hierarchy_creation() {
        let hierarchy = CancellationHierarchy::new();
        assert!(!hierarchy.is_cancelled());

        let stats = hierarchy.stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.active_agents, 0);
        assert_eq!(stats.active_tools, 0);
    }

    #[tokio::test]
    async fn test_session_creation() {
        let hierarchy = CancellationHierarchy::new();

        let session_id = SessionId::new("test-session");
        let token = hierarchy.create_session_token(session_id.clone());

        assert!(!token.is_cancelled());
        assert_eq!(hierarchy.stats().active_sessions, 1);

        // Verify we can get the token back
        let retrieved = hierarchy.get_session_token(&session_id);
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_agent_creation() {
        let hierarchy = CancellationHierarchy::new();

        let session_id = SessionId::new("test-session");
        hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        let token = hierarchy
            .create_agent_token(&session_id, agent_id.clone())
            .unwrap();

        assert!(!token.is_cancelled());
        assert_eq!(hierarchy.stats().active_agents, 1);
    }

    #[tokio::test]
    async fn test_tool_creation() {
        let hierarchy = CancellationHierarchy::new();

        let session_id = SessionId::new("test-session");
        hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        hierarchy.create_agent_token(&session_id, agent_id.clone());

        let tool_id = ToolCallId::new("test-tool");
        let token = hierarchy
            .create_tool_token(&agent_id, tool_id.clone())
            .unwrap();

        assert!(!token.is_cancelled());
        assert_eq!(hierarchy.stats().active_tools, 1);
    }

    #[tokio::test]
    async fn test_session_cancellation_propagates() {
        let hierarchy = CancellationHierarchy::new();

        // Create hierarchy
        let session_id = SessionId::new("test-session");
        let session_token = hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        let agent_token = hierarchy
            .create_agent_token(&session_id, agent_id.clone())
            .unwrap();

        let tool_id = ToolCallId::new("test-tool");
        let tool_token = hierarchy
            .create_tool_token(&agent_id, tool_id.clone())
            .unwrap();

        // Cancel the session
        hierarchy.cancel_session(&session_id);

        // All should be cancelled
        assert!(session_token.is_cancelled());
        assert!(agent_token.is_cancelled());
        assert!(tool_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_agent_cancellation_propagates() {
        let hierarchy = CancellationHierarchy::new();

        // Create hierarchy
        let session_id = SessionId::new("test-session");
        let session_token = hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        let agent_token = hierarchy
            .create_agent_token(&session_id, agent_id.clone())
            .unwrap();

        let tool_id = ToolCallId::new("test-tool");
        let tool_token = hierarchy
            .create_tool_token(&agent_id, tool_id.clone())
            .unwrap();

        // Cancel just the agent
        hierarchy.cancel_agent(&agent_id);

        // Session should NOT be cancelled
        assert!(!session_token.is_cancelled());
        // Agent and tool should be cancelled
        assert!(agent_token.is_cancelled());
        assert!(tool_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_root_cancellation() {
        let hierarchy = CancellationHierarchy::new();

        let session_id = SessionId::new("test-session");
        let session_token = hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        let agent_token = hierarchy
            .create_agent_token(&session_id, agent_id.clone())
            .unwrap();

        // Cancel root
        hierarchy.cancel_all();

        assert!(hierarchy.is_cancelled());
        assert!(session_token.is_cancelled());
        assert!(agent_token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cleanup() {
        let hierarchy = CancellationHierarchy::new();

        let session_id = SessionId::new("test-session");
        hierarchy.create_session_token(session_id.clone());

        let agent_id = AgentId::new("test-agent");
        hierarchy.create_agent_token(&session_id, agent_id.clone());

        let tool_id = ToolCallId::new("test-tool");
        hierarchy.create_tool_token(&agent_id, tool_id.clone());

        assert_eq!(hierarchy.stats().active_sessions, 1);
        assert_eq!(hierarchy.stats().active_agents, 1);
        assert_eq!(hierarchy.stats().active_tools, 1);

        // Clean up
        hierarchy.remove_tool(&tool_id);
        assert_eq!(hierarchy.stats().active_tools, 0);

        hierarchy.remove_session(&session_id);
        assert_eq!(hierarchy.stats().active_sessions, 0);
        assert_eq!(hierarchy.stats().active_agents, 0);
    }

    #[tokio::test]
    async fn test_invalid_parent() {
        let hierarchy = CancellationHierarchy::new();

        // Try to create agent without session
        let agent_id = AgentId::new("orphan-agent");
        let result = hierarchy.create_agent_token(&SessionId::new("nonexistent"), agent_id);
        assert!(result.is_none());

        // Try to create tool without agent
        let tool_id = ToolCallId::new("orphan-tool");
        let result = hierarchy.create_tool_token(&AgentId::new("nonexistent"), tool_id);
        assert!(result.is_none());
    }
}
