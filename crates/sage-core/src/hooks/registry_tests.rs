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

    registry.register(HookEvent::PreToolUse, matcher1).unwrap();
    registry.register(HookEvent::PreToolUse, matcher2).unwrap();
    registry.register(HookEvent::PreToolUse, matcher3).unwrap();

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
        registry
            .get_matching(HookEvent::PreToolUse, "test_case")
            .len(),
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
        registry
            .get_matching(HookEvent::PostToolUse, "anything")
            .len(),
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
        .register(
            HookEvent::PreToolUse,
            create_test_matcher(Some("bash".to_string())),
        )
        .unwrap();
    assert_eq!(registry.count(), 1);

    registry
        .register(
            HookEvent::PreToolUse,
            create_test_matcher(Some("python".to_string())),
        )
        .unwrap();
    assert_eq!(registry.count(), 2);

    registry
        .register(HookEvent::PostToolUse, create_test_matcher(None))
        .unwrap();
    assert_eq!(registry.count(), 3);
}
