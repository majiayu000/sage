//! Runtime configuration persistence
//!
//! This module provides functionality for runtime configuration changes
//! that persist across sessions, similar to Crush's SetConfigField pattern.

use crate::error::{SageError, SageResult};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Configuration persistence manager
///
/// Handles reading and writing configuration fields at runtime,
/// supporting dot-notation paths for nested values.
#[derive(Debug, Clone)]
pub struct ConfigPersistence {
    /// Path to the main config file
    config_path: PathBuf,
    /// Path to the credentials file
    credentials_path: PathBuf,
}

impl ConfigPersistence {
    /// Create a new persistence manager for the given directory
    pub fn new(base_dir: &Path) -> Self {
        Self {
            config_path: base_dir.join("config.json"),
            credentials_path: base_dir.join("credentials.json"),
        }
    }

    /// Create persistence manager with default paths (~/.sage)
    pub fn with_defaults() -> Self {
        let base_dir = dirs::home_dir().unwrap_or_default().join(".sage");
        Self::new(&base_dir)
    }

    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get the credentials file path
    pub fn credentials_path(&self) -> &Path {
        &self.credentials_path
    }

    /// Set a configuration field using dot-notation path
    ///
    /// # Arguments
    /// * `path` - Dot-notation path (e.g., "providers.anthropic.api_key")
    /// * `value` - The value to set
    ///
    /// # Example
    /// ```ignore
    /// persistence.set_field("default_provider", json!("anthropic"))?;
    /// persistence.set_field("providers.openai.model", json!("gpt-4"))?;
    /// ```
    pub fn set_field(&self, path: &str, value: Value) -> SageResult<()> {
        let mut config = self.load_config_json()?;
        set_nested_value(&mut config, path, value);
        self.save_config_json(&config)
    }

    /// Get a configuration field using dot-notation path
    ///
    /// Returns None if the path doesn't exist
    pub fn get_field(&self, path: &str) -> Option<Value> {
        let config = self.load_config_json().ok()?;
        get_nested_value(&config, path)
    }

    /// Remove a configuration field using dot-notation path
    pub fn remove_field(&self, path: &str) -> SageResult<()> {
        let mut config = self.load_config_json()?;
        remove_nested_value(&mut config, path);
        self.save_config_json(&config)
    }

    /// Set the default provider
    pub fn set_default_provider(&self, provider: &str) -> SageResult<()> {
        self.set_field("default_provider", Value::String(provider.to_string()))
    }

    /// Get the default provider
    pub fn get_default_provider(&self) -> Option<String> {
        self.get_field("default_provider")
            .and_then(|v| v.as_str().map(String::from))
    }

    /// Set an API key for a provider in the credentials file
    pub fn set_api_key(&self, provider: &str, api_key: &str) -> SageResult<()> {
        let mut creds = self.load_credentials_json()?;

        // Ensure api_keys object exists
        if !creds
            .get("api_keys")
            .map(|v| v.is_object())
            .unwrap_or(false)
        {
            creds["api_keys"] = Value::Object(serde_json::Map::new());
        }

        creds["api_keys"][provider] = Value::String(api_key.to_string());
        self.save_credentials_json(&creds)
    }

    /// Get an API key for a provider from the credentials file
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        let creds = self.load_credentials_json().ok()?;
        creds
            .get("api_keys")?
            .get(provider)?
            .as_str()
            .map(String::from)
    }

    /// Remove an API key for a provider
    pub fn remove_api_key(&self, provider: &str) -> SageResult<()> {
        let mut creds = self.load_credentials_json()?;

        if let Some(api_keys) = creds.get_mut("api_keys") {
            if let Some(obj) = api_keys.as_object_mut() {
                obj.remove(provider);
            }
        }

        self.save_credentials_json(&creds)
    }

    /// Load the config JSON file
    fn load_config_json(&self) -> SageResult<Value> {
        self.load_json_file(&self.config_path)
    }

    /// Save the config JSON file
    fn save_config_json(&self, value: &Value) -> SageResult<()> {
        self.save_json_file(&self.config_path, value)
    }

    /// Load the credentials JSON file
    fn load_credentials_json(&self) -> SageResult<Value> {
        self.load_json_file(&self.credentials_path)
    }

    /// Save the credentials JSON file
    fn save_credentials_json(&self, value: &Value) -> SageResult<()> {
        self.save_json_file(&self.credentials_path, value)
    }

    /// Load a JSON file, returning empty object if it doesn't exist
    fn load_json_file(&self, path: &Path) -> SageResult<Value> {
        if !path.exists() {
            return Ok(Value::Object(serde_json::Map::new()));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| SageError::io(format!("Failed to read {}: {}", path.display(), e)))?;

        if content.trim().is_empty() {
            return Ok(Value::Object(serde_json::Map::new()));
        }

        serde_json::from_str(&content)
            .map_err(|e| SageError::config(format!("Failed to parse {}: {}", path.display(), e)))
    }

    /// Save a JSON file, creating parent directories if needed
    fn save_json_file(&self, path: &Path, value: &Value) -> SageResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SageError::io(format!("Failed to create directory: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(value)
            .map_err(|e| SageError::config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| SageError::io(format!("Failed to write {}: {}", path.display(), e)))?;

        debug!("Saved configuration to {}", path.display());
        Ok(())
    }
}

