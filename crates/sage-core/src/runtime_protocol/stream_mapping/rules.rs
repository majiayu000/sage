use serde_json::{Map, Value};

use crate::input::{PermissionSuggestion, RuleDestination};
use crate::tools::permission::PermissionBehavior;

use crate::runtime_protocol::permission::{RuntimeRule, RuntimeRuleBehavior, RuntimeRuleSource};

pub(super) fn object_value(value: Value) -> Value {
    if value.is_object() {
        value
    } else {
        let mut map = Map::new();
        map.insert("value".to_string(), value);
        Value::Object(map)
    }
}

pub(super) fn rule_from_suggestion(suggestion: &PermissionSuggestion) -> RuntimeRule {
    RuntimeRule {
        behavior: match suggestion.behavior {
            PermissionBehavior::Allow => RuntimeRuleBehavior::Allow,
            PermissionBehavior::Deny => RuntimeRuleBehavior::Deny,
            PermissionBehavior::Ask => RuntimeRuleBehavior::Ask,
            PermissionBehavior::Passthrough => RuntimeRuleBehavior::Passthrough,
        },
        source: match suggestion.destination {
            RuleDestination::Session => RuntimeRuleSource::SessionSettings,
            RuleDestination::LocalSettings => RuntimeRuleSource::LocalSettings,
            RuleDestination::UserSettings => RuntimeRuleSource::UserSettings,
            RuleDestination::ProjectSettings => RuntimeRuleSource::ProjectSettings,
        },
    }
}
