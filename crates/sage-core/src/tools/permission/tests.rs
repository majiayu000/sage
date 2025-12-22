//! Tests for the permission system

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::tools::permission::*;
    use crate::tools::types::ToolCall;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_risk_level_confirmation() {
        assert!(!RiskLevel::Low.requires_confirmation());
        assert!(!RiskLevel::Medium.requires_confirmation());
        assert!(RiskLevel::High.requires_confirmation());
        assert!(RiskLevel::Critical.requires_confirmation());
    }

    #[test]
    fn test_permission_result() {
        assert!(PermissionResult::allow().is_allowed());
        assert!(!PermissionResult::deny("test").is_allowed());
    }

    #[test]
    fn test_tool_context_path_checking() {
        let ctx = ToolContext::new(PathBuf::from("/home/user"))
            .allow_path("/home/user/projects")
            .deny_path("/home/user/.ssh");

        assert!(ctx.is_path_allowed(std::path::Path::new("/home/user/projects/code.rs")));
        assert!(!ctx.is_path_allowed(std::path::Path::new("/home/user/.ssh/id_rsa")));
    }

    #[tokio::test]
    async fn test_auto_allow_handler() {
        let handler = AutoAllowHandler;
        let call = ToolCall::new("1", "test_tool", HashMap::new());
        let request = PermissionRequest::new("test_tool", call, "test", RiskLevel::High);

        let decision = handler.handle_permission_request(request).await;
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_policy_handler() {
        let policy = PermissionPolicy {
            allow_by_default: true,
            max_auto_allow_risk: RiskLevel::Medium,
            tool_overrides: HashMap::new(),
        };

        let handler = PolicyHandler::new(policy);

        // Low risk should be allowed
        let call = ToolCall::new("1", "read_file", HashMap::new());
        let request = PermissionRequest::new("read_file", call, "test", RiskLevel::Low);
        assert!(
            handler
                .handle_permission_request(request)
                .await
                .is_allowed()
        );

        // High risk should also be allowed due to allow_by_default
        let call = ToolCall::new("2", "delete_file", HashMap::new());
        let request = PermissionRequest::new("delete_file", call, "test", RiskLevel::High);
        assert!(
            handler
                .handle_permission_request(request)
                .await
                .is_allowed()
        );
    }

    #[tokio::test]
    async fn test_permission_cache() {
        let cache = PermissionCache::new();

        let call = ToolCall::new("1", "test", HashMap::new());
        let key = PermissionCache::cache_key("test", &call);

        assert!(cache.get(&key).await.is_none());

        cache.set(key.clone(), true).await;
        assert_eq!(cache.get(&key).await, Some(true));

        cache.clear().await;
        assert!(cache.get(&key).await.is_none());
    }

    // ===== Rule-based Permission Tests =====

    #[test]
    fn test_rule_source_priority() {
        assert!(RuleSource::CliArg.priority() < RuleSource::SessionSettings.priority());
        assert!(RuleSource::SessionSettings.priority() < RuleSource::LocalSettings.priority());
        assert!(RuleSource::LocalSettings.priority() < RuleSource::ProjectSettings.priority());
        assert!(RuleSource::ProjectSettings.priority() < RuleSource::UserSettings.priority());
        assert!(RuleSource::UserSettings.priority() < RuleSource::Builtin.priority());
    }

    #[test]
    fn test_rule_source_display() {
        assert_eq!(format!("{}", RuleSource::CliArg), "command line");
        assert_eq!(
            format!("{}", RuleSource::ProjectSettings),
            "project settings"
        );
    }

    #[test]
    fn test_permission_behavior_display() {
        assert_eq!(format!("{}", PermissionBehavior::Allow), "allow");
        assert_eq!(format!("{}", PermissionBehavior::Deny), "deny");
        assert_eq!(format!("{}", PermissionBehavior::Ask), "ask");
        assert_eq!(
            format!("{}", PermissionBehavior::Passthrough),
            "passthrough"
        );
    }

    #[test]
    fn test_permission_rule_matches_tool() {
        let rule = PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("bash");

        assert!(rule.matches("bash", None, None));
        assert!(!rule.matches("edit", None, None));
    }

    #[test]
    fn test_permission_rule_matches_tool_pattern() {
        let rule =
            PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("edit|write|read");

        assert!(rule.matches("edit", None, None));
        assert!(rule.matches("write", None, None));
        assert!(rule.matches("read", None, None));
        assert!(!rule.matches("bash", None, None));
    }

    #[test]
    fn test_permission_rule_matches_path() {
        let rule = PermissionRule::new(PermissionBehavior::Deny)
            .with_tool_pattern("edit|write")
            .with_path_pattern(".*\\.env.*");

        // Should match .env files
        assert!(rule.matches("edit", Some("/path/to/.env"), None));
        assert!(rule.matches("write", Some("/path/to/.env.local"), None));

        // Should not match regular files
        assert!(!rule.matches("edit", Some("/path/to/code.rs"), None));

        // Should not match if no path provided when path pattern exists
        assert!(!rule.matches("edit", None, None));
    }

    #[test]
    fn test_permission_rule_matches_command() {
        let rule = PermissionRule::new(PermissionBehavior::Deny)
            .with_tool_pattern("bash")
            .with_command_pattern(".*rm.*-rf.*");

        // Should match dangerous commands
        assert!(rule.matches("bash", None, Some("rm -rf /")));
        assert!(rule.matches("bash", None, Some("sudo rm -rf /tmp")));

        // Should not match safe commands
        assert!(!rule.matches("bash", None, Some("ls -la")));
        assert!(!rule.matches("bash", None, Some("cat file.txt")));
    }

    #[test]
    fn test_permission_rule_disabled() {
        let mut rule = PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("bash");
        rule.enabled = false;

        assert!(!rule.matches("bash", None, None));
    }

    #[test]
    fn test_permission_rule_engine_evaluate() {
        let mut engine = PermissionRuleEngine::new();

        // Add rules
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Deny)
                .with_tool_pattern("bash")
                .with_command_pattern(".*rm.*-rf.*")
                .with_reason("Dangerous command"),
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("read|glob|grep"),
        );

        // Dangerous bash command should be denied
        let eval = engine.evaluate("bash", None, Some("rm -rf /"));
        assert_eq!(eval.behavior, PermissionBehavior::Deny);
        assert!(eval.matched_rule.is_some());

        // Safe read should be allowed
        let eval = engine.evaluate("read", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Allow);

        // Unknown tool should get default behavior
        let eval = engine.evaluate("unknown_tool", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Ask); // Default
    }

    #[test]
    fn test_permission_rule_engine_sort_by_priority() {
        let mut engine = PermissionRuleEngine::new();

        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow)
                .with_source(RuleSource::Builtin)
                .with_tool_pattern("bash"),
        );
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Deny)
                .with_source(RuleSource::CliArg)
                .with_tool_pattern("bash"),
        );

        engine.sort_by_priority();

        // CLI arg should be first (higher priority)
        assert_eq!(engine.rules()[0].source, RuleSource::CliArg);
        assert_eq!(engine.rules()[1].source, RuleSource::Builtin);

        // CLI arg rule should be matched first
        let eval = engine.evaluate("bash", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Deny);
    }

    #[test]
    fn test_permission_rule_engine_passthrough() {
        let mut engine = PermissionRuleEngine::new();

        // Passthrough rule should be skipped
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Passthrough).with_tool_pattern("bash"),
        );
        engine.add_rule(PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("bash"));

        let eval = engine.evaluate("bash", None, None);
        assert_eq!(eval.behavior, PermissionBehavior::Allow);
    }

    #[test]
    fn test_permission_evaluation_to_result() {
        let eval = PermissionEvaluation {
            behavior: PermissionBehavior::Allow,
            matched_rule: None,
            reason: None,
        };
        let result = eval.to_result(RiskLevel::Low);
        assert!(matches!(result, PermissionResult::Allow));

        let eval = PermissionEvaluation {
            behavior: PermissionBehavior::Deny,
            matched_rule: None,
            reason: Some("Test reason".to_string()),
        };
        let result = eval.to_result(RiskLevel::High);
        assert!(matches!(result, PermissionResult::Deny { reason } if reason == "Test reason"));
    }

    #[tokio::test]
    async fn test_rule_based_handler() {
        let mut engine = PermissionRuleEngine::new();
        engine.add_rule(
            PermissionRule::new(PermissionBehavior::Allow).with_tool_pattern("read|glob"),
        );
        engine.add_rule(PermissionRule::new(PermissionBehavior::Deny).with_tool_pattern("bash"));

        let handler = RuleBasedHandler::new(engine);

        // Read should be allowed
        let call = ToolCall::new("1", "read", HashMap::new());
        let request = PermissionRequest::new("read", call, "test", RiskLevel::Low);
        let decision = handler.handle_permission_request(request).await;
        assert_eq!(decision, PermissionDecision::Allow);

        // Bash should be denied
        let call = ToolCall::new("2", "bash", HashMap::new());
        let request = PermissionRequest::new("bash", call, "test", RiskLevel::Medium);
        let decision = handler.handle_permission_request(request).await;
        assert_eq!(decision, PermissionDecision::Deny);
    }

    #[test]
    fn test_permission_rules_config_serialization() {
        let config = PermissionRulesConfig {
            rules: vec![
                PermissionRule::new(PermissionBehavior::Allow)
                    .with_tool_pattern("read|glob|grep")
                    .with_source(RuleSource::Builtin),
                PermissionRule::new(PermissionBehavior::Deny)
                    .with_tool_pattern("bash")
                    .with_command_pattern(".*rm.*-rf.*")
                    .with_reason("Dangerous command"),
            ],
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: PermissionRulesConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.rules.len(), 2);
        assert_eq!(deserialized.rules[0].behavior, PermissionBehavior::Allow);
        assert_eq!(deserialized.rules[1].behavior, PermissionBehavior::Deny);
    }

    #[test]
    fn test_permission_rule_display() {
        let rule = PermissionRule::new(PermissionBehavior::Allow)
            .with_tool_pattern("bash")
            .with_command_pattern("ls.*")
            .with_source(RuleSource::ProjectSettings);

        let display = format!("{}", rule);
        assert!(display.contains("allow"));
        assert!(display.contains("project settings"));
        assert!(display.contains("tool=bash"));
        assert!(display.contains("cmd=ls.*"));
    }
}