/// Set a nested value in a JSON object using dot-notation path
fn set_nested_value(root: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return;
    }

    let mut current = root;

    // Navigate to the parent of the target field
    for part in &parts[..parts.len() - 1] {
        // Ensure the current level is an object
        if !current.is_object() {
            *current = Value::Object(serde_json::Map::new());
        }

        // Get or create the next level
        if !current.get(*part).map(|v| v.is_object()).unwrap_or(false) {
            current[*part] = Value::Object(serde_json::Map::new());
        }

        current = current.get_mut(*part).unwrap();
    }

    // Set the final value
    if !current.is_object() {
        *current = Value::Object(serde_json::Map::new());
    }
    current[parts.last().unwrap()] = value;
}

/// Get a nested value from a JSON object using dot-notation path
fn get_nested_value(root: &Value, path: &str) -> Option<Value> {
    let parts: Vec<&str> = path.split('.').collect();

    let mut current = root;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}

/// Remove a nested value from a JSON object using dot-notation path
fn remove_nested_value(root: &mut Value, path: &str) {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        if let Some(obj) = root.as_object_mut() {
            obj.remove(parts[0]);
        }
        return;
    }

    let mut current = root;

    // Navigate to the parent of the target field
    for part in &parts[..parts.len() - 1] {
        if let Some(next) = current.get_mut(*part) {
            current = next;
        } else {
            return; // Path doesn't exist
        }
    }

    // Remove the final key
    if let Some(obj) = current.as_object_mut() {
        if let Some(key) = parts.last() {
            obj.remove(*key);
        }
    }
}

/// Configuration update builder for batch operations
#[derive(Debug, Default)]
pub struct ConfigUpdate {
    fields: Vec<(String, Value)>,
    removals: Vec<String>,
}

impl ConfigUpdate {
    /// Create a new config update builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a field value
    pub fn set(mut self, path: impl Into<String>, value: Value) -> Self {
        self.fields.push((path.into(), value));
        self
    }

    /// Remove a field
    pub fn remove(mut self, path: impl Into<String>) -> Self {
        self.removals.push(path.into());
        self
    }

    /// Apply all updates to the persistence manager
    pub fn apply(self, persistence: &ConfigPersistence) -> SageResult<()> {
        let mut config = persistence.load_config_json()?;

        // Apply removals first
        for path in self.removals {
            remove_nested_value(&mut config, &path);
        }

        // Then apply sets
        for (path, value) in self.fields {
            set_nested_value(&mut config, &path, value);
        }

        persistence.save_config_json(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn test_set_nested_value_simple() {
        let mut root = json!({});
        set_nested_value(&mut root, "key", json!("value"));
        assert_eq!(root["key"], json!("value"));
    }

    #[test]
    fn test_set_nested_value_deep() {
        let mut root = json!({});
        set_nested_value(&mut root, "a.b.c", json!(123));
        assert_eq!(root["a"]["b"]["c"], json!(123));
    }

    #[test]
    fn test_get_nested_value() {
        let root = json!({
            "a": {
                "b": {
                    "c": "value"
                }
            }
        });

        assert_eq!(get_nested_value(&root, "a.b.c"), Some(json!("value")));
        assert_eq!(get_nested_value(&root, "a.b"), Some(json!({"c": "value"})));
        assert_eq!(get_nested_value(&root, "nonexistent"), None);
    }

    #[test]
    fn test_remove_nested_value() {
        let mut root = json!({
            "a": {
                "b": {
                    "c": "value"
                }
            }
        });

        remove_nested_value(&mut root, "a.b.c");
        assert!(root["a"]["b"].get("c").is_none());
    }

    #[test]
    fn test_persistence_set_get_field() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence
            .set_field("default_provider", json!("anthropic"))
            .unwrap();
        assert_eq!(
            persistence.get_field("default_provider"),
            Some(json!("anthropic"))
        );
    }

    #[test]
    fn test_persistence_nested_field() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence
            .set_field("providers.openai.model", json!("gpt-4"))
            .unwrap();
        assert_eq!(
            persistence.get_field("providers.openai.model"),
            Some(json!("gpt-4"))
        );
    }

    #[test]
    fn test_persistence_remove_field() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence.set_field("test.key", json!("value")).unwrap();
        persistence.remove_field("test.key").unwrap();
        assert_eq!(persistence.get_field("test.key"), None);
    }

    #[test]
    fn test_persistence_api_key() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence.set_api_key("anthropic", "sk-ant-test").unwrap();
        assert_eq!(
            persistence.get_api_key("anthropic"),
            Some("sk-ant-test".to_string())
        );
    }

    #[test]
    fn test_persistence_remove_api_key() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence.set_api_key("test", "key").unwrap();
        persistence.remove_api_key("test").unwrap();
        assert_eq!(persistence.get_api_key("test"), None);
    }

    #[test]
    fn test_config_update_batch() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        ConfigUpdate::new()
            .set("a", json!(1))
            .set("b.c", json!(2))
            .apply(&persistence)
            .unwrap();

        assert_eq!(persistence.get_field("a"), Some(json!(1)));
        assert_eq!(persistence.get_field("b.c"), Some(json!(2)));
    }

    #[test]
    fn test_default_provider() {
        let dir = tempdir().unwrap();
        let persistence = ConfigPersistence::new(dir.path());

        persistence.set_default_provider("openai").unwrap();
        assert_eq!(
            persistence.get_default_provider(),
            Some("openai".to_string())
        );
    }
}
