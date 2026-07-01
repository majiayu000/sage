use super::*;
use crate::permissions::{PermissionBehavior, PermissionProfileSource};

#[test]
fn network_target_is_required_even_when_keys_are_supplied() {
    let profile = PermissionProfile::default().with_default_behavior(PermissionBehavior::Allow);
    let decision = PermissionDecisionEngine::new(profile).decide(PermissionDecisionInput::new(
        PermissionAction::Network,
        "WebFetch",
        vec!["WebFetch(https://internal.example/private)".to_string()],
    ));

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("require a request target"));
}

#[test]
fn blank_network_target_is_rejected() {
    let profile = PermissionProfile::default().with_default_behavior(PermissionBehavior::Allow);
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Network,
            "WebFetch",
            vec!["WebFetch(   )".to_string()],
        )
        .with_network_target("   "),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert!(decision.reason.contains("require a request target"));
}

#[test]
fn supplied_network_keys_are_matched_with_normalized_target_aliases() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(https://internal.example/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(
            PermissionAction::Network,
            "WebFetch",
            vec!["WebFetch(https://user:password@internal.example/private)".to_string()],
        )
        .with_network_target("https://user:password@internal.example/private"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("WebFetch(https://internal.example/**)")
    );
}

#[test]
fn network_target_trims_dns_root_dot_before_matching_rules() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(http://internal.example/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("http://internal.example./private"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision.audit_key,
        "WebFetch(http://internal.example/private)"
    );
}

#[test]
fn url_rule_patterns_are_normalized_before_matching() {
    let profile = PermissionProfile::default()
        .with_default_behavior(PermissionBehavior::Allow)
        .add_deny(
            "WebFetch(HTTPS://INTERNAL.EXAMPLE:443/**)",
            PermissionProfileSource::Project,
        );
    let decision = PermissionDecisionEngine::new(profile).decide(
        PermissionDecisionInput::new(PermissionAction::Network, "WebFetch", Vec::new())
            .with_network_target("https://internal.example/private"),
    );

    assert_eq!(decision.kind, PermissionDecisionKind::Deny);
    assert_eq!(
        decision
            .matched_rule
            .as_ref()
            .map(|rule| rule.pattern.as_str()),
        Some("WebFetch(HTTPS://INTERNAL.EXAMPLE:443/**)")
    );
}
