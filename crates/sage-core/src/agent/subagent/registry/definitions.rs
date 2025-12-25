//! Agent definition management methods

use super::super::types::{AgentDefinition, AgentType};
use super::types::AgentRegistry;

impl AgentRegistry {
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

    /// Clear all agent definitions
    pub fn clear_definitions(&self) {
        self.definitions
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
    }
}
