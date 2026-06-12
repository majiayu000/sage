//! Settings-backed permission checks for unified tool execution.

use crate::error::SageResult;
use crate::input::{InputRequest, InputResponseKind};
use crate::settings::SettingsLoader;
use crate::settings::locations::SettingsLocations;
use crate::settings::types::{Settings, SettingsPermissionBehavior};
use crate::settings::validation::SettingsValidator;
use crate::tools::permission::PermissionCache;
use crate::tools::types::{ToolCall, ToolResult};
use std::path::{Path, PathBuf};

use super::UnifiedExecutor;
use super::tool_orchestrator::ToolExecutionContext;

#[path = "settings_permission_paths.rs"]
mod settings_permission_paths;

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

enum SettingsPermissionPromptResult {
    Allowed {
        tool_call: ToolCall,
        input_modified: bool,
    },
    Blocked(ToolResult),
}

impl UnifiedExecutor {
    pub(in crate::agent::unified) async fn check_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
    ) -> SageResult<Option<SettingsPermissionCheck>> {
        let settings = Self::load_settings_strict(&context.working_dir)?;
        let mut current_call = tool_call.clone();
        let mut prompted_count = 0usize;

        loop {
            let Some(decision) =
                Self::settings_permission_decision(&settings, &current_call, &context.working_dir)
            else {
                return if current_call == *tool_call {
                    Ok(None)
                } else {
                    Ok(Some(SettingsPermissionCheck::Allowed(current_call)))
                };
            };

            match decision {
                SettingsPermissionDecision::Allow => {
                    return Ok(Some(SettingsPermissionCheck::Allowed(current_call)));
                }
                SettingsPermissionDecision::Deny(reason) => {
                    return Ok(Some(SettingsPermissionCheck::Blocked(
                        Self::settings_permission_blocked_result(
                            &current_call,
                            format!("Permission denied by settings: {}", reason),
                        ),
                    )));
                }
                SettingsPermissionDecision::Ask(reason) => {
                    prompted_count += 1;
                    if prompted_count > 8 {
                        return Ok(Some(SettingsPermissionCheck::Blocked(
                            Self::settings_permission_blocked_result(
                                &current_call,
                                "Permission request exceeded the maximum number of edited approvals.",
                            ),
                        )));
                    }

                    match self
                        .request_settings_permission(&current_call, reason)
                        .await?
                    {
                        SettingsPermissionPromptResult::Allowed {
                            tool_call: approved_call,
                            input_modified,
                        } => {
                            if input_modified && approved_call != current_call {
                                current_call = approved_call;
                                continue;
                            }

                            return Ok(Some(SettingsPermissionCheck::Allowed(approved_call)));
                        }
                        SettingsPermissionPromptResult::Blocked(result) => {
                            return Ok(Some(SettingsPermissionCheck::Blocked(result)));
                        }
                    }
                }
            }
        }
    }

    pub(in crate::agent) fn unattended_settings_permission_result(
        tool_call: &ToolCall,
        working_dir: &Path,
    ) -> SageResult<Option<ToolResult>> {
        let settings = Self::load_settings_strict(working_dir)?;
        let Some(decision) = Self::settings_permission_decision(&settings, tool_call, working_dir)
        else {
            return Ok(None);
        };

        match decision {
            SettingsPermissionDecision::Allow => Ok(None),
            SettingsPermissionDecision::Deny(reason) => {
                Ok(Some(Self::settings_permission_blocked_result(
                    tool_call,
                    format!("Permission denied by settings: {}", reason),
                )))
            }
            SettingsPermissionDecision::Ask(reason) => {
                Ok(Some(Self::settings_permission_blocked_result(
                    tool_call,
                    format!(
                        "Permission required by settings but sub-agent tool calls cannot prompt for approval: {}",
                        reason
                    ),
                )))
            }
        }
    }

    async fn request_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        reason: String,
    ) -> SageResult<SettingsPermissionPromptResult> {
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
                return Ok(SettingsPermissionPromptResult::Blocked(
                    Self::settings_permission_blocked_result(
                        tool_call,
                        format!("Permission request failed: {}", err),
                    ),
                ));
            }
        };

        match response.kind {
            InputResponseKind::PermissionGranted { modified_input, .. } => {
                let mut approved_call = tool_call.clone();
                let input_modified = modified_input.is_some();
                if let Some(serde_json::Value::Object(map)) = modified_input {
                    approved_call.arguments = map
                        .into_iter()
                        .filter(|(key, _)| !Self::is_confirmation_only_argument(key))
                        .collect();
                }

                Ok(SettingsPermissionPromptResult::Allowed {
                    tool_call: approved_call,
                    input_modified,
                })
            }
            InputResponseKind::PermissionDenied { reason } => {
                let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
                Ok(SettingsPermissionPromptResult::Blocked(
                    Self::settings_permission_blocked_result(
                        tool_call,
                        format!("Permission denied by user: {}", reason),
                    ),
                ))
            }
            InputResponseKind::Cancelled => Ok(SettingsPermissionPromptResult::Blocked(
                Self::settings_permission_blocked_result(
                    tool_call,
                    "Permission request cancelled by user.",
                ),
            )),
            InputResponseKind::FreeText { text }
            | InputResponseKind::Simple { content: text, .. } => {
                match Self::legacy_permission_text_decision(&text) {
                    Some(true) => Ok(SettingsPermissionPromptResult::Allowed {
                        tool_call: tool_call.clone(),
                        input_modified: false,
                    }),
                    Some(false) => Ok(SettingsPermissionPromptResult::Blocked(
                        Self::settings_permission_blocked_result(
                            tool_call,
                            format!("Permission denied by user response: {}", text),
                        ),
                    )),
                    None => Ok(SettingsPermissionPromptResult::Blocked(
                        Self::settings_permission_blocked_result(
                            tool_call,
                            "Invalid permission response from input handler.",
                        ),
                    )),
                }
            }
            _ => Ok(SettingsPermissionPromptResult::Blocked(
                Self::settings_permission_blocked_result(
                    tool_call,
                    "Invalid permission response from input handler.",
                ),
            )),
        }
    }

    fn settings_permission_blocked_result(
        tool_call: &ToolCall,
        message: impl Into<String>,
    ) -> ToolResult {
        ToolResult::error(&tool_call.id, &tool_call.name, message.into())
    }

    fn load_settings_strict(working_dir: &Path) -> SageResult<Settings> {
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
        working_dir: &Path,
    ) -> Option<SettingsPermissionDecision> {
        let permissions = &settings.permissions;
        let has_configured_permissions = !permissions.allow.is_empty()
            || !permissions.deny.is_empty()
            || permissions.default_behavior_set
            || permissions.default_behavior != SettingsPermissionBehavior::Ask;
        if !has_configured_permissions {
            return None;
        }

        let tool_name = Self::canonical_permission_tool_name(&tool_call.name);
        let key = Self::actual_permission_key(&tool_name, tool_call, working_dir);

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

        if tool_name == "Grep" && matches!(key.as_str(), "Grep" | "Grep()") {
            if let Some(pattern) = permissions.deny.iter().find(|pattern| {
                let lower = pattern.to_ascii_lowercase();
                lower == "grep" || lower.starts_with("grep(")
            }) {
                return Some(SettingsPermissionDecision::Deny(format!(
                    "workspace-wide Grep search overlaps deny rule '{}'",
                    pattern
                )));
            }
        }

        if tool_name == "Grep" {
            if let Some(pattern) = permissions.deny.iter().find(|pattern| {
                settings_permission_paths::grep_search_overlaps_deny_rule(&key, pattern)
            }) {
                return Some(SettingsPermissionDecision::Deny(format!(
                    "Grep search overlaps deny rule '{}'",
                    pattern
                )));
            }
        }

        if tool_name == "Glob" {
            if let Some(pattern) = permissions.deny.iter().find(|pattern| {
                settings_permission_paths::glob_search_overlaps_deny_rule(&key, pattern)
            }) {
                return Some(SettingsPermissionDecision::Deny(format!(
                    "Glob search overlaps deny rule '{}'",
                    pattern
                )));
            }
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

    fn actual_permission_key(tool_name: &str, call: &ToolCall, working_dir: &Path) -> String {
        let argument = match tool_name.to_lowercase().as_str() {
            "bash" => call
                .arguments
                .get("command")
                .and_then(|value| value.as_str())
                .map(|command| command.trim().to_string()),
            "read" | "write" | "edit" | "multiedit" => {
                Self::path_permission_argument(call, &["file_path", "path"], working_dir)
            }
            "grep" => Self::path_permission_argument(call, &["path"], working_dir),
            "glob" => Self::glob_permission_argument(call, working_dir),
            "webfetch" => Self::webfetch_permission_argument(call),
            "websearch" => call
                .get_argument::<String>("query")
                .map(|query| query.trim().to_string()),
            "notebookedit" => Self::path_permission_argument(call, &["notebook_path"], working_dir),
            _ => None,
        };

        match argument {
            Some(argument) => format!("{}({})", tool_name, argument),
            None => tool_name.to_string(),
        }
    }

    fn path_permission_argument(
        call: &ToolCall,
        keys: &[&str],
        working_dir: &Path,
    ) -> Option<String> {
        for key in keys {
            if let Some(path) = call.get_argument::<String>(key) {
                return Some(settings_permission_paths::workspace_relative_path(
                    &path,
                    working_dir,
                ));
            }
        }

        None
    }

    fn glob_permission_argument(call: &ToolCall, working_dir: &Path) -> Option<String> {
        let pattern = call.get_argument::<String>("pattern")?;
        let path = call.get_argument::<String>("path");
        let glob_path = path
            .map(|path| PathBuf::from(path).join(&pattern))
            .unwrap_or_else(|| PathBuf::from(pattern));
        Some(settings_permission_paths::workspace_relative_path(
            &glob_path.to_string_lossy(),
            working_dir,
        ))
    }

    fn webfetch_permission_argument(call: &ToolCall) -> Option<String> {
        let url = call.get_argument::<String>("url")?;
        Some(Self::normalize_webfetch_url(&url))
    }

    fn normalize_webfetch_url(url: &str) -> String {
        let trimmed = url.trim();
        let Ok(mut parsed) = reqwest::Url::parse(trimmed) else {
            return trimmed.to_string();
        };

        if !matches!(parsed.scheme(), "http" | "https") {
            return trimmed.to_string();
        }

        if let Some(host) = parsed.host_str() {
            let lowercase_host = host.to_ascii_lowercase();
            if parsed.set_host(Some(&lowercase_host)).is_err() {
                return trimmed.to_string();
            }
        }

        let default_port = match parsed.scheme() {
            "http" => Some(80),
            "https" => Some(443),
            _ => None,
        };
        if parsed.port() == default_port {
            if parsed.set_port(None).is_err() {
                return trimmed.to_string();
            }
        }

        let mut normalized = parsed.to_string();
        if parsed.path() == "/" && parsed.query().is_none() && parsed.fragment().is_none() {
            normalized.truncate(normalized.trim_end_matches('/').len());
        }
        normalized
    }

    fn legacy_permission_text_decision(text: &str) -> Option<bool> {
        match text.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" | "allow" | "allowed" | "approve" | "approved" | "ok" | "true" => {
                Some(true)
            }
            "n" | "no" | "deny" | "denied" | "reject" | "rejected" | "false" => Some(false),
            _ => None,
        }
    }

    fn is_confirmation_only_argument(key: &str) -> bool {
        key == "user_confirmed"
    }
}

#[cfg(test)]
#[path = "settings_permission_tests.rs"]
mod settings_permission_tests;

#[cfg(test)]
#[path = "settings_permission_network_tests.rs"]
mod settings_permission_network_tests;
