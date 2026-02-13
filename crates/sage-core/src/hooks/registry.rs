//! Hook registry for managing hooks

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::events::HookEvent;
use super::types::{HookConfig, HookMatcher};
use crate::error::{SageError, SageResult};

/// Registry for managing hooks with event-based organization
#[derive(Debug, Clone)]
pub struct HookRegistry {
    /// Hooks organized by HookEvent with matchers
    event_hooks: Arc<RwLock<HashMap<HookEvent, Vec<HookMatcher>>>>,
}

impl HookRegistry {
    /// Create a new empty hook registry
    pub fn new() -> Self {
        Self {
            event_hooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ===== Event-based hook methods =====

    /// Register a hook matcher for an event
    pub fn register(&self, event: HookEvent, matcher: HookMatcher) -> SageResult<()> {
        let mut event_hooks = self
            .event_hooks
            .write()
            .map_err(|e| SageError::other(format!("Hook registry lock poisoned: {}", e)))?;
        let hook_list = event_hooks.entry(event).or_insert_with(Vec::new);
        hook_list.push(matcher);
        Ok(())
    }

    /// Get all matching hooks for an event and query value
    pub fn get_matching(&self, event: HookEvent, query: &str) -> Vec<HookConfig> {
        let event_hooks = self.event_hooks.read().ok();
        event_hooks
            .and_then(|hooks| hooks.get(&event).cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|matcher| {
                // Check if pattern matches
                let matches = super::matcher::matches(matcher.pattern.as_deref(), query);
                if matches {
                    // Return the hook from this matcher
                    Some(matcher.hook)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if any hooks are registered for an event
    pub fn has_hooks(&self, event: &HookEvent) -> bool {
        self.event_hooks
            .read()
            .ok()
            .and_then(|hooks| hooks.get(event).map(|list| !list.is_empty()))
            .unwrap_or(false)
    }

    /// List all events with registered hooks
    pub fn list_events(&self) -> Vec<HookEvent> {
        self.event_hooks
            .read()
            .ok()
            .map(|hooks| {
                hooks
                    .iter()
                    .filter(|(_, list)| !list.is_empty())
                    .map(|(event, _)| *event)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear all event hooks
    pub fn clear(&self) -> SageResult<()> {
        let mut event_hooks = self
            .event_hooks
            .write()
            .map_err(|e| SageError::other(format!("Hook registry lock poisoned: {}", e)))?;
        event_hooks.clear();
        Ok(())
    }

    /// Get the number of registered hook matchers
    pub fn count(&self) -> usize {
        self.event_hooks
            .read()
            .ok()
            .map(|h| h.values().map(|v| v.len()).sum())
            .unwrap_or(0)
    }

    /// Load from configuration
    pub fn from_config(config: &HooksConfig) -> Self {
        let registry = Self::new();

        // Register all event hooks
        for matcher in &config.pre_tool_use {
            let _ = registry.register(HookEvent::PreToolUse, matcher.clone());
        }
        for matcher in &config.post_tool_use {
            let _ = registry.register(HookEvent::PostToolUse, matcher.clone());
        }
        for matcher in &config.post_tool_use_failure {
            let _ = registry.register(HookEvent::PostToolUseFailure, matcher.clone());
        }
        for matcher in &config.user_prompt_submit {
            let _ = registry.register(HookEvent::UserPromptSubmit, matcher.clone());
        }
        for matcher in &config.session_start {
            let _ = registry.register(HookEvent::SessionStart, matcher.clone());
        }
        for matcher in &config.session_end {
            let _ = registry.register(HookEvent::SessionEnd, matcher.clone());
        }
        for matcher in &config.subagent_start {
            let _ = registry.register(HookEvent::SubagentStart, matcher.clone());
        }
        for matcher in &config.subagent_stop {
            let _ = registry.register(HookEvent::SubagentStop, matcher.clone());
        }
        for matcher in &config.permission_request {
            let _ = registry.register(HookEvent::PermissionRequest, matcher.clone());
        }
        for matcher in &config.pre_compact {
            let _ = registry.register(HookEvent::PreCompact, matcher.clone());
        }
        for matcher in &config.notification {
            let _ = registry.register(HookEvent::Notification, matcher.clone());
        }
        for matcher in &config.stop {
            let _ = registry.register(HookEvent::Stop, matcher.clone());
        }
        for matcher in &config.status_line {
            let _ = registry.register(HookEvent::StatusLine, matcher.clone());
        }

        registry
    }
}

/// Configuration structure for hooks
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HooksConfig {
    /// Hooks for before tool execution
    #[serde(default)]
    pub pre_tool_use: Vec<HookMatcher>,

    /// Hooks for after successful tool execution
    #[serde(default)]
    pub post_tool_use: Vec<HookMatcher>,

    /// Hooks for after failed tool execution
    #[serde(default)]
    pub post_tool_use_failure: Vec<HookMatcher>,

    /// Hooks for user prompt submission
    #[serde(default)]
    pub user_prompt_submit: Vec<HookMatcher>,

    /// Hooks for session start
    #[serde(default)]
    pub session_start: Vec<HookMatcher>,

    /// Hooks for session end
    #[serde(default)]
    pub session_end: Vec<HookMatcher>,

    /// Hooks for sub-agent start
    #[serde(default)]
    pub subagent_start: Vec<HookMatcher>,

    /// Hooks for sub-agent stop
    #[serde(default)]
    pub subagent_stop: Vec<HookMatcher>,

    /// Hooks for permission requests
    #[serde(default)]
    pub permission_request: Vec<HookMatcher>,

    /// Hooks for before context compaction
    #[serde(default)]
    pub pre_compact: Vec<HookMatcher>,

    /// Hooks for notifications
    #[serde(default)]
    pub notification: Vec<HookMatcher>,

    /// Hooks for agent stop
    #[serde(default)]
    pub stop: Vec<HookMatcher>,

    /// Hooks for status line updates
    #[serde(default)]
    pub status_line: Vec<HookMatcher>,
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
