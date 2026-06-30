//! Parent context fork policy for sub-agent spawns.

use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmMessage;
use crate::types::MessageRole;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Parent conversation message available to a child agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ForkContextMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ForkContextMessage {
    pub fn new(role: MessageRole, content: impl Into<String>) -> SageResult<Self> {
        let content = content.into();
        if content.trim().is_empty() {
            return Err(SageError::config("fork context message content is empty"));
        }
        Ok(Self { role, content })
    }

    pub fn to_llm_message(&self) -> Option<LlmMessage> {
        match self.role {
            MessageRole::User => Some(LlmMessage::user(&self.content)),
            MessageRole::Assistant => Some(LlmMessage::assistant(&self.content)),
            MessageRole::System | MessageRole::Tool | MessageRole::Error => None,
        }
    }
}

/// Controls how much parent context a child agent receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkContextPolicy {
    None,
    All,
    LastN { turns: usize },
}

impl Default for ForkContextPolicy {
    fn default() -> Self {
        Self::None
    }
}

impl ForkContextPolicy {
    pub fn label(&self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::All => "all".to_string(),
            Self::LastN { turns } => format!("last_n:{turns}"),
        }
    }

    pub fn validate(&self) -> SageResult<()> {
        if matches!(self, Self::LastN { turns: 0 }) {
            return Err(SageError::config(
                "fork_context last_n requires turns greater than zero",
            ));
        }
        Ok(())
    }

    pub fn select_messages(
        &self,
        parent_context: &[ForkContextMessage],
    ) -> SageResult<Vec<LlmMessage>> {
        self.validate()?;
        let selected = match self {
            Self::None => Vec::new(),
            Self::All => parent_context
                .iter()
                .filter_map(ForkContextMessage::to_llm_message)
                .collect(),
            Self::LastN { turns } => parent_context[last_n_start(parent_context, *turns)..]
                .iter()
                .filter_map(ForkContextMessage::to_llm_message)
                .collect(),
        };
        Ok(selected)
    }
}

impl Serialize for ForkContextPolicy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::None => serializer.serialize_str("none"),
            Self::All => serializer.serialize_str("all"),
            Self::LastN { turns } => {
                let mut state = serializer.serialize_struct("ForkContextPolicy", 2)?;
                state.serialize_field("mode", "last_n")?;
                state.serialize_field("turns", turns)?;
                state.end()
            }
        }
    }
}

fn last_n_start(parent_context: &[ForkContextMessage], turns: usize) -> usize {
    let mut seen_user_turns = 0usize;
    for (index, message) in parent_context.iter().enumerate().rev() {
        if message.role == MessageRole::User {
            seen_user_turns += 1;
            if seen_user_turns == turns {
                return index;
            }
        }
    }

    if seen_user_turns == 0 {
        parent_context.len().saturating_sub(turns)
    } else {
        0
    }
}

impl<'de> Deserialize<'de> for ForkContextPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        parse_fork_context_value(value).map_err(|err| serde::de::Error::custom(err.to_string()))
    }
}

fn parse_fork_context_value(value: serde_json::Value) -> SageResult<ForkContextPolicy> {
    match value {
        serde_json::Value::String(raw) => parse_fork_context_mode(&raw, None),
        serde_json::Value::Object(mut object) => {
            let mode = object
                .remove("mode")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .ok_or_else(|| {
                    SageError::config("fork_context object requires string field 'mode'")
                })?;
            let turns = object.remove("turns").and_then(|value| value.as_u64());
            if !object.is_empty() {
                return Err(SageError::config("fork_context contains unknown fields"));
            }
            parse_fork_context_mode(&mode, turns)
        }
        _ => Err(SageError::config("fork_context must be a string or object")),
    }
}

fn parse_fork_context_mode(mode: &str, turns: Option<u64>) -> SageResult<ForkContextPolicy> {
    match mode {
        "none" => Ok(ForkContextPolicy::None),
        "all" => Ok(ForkContextPolicy::All),
        "last_n" | "last-n" => {
            let turns =
                turns.ok_or_else(|| SageError::config("last_n fork_context requires turns"))?;
            let turns = usize::try_from(turns)
                .map_err(|_| SageError::config("last_n fork_context turns is too large"))?;
            if turns == 0 {
                return Err(SageError::config(
                    "last_n fork_context requires turns greater than zero",
                ));
            }
            Ok(ForkContextPolicy::LastN { turns })
        }
        other => Err(SageError::config(format!(
            "unsupported fork_context mode '{other}'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: MessageRole, content: &str) -> ForkContextMessage {
        ForkContextMessage::new(role, content).expect("valid message")
    }

    #[test]
    fn subagent_fork_context_none_selects_no_messages() {
        let parent = vec![msg(MessageRole::User, "one")];
        let selected = ForkContextPolicy::None
            .select_messages(&parent)
            .expect("select");
        assert!(selected.is_empty());
    }

    #[test]
    fn subagent_fork_context_all_selects_every_message() {
        let parent = vec![
            msg(MessageRole::User, "one"),
            msg(MessageRole::System, "parent system"),
            msg(MessageRole::Assistant, "two"),
            msg(MessageRole::Tool, "tool result"),
            msg(MessageRole::Error, "error result"),
        ];
        let selected = ForkContextPolicy::All
            .select_messages(&parent)
            .expect("select");
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].content, "one");
        assert_eq!(selected[1].content, "two");
    }

    #[test]
    fn subagent_fork_context_last_n_keeps_recent_user_turns() {
        let parent = vec![
            msg(MessageRole::User, "u1"),
            msg(MessageRole::Assistant, "a1"),
            msg(MessageRole::User, "u2"),
            msg(MessageRole::Assistant, "a2"),
            msg(MessageRole::User, "u3"),
        ];
        let selected = ForkContextPolicy::LastN { turns: 2 }
            .select_messages(&parent)
            .expect("select");
        let contents = selected
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(contents, vec!["u2", "a2", "u3"]);
    }

    #[test]
    fn subagent_fork_context_last_n_zero_fails_closed() {
        let error = serde_json::from_value::<ForkContextPolicy>(serde_json::json!({
            "mode": "last_n",
            "turns": 0
        }))
        .expect_err("last_n zero must fail");
        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    fn subagent_fork_context_round_trips_last_n() {
        let policy = ForkContextPolicy::LastN { turns: 3 };
        let value = serde_json::to_value(&policy).expect("serialize");
        assert_eq!(value, serde_json::json!({"mode": "last_n", "turns": 3}));
        let parsed: ForkContextPolicy = serde_json::from_value(value).expect("deserialize");
        assert_eq!(parsed, policy);
    }
}
