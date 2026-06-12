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
                        // Deny rules take precedence over allow rules.
                        for pattern in &settings.permissions.deny {
                            if Self::pattern_matches(pattern, key) {
                                return Some(false);
                            }
                        }
                        for pattern in &settings.permissions.allow {
                            if Self::pattern_matches(pattern, key) {
                                return Some(true);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if a pattern matches a key
    pub(crate) fn pattern_matches(pattern: &str, key: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let (Some((pattern_tool, pattern_arg)), Some((key_tool, key_arg))) = (
            Self::split_permission_key(pattern),
            Self::split_permission_key(key),
        ) {
            if !pattern_tool.eq_ignore_ascii_case(key_tool) {
                return false;
            }

            return if Self::uses_path_glob(pattern_tool) && Self::uses_path_glob(key_tool) {
                Self::path_glob_matches(pattern_arg, key_arg)
            } else {
                Self::glob_matches(pattern_arg, key_arg)
            };
        }

        if pattern.contains('*') {
            return Self::glob_matches(pattern, key);
        }

        if let Some((key_tool, _)) = Self::split_permission_key(key) {
            return pattern.eq_ignore_ascii_case(key_tool);
        }

        pattern.eq_ignore_ascii_case(key)
    }

    fn uses_path_glob(tool_name: &str) -> bool {
        matches!(
            tool_name.to_lowercase().as_str(),
            "read"
                | "write"
                | "edit"
                | "multiedit"
                | "multi_edit"
                | "grep"
                | "glob"
                | "notebookedit"
                | "notebook_edit"
        )
    }

    fn split_permission_key(value: &str) -> Option<(&str, &str)> {
        let open = value.find('(')?;
        let close = value.rfind(')')?;
        if close <= open {
            return None;
        }

        Some((value[..open].trim(), &value[open + 1..close]))
    }

    fn glob_matches(pattern: &str, text: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if !pattern.contains('*') {
            return pattern == text;
        }

        let mut remaining = text;
        let mut parts = pattern.split('*').peekable();

        if let Some(first) = parts.next() {
            if !first.is_empty() {
                let Some(stripped) = remaining.strip_prefix(first) else {
                    return false;
                };
                remaining = stripped;
            }
        }

        while let Some(part) = parts.next() {
            if part.is_empty() {
                continue;
            }

            if parts.peek().is_none() {
                return remaining.ends_with(part);
            }

            let Some(index) = remaining.find(part) else {
                return false;
            };
            remaining = &remaining[index + part.len()..];
        }

        pattern.ends_with('*') || remaining.is_empty()
    }

    fn path_glob_matches(pattern: &str, text: &str) -> bool {
        let pattern: Vec<char> = pattern.chars().collect();
        let text: Vec<char> = text.chars().collect();
        let mut memo = HashMap::new();
        Self::path_glob_matches_inner(&pattern, &text, 0, 0, &mut memo)
    }

    fn path_glob_matches_inner(
        pattern: &[char],
        text: &[char],
        pattern_index: usize,
        text_index: usize,
        memo: &mut HashMap<(usize, usize), bool>,
    ) -> bool {
        if let Some(result) = memo.get(&(pattern_index, text_index)) {
            return *result;
        }

        let result = if pattern_index == pattern.len() {
            text_index == text.len()
        } else if pattern[pattern_index] == '*' {
            if pattern.get(pattern_index + 1) == Some(&'*') {
                (text_index..=text.len()).any(|next| {
                    Self::path_glob_matches_inner(pattern, text, pattern_index + 2, next, memo)
                })
            } else {
                let mut next = text_index;
                loop {
                    if Self::path_glob_matches_inner(pattern, text, pattern_index + 1, next, memo) {
                        break true;
                    }
                    if next == text.len() || text[next] == '/' {
                        break false;
                    }
                    next += 1;
                }
            }
        } else {
            text.get(text_index) == Some(&pattern[pattern_index])
                && Self::path_glob_matches_inner(
                    pattern,
                    text,
                    pattern_index + 1,
                    text_index + 1,
                    memo,
                )
        };

        memo.insert((pattern_index, text_index), result);
        result
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
        self.session_cache
            .write()
            .await
            .insert(key.clone(), allowed);

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
#[path = "cache_tests.rs"]
mod cache_tests;
