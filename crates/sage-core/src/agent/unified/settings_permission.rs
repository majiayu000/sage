//! Settings-backed permission checks for unified tool execution.

use crate::error::SageResult;
use crate::input::{InputRequest, InputResponseKind};
use crate::settings::SettingsLoader;
use crate::settings::locations::SettingsLocations;
use crate::settings::types::{Settings, SettingsPermissionBehavior};
use crate::settings::validation::SettingsValidator;
use crate::tools::permission::PermissionCache;
use crate::tools::types::{ToolCall, ToolResult};

use super::UnifiedExecutor;
use super::tool_orchestrator::ToolExecutionContext;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingsPermissionDecision {
    Allow,
    Deny(String),
    Ask(String),
}

pub(in crate::agent::unified) enum SettingsPermissionCheck {
    Allowed(ToolCall),
    Blocked(ToolResult),
}

impl UnifiedExecutor {
    pub(in crate::agent::unified) async fn check_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
    ) -> SageResult<Option<SettingsPermissionCheck>> {
        let settings = Self::load_settings_strict(&context.working_dir)?;
        let Some(decision) = Self::settings_permission_decision(&settings, tool_call) else {
            return Ok(None);
        };

        match decision {
            SettingsPermissionDecision::Allow => {
                Ok(Some(SettingsPermissionCheck::Allowed(tool_call.clone())))
            }
            SettingsPermissionDecision::Deny(reason) => {
                Ok(Some(SettingsPermissionCheck::Blocked(ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!("Permission denied by settings: {}", reason),
                ))))
            }
            SettingsPermissionDecision::Ask(reason) => self
                .request_settings_permission(tool_call, reason)
                .await
                .map(Some),
        }
    }

    async fn request_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        reason: String,
    ) -> SageResult<SettingsPermissionCheck> {
        self.event_manager.stop_animation().await;
        let input = serde_json::to_value(&tool_call.arguments).unwrap_or(serde_json::Value::Null);
        let request = InputRequest::permission(
            &tool_call.name,
            format!(
                "Tool '{}' requires permission from settings.\n{}",
                tool_call.name, reason
            ),
            input,
        );

        let response = match self.request_user_input(request).await {
            Ok(response) => response,
            Err(err) => {
                return Ok(SettingsPermissionCheck::Blocked(ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!("Permission request failed: {}", err),
                )));
            }
        };

        match response.kind {
            InputResponseKind::PermissionGranted { modified_input, .. } => {
                let mut approved_call = tool_call.clone();
                if let Some(serde_json::Value::Object(map)) = modified_input {
                    approved_call.arguments = map.into_iter().collect();
                }

                Ok(SettingsPermissionCheck::Allowed(approved_call))
            }
            InputResponseKind::PermissionDenied { reason } => {
                let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
                Ok(SettingsPermissionCheck::Blocked(ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    format!("Permission denied by user: {}", reason),
                )))
            }
            InputResponseKind::Cancelled => {
                Ok(SettingsPermissionCheck::Blocked(ToolResult::error(
                    &tool_call.id,
                    &tool_call.name,
                    "Permission request cancelled by user.",
                )))
            }
            _ => Ok(SettingsPermissionCheck::Blocked(ToolResult::error(
                &tool_call.id,
                &tool_call.name,
                "Invalid permission response from input handler.",
            ))),
        }
    }

    fn load_settings_strict(working_dir: &std::path::Path) -> SageResult<Settings> {
        let locations = SettingsLocations::discover_from(working_dir);
        let loader = SettingsLoader::from_directory(working_dir);
        let mut settings = Settings::default();

        if locations.user.exists() {
            settings.merge(loader.load_from_file(&locations.user)?);
        }
        if let Some(project) = locations.project.as_ref() {
            settings.merge(loader.load_from_file(project)?);
        }
        if let Some(local) = locations.local.as_ref() {
            settings.merge(loader.load_from_file(local)?);
        }

        settings.apply_env_overrides();
        SettingsValidator::new().validate(&settings)?;
        Ok(settings)
    }

    fn settings_permission_decision(
        settings: &Settings,
        tool_call: &ToolCall,
    ) -> Option<SettingsPermissionDecision> {
        let permissions = &settings.permissions;
        let has_configured_permissions = !permissions.allow.is_empty()
            || !permissions.deny.is_empty()
            || permissions.default_behavior != SettingsPermissionBehavior::Ask;
        if !has_configured_permissions {
            return None;
        }

        let key = Self::actual_permission_key(
            &Self::canonical_permission_tool_name(&tool_call.name),
            tool_call,
        );

        if let Some(pattern) = permissions
            .deny
            .iter()
            .find(|pattern| PermissionCache::pattern_matches(pattern, &key))
        {
            return Some(SettingsPermissionDecision::Deny(format!(
                "matched deny rule '{}'",
                pattern
            )));
        }

        if permissions
            .allow
            .iter()
            .any(|pattern| PermissionCache::pattern_matches(pattern, &key))
        {
            return Some(SettingsPermissionDecision::Allow);
        }

        match permissions.default_behavior {
            SettingsPermissionBehavior::Allow => Some(SettingsPermissionDecision::Allow),
            SettingsPermissionBehavior::Deny => Some(SettingsPermissionDecision::Deny(format!(
                "no allow rule matched '{}'",
                key
            ))),
            SettingsPermissionBehavior::Ask => Some(SettingsPermissionDecision::Ask(format!(
                "No permission rule matched '{}'.",
                key
            ))),
        }
    }

    fn canonical_permission_tool_name(tool_name: &str) -> String {
        match tool_name.to_lowercase().as_str() {
            "bash" => "Bash".to_string(),
            "read" => "Read".to_string(),
            "write" => "Write".to_string(),
            "edit" => "Edit".to_string(),
            "multiedit" | "multi_edit" => "MultiEdit".to_string(),
            "glob" => "Glob".to_string(),
            "grep" => "Grep".to_string(),
            "task" => "Task".to_string(),
            "webfetch" | "web_fetch" => "WebFetch".to_string(),
            "websearch" | "web_search" => "WebSearch".to_string(),
            "todowrite" | "todo_write" => "TodoWrite".to_string(),
            "askuserquestion" | "ask_user_question" => "AskUserQuestion".to_string(),
            "notebookedit" | "notebook_edit" => "NotebookEdit".to_string(),
            _ => tool_name.to_string(),
        }
    }

    fn actual_permission_key(tool_name: &str, call: &ToolCall) -> String {
        let argument = match tool_name.to_lowercase().as_str() {
            "bash" => call
                .arguments
                .get("command")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            "read" | "write" | "edit" | "multiedit" => call
                .arguments
                .get("file_path")
                .or_else(|| call.arguments.get("path"))
                .and_then(|value| value.as_str())
                .map(|path| path.trim_start_matches('/').to_string()),
            _ => None,
        };

        match argument {
            Some(argument) => format!("{}({})", tool_name, argument),
            None => tool_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::types::PermissionSettings;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn bash_call(command: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert(
            "command".to_string(),
            serde_json::Value::String(command.to_string()),
        );
        ToolCall::new("call-1", "bash", arguments)
    }

    #[test]
    fn test_settings_permission_denies_matching_rule() {
        let settings = Settings {
            permissions: PermissionSettings {
                deny: vec!["Bash(echo *)".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("echo blocked"));

        assert!(matches!(
            decision,
            Some(SettingsPermissionDecision::Deny(_))
        ));
    }

    #[test]
    fn test_settings_permission_matches_specific_command_against_actual_input() {
        let settings = Settings {
            permissions: PermissionSettings {
                deny: vec!["Bash(rm -rf *)".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("rm -rf /tmp/foo"));

        assert!(matches!(
            decision,
            Some(SettingsPermissionDecision::Deny(_))
        ));
    }

    #[test]
    fn test_settings_permission_wildcard_deny_matches_any_tool() {
        let settings = Settings {
            permissions: PermissionSettings {
                deny: vec!["*".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("echo blocked"));

        assert!(matches!(
            decision,
            Some(SettingsPermissionDecision::Deny(_))
        ));
    }

    #[test]
    fn test_settings_permission_allows_matching_rule() {
        let settings = Settings {
            permissions: PermissionSettings {
                allow: vec!["Bash(echo *)".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("echo allowed"));

        assert_eq!(decision, Some(SettingsPermissionDecision::Allow));
    }

    #[test]
    fn test_settings_permission_deny_precedes_allow() {
        let settings = Settings {
            permissions: PermissionSettings {
                allow: vec!["Bash".to_string()],
                deny: vec!["Bash(echo *)".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("echo blocked"));

        assert!(matches!(
            decision,
            Some(SettingsPermissionDecision::Deny(_))
        ));
    }

    #[test]
    fn test_settings_permission_default_deny_blocks_unmatched_call() {
        let settings = Settings {
            permissions: PermissionSettings {
                default_behavior: SettingsPermissionBehavior::Deny,
                ..Default::default()
            },
            ..Default::default()
        };

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("cargo test"));

        assert!(matches!(
            decision,
            Some(SettingsPermissionDecision::Deny(_))
        ));
    }

    #[test]
    fn test_empty_default_settings_do_not_force_permission_prompt() {
        let settings = Settings::default();

        let decision =
            UnifiedExecutor::settings_permission_decision(&settings, &bash_call("cargo test"));

        assert_eq!(decision, None);
    }

    #[test]
    fn test_load_settings_strict_rejects_invalid_project_settings() -> SageResult<()> {
        let temp_dir = TempDir::new()?;
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir)?;
        fs::write(sage_dir.join("settings.local.json"), "{ invalid json")?;

        let result = UnifiedExecutor::load_settings_strict(temp_dir.path());

        assert!(result.is_err());
        Ok(())
    }
}
