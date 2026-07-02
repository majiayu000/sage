//! Permission retry flow for step tool execution.

use crate::input::{InputRequest, InputResponseKind};
use crate::tools::types::{ToolCall, ToolResult};

use super::super::settings_permission::SettingsPermissionCheck;
use super::super::tool_orchestrator::ToolExecutionContext;
use super::UnifiedExecutor;

pub(in crate::agent::unified) enum SettingsRecheckAfterDestructiveConfirmation {
    Ready(ToolCall),
    NeedsDestructiveConfirmation(ToolCall),
}

impl UnifiedExecutor {
    /// Execute a tool and request explicit user confirmation for destructive operations.
    ///
    /// Returns the tool result together with the call that was actually
    /// executed (stripped of the internal confirmation marker), so callers can
    /// keep post-execution hooks, session records, and undo tracking in sync
    /// with user-edited arguments.
    pub(super) async fn execute_with_permission_check(
        &mut self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> (ToolResult, ToolCall, bool) {
        let mut current_call = tool_call.clone();
        let mut prompted_count = 0usize;

        loop {
            let first_result = self
                .tool_orchestrator
                .execution_phase(&current_call, context, cancel_token.clone())
                .await;

            if !Self::requires_destructive_confirmation(&first_result) {
                let executed_call = Self::without_user_confirmation_marker(&current_call);
                return (first_result, executed_call, false);
            }

            prompted_count += 1;
            if prompted_count > 8 {
                let result = ToolResult::error(
                    &current_call.id,
                    &current_call.name,
                    "Destructive confirmation exceeded the maximum number of edited approvals.",
                );
                let executed_call = Self::without_user_confirmation_marker(&current_call);
                return (result, executed_call, false);
            }

            self.event_manager.stop_animation().await;

            let command = current_call
                .arguments
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown command");
            let description = format!(
                "Tool '{}' requires confirmation for a potentially destructive operation.\nCommand: {}",
                current_call.name, command
            );
            let input =
                serde_json::to_value(&current_call.arguments).unwrap_or(serde_json::Value::Null);
            let request = InputRequest::permission(&current_call.name, description, input);

            let response = match self.request_user_input(request).await {
                Ok(response) => response,
                Err(err) => {
                    let result = ToolResult::error(
                        &current_call.id,
                        &current_call.name,
                        format!("Operation cancelled: {}", err),
                    );
                    let executed_call = Self::without_user_confirmation_marker(&current_call);
                    return (result, executed_call, false);
                }
            };

            match response.kind {
                InputResponseKind::PermissionGranted { modified_input, .. } => {
                    let mut confirmed_call = current_call.clone();
                    let input_modified = modified_input.is_some();
                    if let Some(serde_json::Value::Object(map)) = modified_input {
                        confirmed_call.arguments = map.into_iter().collect();
                    }
                    Self::mark_user_confirmed(&mut confirmed_call);

                    match self
                        .recheck_settings_after_destructive_confirmation(
                            confirmed_call,
                            context,
                            input_modified,
                        )
                        .await
                    {
                        Ok(SettingsRecheckAfterDestructiveConfirmation::Ready(confirmed_call)) => {
                            if input_modified {
                                // The user may have redirected the call to a
                                // different file; snapshot it before it runs.
                                self.track_file_for_undo(&confirmed_call, context).await;
                            }
                            let executed_call =
                                Self::without_user_confirmation_marker(&confirmed_call);
                            self.record_session_tool_call(&executed_call).await;
                            let result = self
                                .tool_orchestrator
                                .execution_phase(&confirmed_call, context, cancel_token)
                                .await;
                            return (result, executed_call, true);
                        }
                        Ok(
                            SettingsRecheckAfterDestructiveConfirmation::NeedsDestructiveConfirmation(
                                approved_call,
                            ),
                        ) => {
                            if input_modified {
                                self.track_file_for_undo(&approved_call, context).await;
                            }
                            current_call = approved_call;
                        }
                        Err((blocked_result, blocked_call)) => {
                            let executed_call =
                                Self::without_user_confirmation_marker(&blocked_call);
                            return (blocked_result, executed_call, false);
                        }
                    }
                }
                InputResponseKind::PermissionDenied { reason } => {
                    let reason = reason.unwrap_or_else(|| "No reason provided".to_string());
                    let result = ToolResult::error(
                        &current_call.id,
                        &current_call.name,
                        format!("Operation cancelled by user: {}", reason),
                    );
                    let executed_call = Self::without_user_confirmation_marker(&current_call);
                    return (result, executed_call, false);
                }
                InputResponseKind::Cancelled => {
                    let result = ToolResult::error(
                        &current_call.id,
                        &current_call.name,
                        "Operation cancelled by user.",
                    );
                    let executed_call = Self::without_user_confirmation_marker(&current_call);
                    return (result, executed_call, false);
                }
                _ => {
                    let result = ToolResult::error(
                        &current_call.id,
                        &current_call.name,
                        "Invalid permission response from input handler.",
                    );
                    let executed_call = Self::without_user_confirmation_marker(&current_call);
                    return (result, executed_call, false);
                }
            }
        }
    }

    pub(super) async fn recheck_settings_after_destructive_confirmation(
        &mut self,
        confirmed_call: ToolCall,
        context: &ToolExecutionContext,
        input_modified: bool,
    ) -> std::result::Result<SettingsRecheckAfterDestructiveConfirmation, (ToolResult, ToolCall)>
    {
        if !input_modified {
            return Ok(SettingsRecheckAfterDestructiveConfirmation::Ready(
                confirmed_call,
            ));
        }

        match self
            .check_settings_permission(&confirmed_call, context)
            .await
        {
            Ok(Some(SettingsPermissionCheck::Blocked { result, tool_call })) => {
                Err((result, tool_call))
            }
            Ok(Some(SettingsPermissionCheck::Allowed(mut approved_call))) => {
                let confirmed_without_marker =
                    Self::without_user_confirmation_marker(&confirmed_call);
                if approved_call != confirmed_call && approved_call != confirmed_without_marker {
                    return Ok(
                        SettingsRecheckAfterDestructiveConfirmation::NeedsDestructiveConfirmation(
                            approved_call,
                        ),
                    );
                }

                Self::mark_user_confirmed(&mut approved_call);
                Ok(SettingsRecheckAfterDestructiveConfirmation::Ready(
                    approved_call,
                ))
            }
            Ok(None) => Ok(SettingsRecheckAfterDestructiveConfirmation::Ready(
                confirmed_call,
            )),
            Err(err) => {
                let result = ToolResult::error(
                    &confirmed_call.id,
                    &confirmed_call.name,
                    format!("Settings permission check failed: {}", err),
                );
                Err((result, confirmed_call))
            }
        }
    }

    pub(super) fn mark_user_confirmed(tool_call: &mut ToolCall) {
        tool_call
            .arguments
            .insert("user_confirmed".to_string(), serde_json::Value::Bool(true));
    }

    fn without_user_confirmation_marker(tool_call: &ToolCall) -> ToolCall {
        let mut tool_call = tool_call.clone();
        tool_call.arguments.remove("user_confirmed");
        tool_call
    }

    fn requires_destructive_confirmation(result: &ToolResult) -> bool {
        if result.success {
            return false;
        }

        result.error.as_ref().is_some_and(|err| {
            err.contains("DESTRUCTIVE COMMAND BLOCKED") || err.contains("Confirmation required")
        })
    }
}
