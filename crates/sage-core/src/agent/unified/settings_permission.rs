//! Settings-backed permission checks for unified tool execution.

use crate::error::SageResult;
use crate::input::{InputRequest, InputResponseKind};
use crate::settings::SettingsLoader;
use crate::settings::locations::SettingsLocations;
use crate::settings::types::{Settings, SettingsPermissionBehavior};
use crate::settings::validation::SettingsValidator;
use crate::tools::permission::PermissionCache;
use crate::tools::types::{ToolCall, ToolResult};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

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
                    approved_call.arguments = map.into_iter().collect();
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

        if tool_name == "Grep"
            && tool_call
                .get_argument::<String>("path")
                .is_none_or(|path| path.is_empty())
        {
            if let Some(pattern) = permissions.deny.iter().find(|pattern| {
                let lower = pattern.to_ascii_lowercase();
                lower == "grep" || lower.starts_with("grep(")
            }) {
                return Some(SettingsPermissionDecision::Deny(format!(
                    "omitted Grep path searches the workspace and overlaps deny rule '{}'",
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
                .map(str::to_string),
            "read" | "write" | "edit" | "multiedit" => {
                Self::path_permission_argument(call, &["file_path", "path"], working_dir)
            }
            "grep" => Self::path_permission_argument(call, &["path"], working_dir),
            "glob" => Self::glob_permission_argument(call, working_dir),
            "webfetch" => call
                .arguments
                .get("url")
                .and_then(|value| value.as_str())
                .map(str::to_string),
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
                return Some(Self::workspace_relative_path(&path, working_dir));
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
        Some(Self::workspace_relative_path(
            &glob_path.to_string_lossy(),
            working_dir,
        ))
    }

    fn workspace_relative_path(path: &str, working_dir: &Path) -> String {
        let working_dir = Self::absolute_working_dir(working_dir);
        let path = Self::absolute_permission_path(path, &working_dir);
        let relative_path = if path.is_absolute() {
            path.strip_prefix(&working_dir)
                .unwrap_or(&path)
                .to_path_buf()
        } else {
            path
        };

        relative_path
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string()
    }

    fn absolute_working_dir(working_dir: &Path) -> PathBuf {
        let path = if working_dir.is_absolute() {
            working_dir.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(working_dir)
        };
        let normalized = Self::normalize_permission_path(&path);
        normalized.canonicalize().unwrap_or(normalized)
    }

    fn absolute_permission_path(path: &str, working_dir: &Path) -> PathBuf {
        let path = Path::new(path);
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_dir.join(path)
        };
        let normalized_path = Self::normalize_permission_path(&absolute_path);
        Self::canonicalize_existing_prefix(&normalized_path).unwrap_or(normalized_path)
    }

    fn canonicalize_existing_prefix(path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = path.canonicalize() {
            return Some(Self::normalize_permission_path(&canonical));
        }

        let mut current = path.to_path_buf();
        let mut missing_components: Vec<OsString> = Vec::new();

        loop {
            if current.exists() {
                let mut resolved = current.canonicalize().ok()?;
                for component in missing_components.iter().rev() {
                    resolved.push(component);
                }
                return Some(Self::normalize_permission_path(&resolved));
            }

            if let Some(file_name) = current.file_name() {
                missing_components.push(file_name.to_os_string());
            }

            let parent = current.parent()?;
            if parent == current {
                return None;
            }
            current = parent.to_path_buf();
        }
    }

    fn normalize_permission_path(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        normalized.push("..");
                    }
                }
                Component::Normal(part) => normalized.push(part),
                Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
            }
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
}

#[cfg(test)]
#[path = "settings_permission_tests.rs"]
mod settings_permission_tests;
