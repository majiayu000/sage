use crate::error::SageResult;
use crate::input::{InputRequest, InputResponseKind};
use crate::permissions::ApprovalCacheDecision;
use crate::tools::types::{ToolCall, ToolResult};

use super::UnifiedExecutor;
use super::settings_permission_policy;

pub(super) enum SettingsPermissionPromptResult {
    Allowed {
        tool_call: ToolCall,
        input_modified: bool,
    },
    Blocked {
        result: ToolResult,
        cache_decision: Option<ApprovalCacheDecision>,
    },
}

impl UnifiedExecutor {
    pub(super) async fn request_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        reason: String,
    ) -> SageResult<SettingsPermissionPromptResult> {
        self.event_manager.stop_animation().await;
        let input = serde_json::to_value(&tool_call.arguments).unwrap_or(serde_json::Value::Null);
        let mut request = InputRequest::permission(
            &tool_call.name,
            format!(
                "Tool '{}' requires permission from settings.\n{}",
                tool_call.name, reason
            ),
            input,
        );
        if let Some(timeout) = self.options.prompt_timeout {
            request = request.with_timeout(timeout);
        }

        let response = match self.request_user_input(request).await {
            Ok(response) => response,
            Err(err) => {
                return Ok(SettingsPermissionPromptResult::Blocked {
                    result: Self::settings_permission_blocked_result(
                        tool_call,
                        format!("Permission request failed: {}", err),
                    ),
                    cache_decision: None,
                });
            }
        };

        match response.kind {
            InputResponseKind::PermissionGranted { modified_input, .. } => {
                let mut approved_call = tool_call.clone();
                let input_modified = modified_input.is_some();
                if let Some(serde_json::Value::Object(map)) = modified_input {
                    approved_call.arguments = map
                        .into_iter()
                        .filter(|(key, _)| {
                            !settings_permission_policy::is_confirmation_only_argument(key)
                        })
                        .collect();
                }

                Ok(SettingsPermissionPromptResult::Allowed {
                    tool_call: approved_call,
                    input_modified,
                })
            }
            InputResponseKind::PermissionDenied { reason } => {
                let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
                Ok(SettingsPermissionPromptResult::Blocked {
                    result: Self::settings_permission_blocked_result(
                        tool_call,
                        format!("Permission denied by user: {}", reason),
                    ),
                    cache_decision: Some(ApprovalCacheDecision::Deny),
                })
            }
            InputResponseKind::Cancelled => Ok(SettingsPermissionPromptResult::Blocked {
                result: Self::settings_permission_blocked_result(
                    tool_call,
                    "Permission request cancelled by user.",
                ),
                cache_decision: None,
            }),
            InputResponseKind::FreeText { text }
            | InputResponseKind::Simple { content: text, .. } => {
                match Self::legacy_permission_text_decision(&text) {
                    Some(true) => Ok(SettingsPermissionPromptResult::Allowed {
                        tool_call: tool_call.clone(),
                        input_modified: false,
                    }),
                    Some(false) => Ok(SettingsPermissionPromptResult::Blocked {
                        result: Self::settings_permission_blocked_result(
                            tool_call,
                            format!("Permission denied by user response: {}", text),
                        ),
                        cache_decision: Some(ApprovalCacheDecision::Deny),
                    }),
                    None => Ok(SettingsPermissionPromptResult::Blocked {
                        result: Self::settings_permission_blocked_result(
                            tool_call,
                            "Invalid permission response from input handler.",
                        ),
                        cache_decision: None,
                    }),
                }
            }
            _ => Ok(SettingsPermissionPromptResult::Blocked {
                result: Self::settings_permission_blocked_result(
                    tool_call,
                    "Invalid permission response from input handler.",
                ),
                cache_decision: None,
            }),
        }
    }
}
