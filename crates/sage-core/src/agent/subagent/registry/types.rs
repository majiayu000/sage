//! Core AgentRegistry type definition

use super::super::types::{AgentDefinition, RunningAgent};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for managing available agents and running agent instances
#[derive(Debug, Clone)]
pub struct AgentRegistry {
    /// Registered agent definitions indexed by their type string
    pub(super) definitions: Arc<RwLock<HashMap<String, AgentDefinition>>>,
    /// Currently running agents indexed by agent ID
    pub(super) running: Arc<RwLock<HashMap<String, RunningAgent>>>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
        }
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
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
