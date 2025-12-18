//! Hook registry for managing hooks

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::events::HookEvent;
use super::types::{HookConfig, HookMatcher};

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
    pub fn register(&self, event: HookEvent, matcher: HookMatcher) -> Result<(), String> {
        let mut event_hooks = self.event_hooks.write().map_err(|e| e.to_string())?;
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
    pub fn clear(&self) -> Result<(), String> {
        let mut event_hooks = self.event_hooks.write().map_err(|e| e.to_string())?;
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
mod tests {
    use super::*;
    use crate::hooks::types::{CommandHook, HookConfig, HookImplementation, HookType};

    fn create_test_matcher(pattern: Option<String>) -> HookMatcher {
        let config = HookConfig {
            name: "test_hook".to_string(),
            hook_type: HookType::PreToolExecution,
            implementation: HookImplementation::Command(CommandHook::new("echo test")),
            can_block: false,
            timeout_secs: 30,
            enabled: true,
        };
        HookMatcher::new(pattern, config)
    }

    #[test]
    fn test_register() {
        let registry = HookRegistry::new();
        let matcher = create_test_matcher(Some("bash".to_string()));

        assert!(registry.register(HookEvent::PreToolUse, matcher).is_ok());
        assert!(registry.has_hooks(&HookEvent::PreToolUse));
    }

    #[test]
    fn test_get_matching() {
        let registry = HookRegistry::new();

        // Register hooks with different patterns
        let matcher1 = create_test_matcher(Some("bash".to_string()));
        let matcher2 = create_test_matcher(Some("python".to_string()));
        let matcher3 = create_test_matcher(None); // Wildcard

        registry
            .register(HookEvent::PreToolUse, matcher1)
            .unwrap();
        registry
            .register(HookEvent::PreToolUse, matcher2)
            .unwrap();
        registry
            .register(HookEvent::PreToolUse, matcher3)
            .unwrap();

        // Test matching
        let bash_hooks = registry.get_matching(HookEvent::PreToolUse, "bash");
        assert_eq!(bash_hooks.len(), 2); // bash pattern + wildcard

        let python_hooks = registry.get_matching(HookEvent::PreToolUse, "python");
        assert_eq!(python_hooks.len(), 2); // python pattern + wildcard

        let ruby_hooks = registry.get_matching(HookEvent::PreToolUse, "ruby");
        assert_eq!(ruby_hooks.len(), 1); // only wildcard
    }

    #[test]
    fn test_get_matching_pipe_pattern() {
        let registry = HookRegistry::new();
        let matcher = create_test_matcher(Some("bash|python|node".to_string()));

        registry.register(HookEvent::PreToolUse, matcher).unwrap();

        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "bash").len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "python").len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "node").len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "ruby").len(),
            0
        );
    }

    #[test]
    fn test_get_matching_regex_pattern() {
        let registry = HookRegistry::new();
        let matcher = create_test_matcher(Some("^test_.*".to_string()));

        registry.register(HookEvent::PreToolUse, matcher).unwrap();

        assert_eq!(
            registry
                .get_matching(HookEvent::PreToolUse, "test_function")
                .len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "test_case").len(),
            1
        );
        assert_eq!(
            registry
                .get_matching(HookEvent::PreToolUse, "my_test")
                .len(),
            0
        );
    }

    #[test]
    fn test_has_hooks() {
        let registry = HookRegistry::new();
        assert!(!registry.has_hooks(&HookEvent::PreToolUse));

        let matcher = create_test_matcher(Some("bash".to_string()));
        registry.register(HookEvent::PreToolUse, matcher).unwrap();

        assert!(registry.has_hooks(&HookEvent::PreToolUse));
        assert!(!registry.has_hooks(&HookEvent::PostToolUse));
    }

    #[test]
    fn test_list_events() {
        let registry = HookRegistry::new();
        assert!(registry.list_events().is_empty());

        let matcher1 = create_test_matcher(Some("bash".to_string()));
        let matcher2 = create_test_matcher(Some("python".to_string()));

        registry.register(HookEvent::PreToolUse, matcher1).unwrap();
        registry.register(HookEvent::PostToolUse, matcher2).unwrap();

        let events = registry.list_events();
        assert_eq!(events.len(), 2);
        assert!(events.contains(&HookEvent::PreToolUse));
        assert!(events.contains(&HookEvent::PostToolUse));
    }

    #[test]
    fn test_clear() {
        let registry = HookRegistry::new();
        let matcher = create_test_matcher(Some("bash".to_string()));

        registry.register(HookEvent::PreToolUse, matcher).unwrap();
        assert!(registry.has_hooks(&HookEvent::PreToolUse));

        registry.clear().unwrap();
        assert!(!registry.has_hooks(&HookEvent::PreToolUse));
        assert!(registry.list_events().is_empty());
    }

    #[test]
    fn test_from_config() {
        let config = HooksConfig {
            pre_tool_use: vec![
                create_test_matcher(Some("bash".to_string())),
                create_test_matcher(Some("python".to_string())),
            ],
            post_tool_use: vec![create_test_matcher(None)],
            post_tool_use_failure: vec![],
            user_prompt_submit: vec![],
            session_start: vec![create_test_matcher(Some("cli".to_string()))],
            session_end: vec![],
            subagent_start: vec![],
            subagent_stop: vec![],
            permission_request: vec![],
            pre_compact: vec![],
            notification: vec![],
            stop: vec![],
            status_line: vec![],
        };

        let registry = HookRegistry::from_config(&config);

        // Check pre_tool_use hooks
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "bash").len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "python").len(),
            1
        );

        // Check post_tool_use hooks (wildcard)
        assert_eq!(
            registry.get_matching(HookEvent::PostToolUse, "anything").len(),
            1
        );

        // Check session_start hooks
        assert_eq!(
            registry.get_matching(HookEvent::SessionStart, "cli").len(),
            1
        );

        // Check list_events
        let events = registry.list_events();
        assert!(events.contains(&HookEvent::PreToolUse));
        assert!(events.contains(&HookEvent::PostToolUse));
        assert!(events.contains(&HookEvent::SessionStart));
    }

    #[test]
    fn test_from_config_empty() {
        let config = HooksConfig::default();
        let registry = HookRegistry::from_config(&config);

        assert!(registry.list_events().is_empty());
        assert!(!registry.has_hooks(&HookEvent::PreToolUse));
    }

    #[test]
    fn test_hooks_config_serialization() {
        let config = HooksConfig {
            pre_tool_use: vec![create_test_matcher(Some("bash".to_string()))],
            post_tool_use: vec![],
            post_tool_use_failure: vec![],
            user_prompt_submit: vec![],
            session_start: vec![],
            session_end: vec![],
            subagent_start: vec![],
            subagent_stop: vec![],
            permission_request: vec![],
            pre_compact: vec![],
            notification: vec![],
            stop: vec![],
            status_line: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: HooksConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.pre_tool_use.len(), 1);
        assert_eq!(
            deserialized.pre_tool_use[0].pattern,
            Some("bash".to_string())
        );
    }

    #[test]
    fn test_multiple_events_same_pattern() {
        let registry = HookRegistry::new();
        let matcher1 = create_test_matcher(Some("bash".to_string()));
        let matcher2 = create_test_matcher(Some("bash".to_string()));

        registry.register(HookEvent::PreToolUse, matcher1).unwrap();
        registry.register(HookEvent::PostToolUse, matcher2).unwrap();

        assert_eq!(
            registry.get_matching(HookEvent::PreToolUse, "bash").len(),
            1
        );
        assert_eq!(
            registry.get_matching(HookEvent::PostToolUse, "bash").len(),
            1
        );
    }

    #[test]
    fn test_count() {
        let registry = HookRegistry::new();
        assert_eq!(registry.count(), 0);

        registry
            .register(HookEvent::PreToolUse, create_test_matcher(Some("bash".to_string())))
            .unwrap();
        assert_eq!(registry.count(), 1);

        registry
            .register(HookEvent::PreToolUse, create_test_matcher(Some("python".to_string())))
            .unwrap();
        assert_eq!(registry.count(), 2);

        registry
            .register(HookEvent::PostToolUse, create_test_matcher(None))
            .unwrap();
        assert_eq!(registry.count(), 3);
    }
}
