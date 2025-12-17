//! Agent registry for managing available subagents and running agent instances

use super::types::{
    AgentDefinition, AgentProgress, AgentStatus, AgentType, RunningAgent, SubAgentConfig,
};
use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio_util::sync::CancellationToken;

/// Registry for managing available agents and running agent instances
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    /// Registered agent definitions indexed by their type string
    definitions: Arc<RwLock<HashMap<String, AgentDefinition>>>,
    /// Currently running agents indexed by agent ID
    running: Arc<RwLock<HashMap<String, RunningAgent>>>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ===== Agent Definition Management =====

    /// Register a new agent definition
    pub fn register(&self, agent: AgentDefinition) {
        let id = agent.id();
        self.definitions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, agent);
    }

    /// Get an agent definition by type
    pub fn get(&self, agent_type: &AgentType) -> Option<AgentDefinition> {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(agent_type.as_str())
            .cloned()
    }

    /// Get an agent definition by type string
    pub fn get_by_name(&self, name: &str) -> Option<AgentDefinition> {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(name)
            .cloned()
    }

    /// List all registered agent definitions
    pub fn list_definitions(&self) -> Vec<AgentDefinition> {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .cloned()
            .collect()
    }

    /// Check if an agent type is registered
    pub fn contains(&self, agent_type: &AgentType) -> bool {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(agent_type.as_str())
    }

    // ===== Running Agent Management =====

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
        let running = self.running.read().unwrap_or_else(|poisoned| poisoned.into_inner());

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

    /// Get the number of registered agent definitions
    pub fn len(&self) -> usize {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Check if the registry has no definitions
    pub fn is_empty(&self) -> bool {
        self.definitions
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    }

    /// Get the number of running agents
    pub fn running_count(&self) -> usize {
        self.running
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Clear all agent definitions
    pub fn clear_definitions(&self) {
        self.definitions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }

    /// Clear all running agents
    pub fn clear_running(&self) {
        self.running
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::subagent::types::{ExecutionMetadata, SubAgentResult, ToolAccessControl};

    fn create_test_agent(agent_type: AgentType, name: &str) -> AgentDefinition {
        AgentDefinition {
            agent_type,
            name: name.to_string(),
            description: format!("{} agent", name),
            available_tools: ToolAccessControl::All,
            model: None,
            system_prompt: format!("Prompt for {}", name),
        }
    }

    // ===== Agent Definition Tests =====

    #[test]
    fn test_registry_register_and_get() {
        let registry = AgentRegistry::new();
        let agent = create_test_agent(AgentType::GeneralPurpose, "General");

        registry.register(agent.clone());

        let retrieved = registry.get(&AgentType::GeneralPurpose);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "General");
    }

    #[test]
    fn test_registry_get_by_name() {
        let registry = AgentRegistry::new();
        let agent = create_test_agent(AgentType::Plan, "Planner");

        registry.register(agent);

        let retrieved = registry.get_by_name("plan");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_type, AgentType::Plan);
    }

    #[test]
    fn test_registry_list_definitions() {
        let registry = AgentRegistry::new();
        registry.register(create_test_agent(AgentType::GeneralPurpose, "General"));
        registry.register(create_test_agent(AgentType::Explore, "Explorer"));
        registry.register(create_test_agent(AgentType::Plan, "Planner"));

        let agents = registry.list_definitions();
        assert_eq!(agents.len(), 3);
    }

    #[test]
    fn test_registry_contains() {
        let registry = AgentRegistry::new();
        registry.register(create_test_agent(AgentType::GeneralPurpose, "General"));

        assert!(registry.contains(&AgentType::GeneralPurpose));
        assert!(!registry.contains(&AgentType::Explore));
    }

    #[test]
    fn test_registry_len_and_is_empty() {
        let registry = AgentRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        registry.register(create_test_agent(AgentType::GeneralPurpose, "General"));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_clear_definitions() {
        let registry = AgentRegistry::new();
        registry.register(create_test_agent(AgentType::GeneralPurpose, "General"));
        registry.register(create_test_agent(AgentType::Explore, "Explorer"));

        assert_eq!(registry.len(), 2);

        registry.clear_definitions();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    // ===== Running Agent Tests =====

    #[test]
    fn test_create_running_agent() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");

        let agent_id = registry.create_running_agent(config);

        assert!(!agent_id.is_empty());
        assert_eq!(registry.running_count(), 1);
    }

    #[test]
    fn test_update_and_get_status() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
        let agent_id = registry.create_running_agent(config);

        // Initial status should be Pending
        let status = registry.get_status(&agent_id);
        assert!(matches!(status, Some(AgentStatus::Pending)));

        // Update to Running with progress
        let mut progress = AgentProgress::default();
        progress.add_tokens(100);
        registry.update_progress(&agent_id, progress);

        let status = registry.get_status(&agent_id);
        assert!(matches!(status, Some(AgentStatus::Running(_))));

        // Update to Failed
        registry.update_status(&agent_id, AgentStatus::Failed("test error".to_string()));
        let status = registry.get_status(&agent_id);
        assert!(matches!(status, Some(AgentStatus::Failed(_))));
    }

    #[test]
    fn test_update_and_get_progress() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::Explore, "Search files");
        let agent_id = registry.create_running_agent(config);

        // Initially no progress (status is Pending, not Running)
        assert!(registry.get_progress(&agent_id).is_none());

        // Update progress
        let mut progress = AgentProgress::default();
        progress.next_step();
        progress.add_tokens(100);
        progress.increment_tool_use();
        registry.update_progress(&agent_id, progress.clone());

        let retrieved_progress = registry.get_progress(&agent_id);
        assert!(retrieved_progress.is_some());
        let retrieved_progress = retrieved_progress.unwrap();
        assert_eq!(retrieved_progress.current_step, 1);
        assert_eq!(retrieved_progress.token_count, 100);
        assert_eq!(retrieved_progress.tool_use_count, 1);
    }

    #[test]
    fn test_list_running() {
        let registry = AgentRegistry::new();

        let config1 = SubAgentConfig::new(AgentType::GeneralPurpose, "Task 1");
        let id1 = registry.create_running_agent(config1);

        let config2 = SubAgentConfig::new(AgentType::Explore, "Task 2");
        let id2 = registry.create_running_agent(config2);

        let progress = AgentProgress::default();
        registry.update_status(&id1, AgentStatus::Running(progress));

        let result = SubAgentResult {
            agent_id: id2.clone(),
            content: "done".to_string(),
            metadata: ExecutionMetadata::default(),
        };
        registry.update_status(&id2, AgentStatus::Completed(result));

        let running = registry.list_running();
        assert_eq!(running.len(), 2);

        // Find our agents in the list
        let agent1 = running.iter().find(|(id, _, _)| id == &id1);
        assert!(agent1.is_some());
        let (_, agent_type, status) = agent1.unwrap();
        assert_eq!(*agent_type, AgentType::GeneralPurpose);
        assert!(matches!(status, AgentStatus::Running(_)));
    }

    #[test]
    fn test_kill_agent() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
        let agent_id = registry.create_running_agent(config);

        // Get cancel token and verify it's not cancelled
        let token = registry.get_cancel_token(&agent_id);
        assert!(token.is_some());
        let token = token.unwrap();
        assert!(!token.is_cancelled());

        // Kill the agent
        let result = registry.kill(&agent_id);
        assert!(result.is_ok());

        // Token should now be cancelled
        assert!(token.is_cancelled());

        // Trying to kill non-existent agent should fail
        let result = registry.kill("non-existent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_running_agent() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
        let agent_id = registry.create_running_agent(config);

        assert_eq!(registry.running_count(), 1);

        registry.remove(&agent_id);
        assert_eq!(registry.running_count(), 0);
        assert!(registry.get_status(&agent_id).is_none());
    }

    #[test]
    fn test_clear_running() {
        let registry = AgentRegistry::new();

        let config1 = SubAgentConfig::new(AgentType::GeneralPurpose, "Task 1");
        registry.create_running_agent(config1);

        let config2 = SubAgentConfig::new(AgentType::Explore, "Task 2");
        registry.create_running_agent(config2);

        assert_eq!(registry.running_count(), 2);

        registry.clear_running();
        assert_eq!(registry.running_count(), 0);
    }

    #[test]
    fn test_get_cancel_token() {
        let registry = AgentRegistry::new();
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
        let agent_id = registry.create_running_agent(config);

        let token = registry.get_cancel_token(&agent_id);
        assert!(token.is_some());

        let token = registry.get_cancel_token("non-existent-id");
        assert!(token.is_none());
    }

    #[test]
    fn test_registry_clone() {
        let registry1 = AgentRegistry::new();
        registry1.register(create_test_agent(AgentType::GeneralPurpose, "General"));

        let config = SubAgentConfig::new(AgentType::Explore, "Task");
        let agent_id = registry1.create_running_agent(config);

        // Clone the registry
        let registry2 = registry1.clone();

        // Both registries should share the same data
        assert_eq!(registry1.len(), registry2.len());
        assert_eq!(registry1.running_count(), registry2.running_count());

        // Modifications through one registry should be visible in the other
        let progress = AgentProgress::default();
        registry2.update_status(&agent_id, AgentStatus::Running(progress));
        assert!(matches!(
            registry1.get_status(&agent_id),
            Some(AgentStatus::Running(_))
        ));
    }

    #[test]
    fn test_default() {
        let registry = AgentRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.running_count(), 0);
    }
}
