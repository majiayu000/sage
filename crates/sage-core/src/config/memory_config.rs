//! Agent memory runtime configuration.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_MAX_RECALL_ITEMS: usize = 8;
const DEFAULT_MAX_RECALL_CHARS: usize = 4_000;
const DEFAULT_MAX_STORED_OUTCOME_CHARS: usize = 2_000;

/// Configuration for cross-session agent memory and learning recall.
#[derive(Debug, Clone)]
pub struct AgentMemoryConfig {
    /// Whether agent memory and learning recall are enabled.
    pub enabled: bool,
    /// Whether `enabled` was declared by a config source.
    pub enabled_set: bool,
    /// Optional memory storage path. Relative paths resolve from the working directory.
    pub storage_path: Option<PathBuf>,
    /// Maximum remembered items injected into a prompt.
    pub max_recall_items: usize,
    /// Whether `max_recall_items` was declared by a config source.
    pub max_recall_items_set: bool,
    /// Maximum rendered recall characters injected into a prompt.
    pub max_recall_chars: usize,
    /// Whether `max_recall_chars` was declared by a config source.
    pub max_recall_chars_set: bool,
    /// Maximum characters retained from a completed task outcome.
    pub max_stored_outcome_chars: usize,
    /// Whether `max_stored_outcome_chars` was declared by a config source.
    pub max_stored_outcome_chars_set: bool,
}

impl Default for AgentMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            enabled_set: false,
            storage_path: None,
            max_recall_items: DEFAULT_MAX_RECALL_ITEMS,
            max_recall_items_set: false,
            max_recall_chars: DEFAULT_MAX_RECALL_CHARS,
            max_recall_chars_set: false,
            max_stored_outcome_chars: DEFAULT_MAX_STORED_OUTCOME_CHARS,
            max_stored_outcome_chars_set: false,
        }
    }
}

impl AgentMemoryConfig {
    /// Merge another memory config, preserving source-declared field semantics.
    pub fn merge(&mut self, other: AgentMemoryConfig) {
        if other.enabled_set {
            self.enabled = other.enabled;
            self.enabled_set = true;
        }

        if other.storage_path.is_some() {
            self.storage_path = other.storage_path;
        }

        if other.max_recall_items_set {
            self.max_recall_items = other.max_recall_items;
            self.max_recall_items_set = true;
        }

        if other.max_recall_chars_set {
            self.max_recall_chars = other.max_recall_chars;
            self.max_recall_chars_set = true;
        }

        if other.max_stored_outcome_chars_set {
            self.max_stored_outcome_chars = other.max_stored_outcome_chars;
            self.max_stored_outcome_chars_set = true;
        }
    }
}

impl Serialize for AgentMemoryConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let include_items =
            self.max_recall_items_set || self.max_recall_items != DEFAULT_MAX_RECALL_ITEMS;
        let include_chars =
            self.max_recall_chars_set || self.max_recall_chars != DEFAULT_MAX_RECALL_CHARS;
        let include_outcome = self.max_stored_outcome_chars_set
            || self.max_stored_outcome_chars != DEFAULT_MAX_STORED_OUTCOME_CHARS;
        let len = 1
            + usize::from(self.storage_path.is_some())
            + usize::from(include_items)
            + usize::from(include_chars)
            + usize::from(include_outcome);
        let mut state = serializer.serialize_struct("AgentMemoryConfig", len)?;
        state.serialize_field("enabled", &self.enabled)?;
        if let Some(path) = &self.storage_path {
            state.serialize_field("storage_path", path)?;
        }
        if include_items {
            state.serialize_field("max_recall_items", &self.max_recall_items)?;
        }
        if include_chars {
            state.serialize_field("max_recall_chars", &self.max_recall_chars)?;
        }
        if include_outcome {
            state.serialize_field("max_stored_outcome_chars", &self.max_stored_outcome_chars)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for AgentMemoryConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct AgentMemoryConfigWire {
            enabled: Option<bool>,
            storage_path: Option<PathBuf>,
            max_recall_items: Option<usize>,
            max_recall_chars: Option<usize>,
            max_stored_outcome_chars: Option<usize>,
        }

        let wire = AgentMemoryConfigWire::deserialize(deserializer)?;
        Ok(Self {
            enabled: wire.enabled.unwrap_or_default(),
            enabled_set: wire.enabled.is_some(),
            storage_path: wire.storage_path,
            max_recall_items: wire.max_recall_items.unwrap_or(DEFAULT_MAX_RECALL_ITEMS),
            max_recall_items_set: wire.max_recall_items.is_some(),
            max_recall_chars: wire.max_recall_chars.unwrap_or(DEFAULT_MAX_RECALL_CHARS),
            max_recall_chars_set: wire.max_recall_chars.is_some(),
            max_stored_outcome_chars: wire
                .max_stored_outcome_chars
                .unwrap_or(DEFAULT_MAX_STORED_OUTCOME_CHARS),
            max_stored_outcome_chars_set: wire.max_stored_outcome_chars.is_some(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_config_defaults_to_disabled() {
        let config = AgentMemoryConfig::default();

        assert!(!config.enabled);
        assert!(!config.enabled_set);
        assert_eq!(config.max_recall_items, DEFAULT_MAX_RECALL_ITEMS);
    }

    #[test]
    fn memory_config_deserialize_tracks_declared_fields() {
        let config: AgentMemoryConfig = serde_json::from_str(
            r#"{
                "enabled": false,
                "storage_path": ".sage/custom-memory.json",
                "max_recall_items": 3
            }"#,
        )
        .unwrap();

        assert!(!config.enabled);
        assert!(config.enabled_set);
        assert_eq!(
            config.storage_path,
            Some(PathBuf::from(".sage/custom-memory.json"))
        );
        assert_eq!(config.max_recall_items, 3);
        assert!(config.max_recall_items_set);
        assert_eq!(config.max_recall_chars, DEFAULT_MAX_RECALL_CHARS);
        assert!(!config.max_recall_chars_set);
    }

    #[test]
    fn memory_config_merge_allows_later_disable() {
        let mut base = AgentMemoryConfig {
            enabled: true,
            enabled_set: true,
            ..Default::default()
        };
        let other: AgentMemoryConfig = serde_json::from_str(r#"{"enabled": false}"#).unwrap();

        base.merge(other);

        assert!(!base.enabled);
        assert!(base.enabled_set);
    }
}
