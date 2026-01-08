//! Permission cache for "always allow" / "always deny" decisions
//!
//! Provides both in-memory and persistent caching of permission decisions.
//! Persistent decisions are saved to `.sage/settings.local.json`.

use crate::error::SageResult;
use crate::settings::{Settings, SettingsLoader};
use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Permission cache for "always allow" / "always deny" decisions
///
/// This cache operates in two modes:
/// 1. **Session cache**: In-memory cache that lasts for the current session
/// 2. **Persistent cache**: Saved to `.sage/settings.local.json` for cross-session persistence
#[derive(Debug)]
pub struct PermissionCache {
    /// In-memory session cache
    session_cache: RwLock<HashMap<String, bool>>,
    /// Path to settings file for persistence (if enabled)
    settings_path: Option<PathBuf>,
    /// Whether persistence is enabled
    persist_enabled: bool,
}

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionCache {
    /// Create a new permission cache (session-only)
    pub fn new() -> Self {
        Self {
            session_cache: RwLock::new(HashMap::new()),
            settings_path: None,
            persist_enabled: false,
        }
    }

    /// Create a permission cache with persistence enabled
    ///
    /// Persistent decisions are saved to `.sage/settings.local.json` in the
    /// specified directory (usually the project root).
    pub fn with_persistence(project_dir: impl Into<PathBuf>) -> Self {
        let project_dir = project_dir.into();
        let settings_path = project_dir.join(".sage").join("settings.local.json");

        Self {
            session_cache: RwLock::new(HashMap::new()),
            settings_path: Some(settings_path),
            persist_enabled: true,
        }
    }

    /// Enable or disable persistence
    pub fn set_persistence_enabled(&mut self, enabled: bool) {
        self.persist_enabled = enabled;
    }

    /// Generate cache key for a tool call
    ///
    /// Creates a deterministic key based on tool name and its primary argument.
    /// For example:
    /// - `Bash(npm install)` -> `"Bash(npm *)"`
    /// - `Read(/src/main.rs)` -> `"Read(src/**)"`
    pub fn cache_key(tool_name: &str, call: &ToolCall) -> String {
        // Extract the primary argument for the permission pattern
        let arg_pattern = Self::extract_pattern_from_call(tool_name, call);

        if let Some(pattern) = arg_pattern {
            format!("{}({})", tool_name, pattern)
        } else {
            tool_name.to_string()
        }
    }

    /// Extract a generalizable pattern from tool arguments
    fn extract_pattern_from_call(tool_name: &str, call: &ToolCall) -> Option<String> {
        match tool_name.to_lowercase().as_str() {
            "bash" => {
                // For bash, use the command prefix (first word + *)
                call.arguments
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(|cmd| {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if parts.len() > 1 {
                            format!("{} *", parts[0])
                        } else {
                            parts.first().unwrap_or(&"*").to_string()
                        }
                    })
            }
            "read" | "write" | "edit" => {
                // For file operations, use directory pattern
                call.arguments
                    .get("file_path")
                    .or_else(|| call.arguments.get("path"))
                    .and_then(|v| v.as_str())
                    .map(|path| {
                        // Convert to directory pattern: /foo/bar/file.rs -> foo/**
                        let path = path.trim_start_matches('/');
                        if let Some(first_dir) = path.split('/').next() {
                            format!("{}/**", first_dir)
                        } else {
                            "**".to_string()
                        }
                    })
            }
            _ => {
                // For other tools, use full argument keys
                let arg_keys: Vec<_> = call.arguments.keys().collect();
                if arg_keys.is_empty() {
                    None
                } else {
                    Some(format!("{:?}", arg_keys))
                }
            }
        }
    }

    /// Check if there's a cached decision (session cache only)
    pub async fn get(&self, key: &str) -> Option<bool> {
        self.session_cache.read().await.get(key).copied()
    }

    /// Check if there's a cached decision (including persistent settings)
    pub async fn get_with_persistence(&self, key: &str) -> Option<bool> {
        // First check session cache
        if let Some(decision) = self.session_cache.read().await.get(key).copied() {
            return Some(decision);
        }

        // Then check persistent settings if enabled
        if self.persist_enabled {
            if let Some(ref settings_path) = self.settings_path {
                if let Some(parent) = settings_path.parent() {
                    let loader = SettingsLoader::from_directory(parent);
                    if let Ok(settings) = loader.load() {
                        // Check if key matches any allow/deny pattern
                        for pattern in &settings.permissions.allow {
                            if pattern == key || Self::pattern_matches(pattern, key) {
                                return Some(true);
                            }
                        }
                        for pattern in &settings.permissions.deny {
                            if pattern == key || Self::pattern_matches(pattern, key) {
                                return Some(false);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if a pattern matches a key
    fn pattern_matches(pattern: &str, key: &str) -> bool {
        // Simple glob matching
        if pattern == key {
            return true;
        }

        // Handle wildcards
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                let (prefix, suffix) = (pattern_parts[0], pattern_parts[1]);
                return key.starts_with(prefix) && key.ends_with(suffix);
            }
        }

        false
    }

    /// Cache a decision (session only)
    pub async fn set(&self, key: String, allowed: bool) {
        self.session_cache.write().await.insert(key, allowed);
    }

    /// Cache a decision with optional persistence
    ///
    /// If `persist` is true and persistence is enabled, the decision is also
    /// saved to `.sage/settings.local.json`.
    pub async fn set_with_persistence(
        &self,
        key: String,
        allowed: bool,
        persist: bool,
    ) -> SageResult<()> {
        // Always update session cache
        self.session_cache.write().await.insert(key.clone(), allowed);

        // Persist if requested and enabled
        if persist && self.persist_enabled {
            self.persist_decision(&key, allowed).await?;
        }

        Ok(())
    }

    /// Persist a decision to settings file
    async fn persist_decision(&self, key: &str, allowed: bool) -> SageResult<()> {
        let Some(ref settings_path) = self.settings_path else {
            return Ok(());
        };

        // Load existing settings or create new
        let loader = SettingsLoader::new().without_validation();
        let mut settings = if settings_path.exists() {
            loader.load_from_file(settings_path).unwrap_or_default()
        } else {
            Settings::default()
        };

        // Add to appropriate list
        if allowed {
            if !settings.permissions.allow.contains(&key.to_string()) {
                settings.permissions.allow.push(key.to_string());
            }
            // Remove from deny if present
            settings.permissions.deny.retain(|p| p != key);
        } else {
            if !settings.permissions.deny.contains(&key.to_string()) {
                settings.permissions.deny.push(key.to_string());
            }
            // Remove from allow if present
            settings.permissions.allow.retain(|p| p != key);
        }

        // Save settings
        loader.save_to_file(&settings, settings_path)?;

        tracing::info!(
            "Persisted permission decision: {} = {} to {:?}",
            key,
            if allowed { "allow" } else { "deny" },
            settings_path
        );

        Ok(())
    }

    /// Clear the session cache (does not affect persistent settings)
    pub async fn clear(&self) {
        self.session_cache.write().await.clear();
    }

    /// Get the path to the persistent settings file
    pub fn settings_path(&self) -> Option<&PathBuf> {
        self.settings_path.as_ref()
    }

    /// Check if persistence is enabled
    pub fn is_persistence_enabled(&self) -> bool {
        self.persist_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_cache() {
        let cache = PermissionCache::new();

        cache.set("test_key".to_string(), true).await;
        assert_eq!(cache.get("test_key").await, Some(true));

        cache.set("test_key".to_string(), false).await;
        assert_eq!(cache.get("test_key").await, Some(false));
    }

    #[tokio::test]
    async fn test_persistent_cache() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        let cache = PermissionCache::with_persistence(temp_dir.path());

        // Persist a decision
        cache
            .set_with_persistence("Bash(npm *)".to_string(), true, true)
            .await
            .unwrap();

        // Check that it was saved
        let settings_path = sage_dir.join("settings.local.json");
        assert!(settings_path.exists());

        let content = fs::read_to_string(&settings_path).unwrap();
        assert!(content.contains("Bash(npm *)"));
    }

    #[tokio::test]
    async fn test_cache_key_bash() {
        let mut arguments = HashMap::new();
        arguments.insert(
            "command".to_string(),
            serde_json::Value::String("npm install lodash".to_string()),
        );

        let call = ToolCall {
            id: "test-id".to_string(),
            name: "bash".to_string(),
            arguments,
            call_id: None,
        };

        let key = PermissionCache::cache_key("Bash", &call);
        assert_eq!(key, "Bash(npm *)");
    }

    #[tokio::test]
    async fn test_cache_key_read() {
        let mut arguments = HashMap::new();
        arguments.insert(
            "file_path".to_string(),
            serde_json::Value::String("/src/main.rs".to_string()),
        );

        let call = ToolCall {
            id: "test-id".to_string(),
            name: "read".to_string(),
            arguments,
            call_id: None,
        };

        let key = PermissionCache::cache_key("Read", &call);
        assert_eq!(key, "Read(src/**)");
    }

    #[tokio::test]
    async fn test_pattern_matches() {
        assert!(PermissionCache::pattern_matches("Bash(npm *)", "Bash(npm *)"));
        assert!(PermissionCache::pattern_matches(
            "Bash(npm *)",
            "Bash(npm install)"
        ));
        assert!(!PermissionCache::pattern_matches(
            "Bash(npm *)",
            "Bash(yarn install)"
        ));
    }

    #[tokio::test]
    async fn test_get_with_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        // Create settings file with pre-existing permissions
        let settings_content = r#"{
            "permissions": {
                "allow": ["Read(src/**)"],
                "deny": ["Bash(rm *)"]
            }
        }"#;
        fs::write(sage_dir.join("settings.local.json"), settings_content).unwrap();

        let cache = PermissionCache::with_persistence(temp_dir.path());

        // Should find in persistent settings
        assert_eq!(
            cache.get_with_persistence("Read(src/**)").await,
            Some(true)
        );
        assert_eq!(cache.get_with_persistence("Bash(rm *)").await, Some(false));

        // Should not find non-existent
        assert_eq!(cache.get_with_persistence("Unknown").await, None);
    }
}
