//! Settings-backed permission checks for unified tool execution.

use crate::error::SageResult;
use crate::input::{InputRequest, InputResponseKind};
use crate::permissions::{
    ApprovalCacheDecision, ApprovalCacheLookup, FilesystemPermissionProfile,
    PermissionDecisionEngine, PermissionDecisionKind, PermissionPreflight, PermissionProfile,
    PermissionProfileSource, SandboxSupport,
};
use crate::settings::SettingsLoader;
use crate::settings::locations::SettingsLocations;
use crate::settings::types::Settings;
use crate::settings::validation::SettingsValidator;
use crate::tools::types::{ToolCall, ToolResult};
use std::path::Path;
use std::time::{Duration, Instant};

use super::UnifiedExecutor;
use super::tool_orchestrator::ToolExecutionContext;

#[cfg(test)]
use crate::settings::types::SettingsPermissionBehavior;

#[path = "settings_permission_paths.rs"]
mod settings_permission_paths;

#[path = "settings_permission_keys.rs"]
mod settings_permission_keys;

#[path = "settings_permission_inputs.rs"]
mod settings_permission_inputs;

#[path = "settings_permission_diagnostics.rs"]
mod settings_permission_diagnostics;

#[path = "settings_permission_policy.rs"]
mod settings_permission_policy;

#[path = "settings_permission_check.rs"]
mod settings_permission_check;
pub(in crate::agent::unified) use settings_permission_check::SettingsPermissionCheck;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettingsPermissionDecision {
    Allow,
    Deny(String),
    Ask(SettingsPermissionAsk),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SettingsPermissionAsk {
    reason: String,
    audit_key: String,
    cache_ttl_ms: Option<u64>,
}

enum SettingsPermissionPromptResult {
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
    pub(in crate::agent::unified) async fn check_settings_permission(
        &mut self,
        tool_call: &ToolCall,
        context: &ToolExecutionContext,
    ) -> SageResult<Option<SettingsPermissionCheck>> {
        let settings = Self::load_settings_strict(&context.working_dir)?;
        let mut current_call = tool_call.clone();
        let mut prompted_count = 0usize;

        loop {
            let sandbox_support = if context.sandboxed {
                SandboxSupport::Supported
            } else {
                SandboxSupport::Unsupported
            };
            let Some(decision) = Self::settings_permission_decision_with_sandbox_support(
                &settings,
                &current_call,
                &context.working_dir,
                sandbox_support,
            ) else {
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
                    let result = Self::settings_permission_blocked_result(
                        &current_call,
                        format!("Permission denied by settings: {}", reason),
                    );
                    return Ok(Some(SettingsPermissionCheck::Blocked {
                        result,
                        tool_call: current_call,
                    }));
                }
                SettingsPermissionDecision::Ask(ask) => {
                    let cache_ttl = approval_cache_ttl(ask.cache_ttl_ms);
                    if cache_ttl.is_some() {
                        match self.approval_cache.lookup(&ask.audit_key, Instant::now()) {
                            ApprovalCacheLookup::Hit(ApprovalCacheDecision::Allow) => {
                                return Ok(Some(SettingsPermissionCheck::Allowed(current_call)));
                            }
                            ApprovalCacheLookup::Hit(ApprovalCacheDecision::Deny) => {
                                let result = Self::settings_permission_blocked_result(
                                    &current_call,
                                    format!(
                                        "Permission denied by cached approval decision: {}",
                                        ask.reason
                                    ),
                                );
                                return Ok(Some(SettingsPermissionCheck::Blocked {
                                    result,
                                    tool_call: current_call,
                                }));
                            }
                            ApprovalCacheLookup::Expired | ApprovalCacheLookup::Miss => {}
                        }
                    }

                    prompted_count += 1;
                    if prompted_count > 8 {
                        let result = Self::settings_permission_blocked_result(
                            &current_call,
                            "Permission request exceeded the maximum number of edited approvals.",
                        );
                        return Ok(Some(SettingsPermissionCheck::Blocked {
                            result,
                            tool_call: current_call,
                        }));
                    }

                    match self
                        .request_settings_permission(&current_call, ask.reason)
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

                            if let Some(ttl) = cache_ttl {
                                self.approval_cache.insert(
                                    ask.audit_key,
                                    ApprovalCacheDecision::Allow,
                                    Some(ttl),
                                    Instant::now(),
                                );
                            }
                            return Ok(Some(SettingsPermissionCheck::Allowed(approved_call)));
                        }
                        SettingsPermissionPromptResult::Blocked {
                            result,
                            cache_decision,
                        } => {
                            if let (Some(ttl), Some(cache_decision)) = (cache_ttl, cache_decision) {
                                self.approval_cache.insert(
                                    ask.audit_key,
                                    cache_decision,
                                    Some(ttl),
                                    Instant::now(),
                                );
                            }
                            return Ok(Some(SettingsPermissionCheck::Blocked {
                                result,
                                tool_call: current_call,
                            }));
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
            SettingsPermissionDecision::Ask(ask) => {
                Ok(Some(Self::settings_permission_blocked_result(
                    tool_call,
                    format!(
                        "Permission required by settings but sub-agent tool calls cannot prompt for approval: {}",
                        ask.reason
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

    fn settings_permission_blocked_result(
        tool_call: &ToolCall,
        message: impl Into<String>,
    ) -> ToolResult {
        let message = message.into();
        settings_permission_diagnostics::record_blocked_result(tool_call, &message);
        ToolResult::error(&tool_call.id, &tool_call.name, message)
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
        loader.load_managed_configs(&mut settings)?;
        SettingsValidator::new().validate(&settings)?;
        Ok(settings)
    }

    fn settings_permission_decision(
        settings: &Settings,
        tool_call: &ToolCall,
        working_dir: &Path,
    ) -> Option<SettingsPermissionDecision> {
        Self::settings_permission_decision_with_sandbox_support(
            settings,
            tool_call,
            working_dir,
            SandboxSupport::Unknown,
        )
    }

    fn settings_permission_decision_with_sandbox_support(
        settings: &Settings,
        tool_call: &ToolCall,
        working_dir: &Path,
        sandbox_support: SandboxSupport,
    ) -> Option<SettingsPermissionDecision> {
        let tool_name = settings_permission_keys::canonical_permission_tool_name(&tool_call.name);
        let keys =
            settings_permission_keys::actual_permission_keys(&tool_name, tool_call, working_dir);
        let mut profile = PermissionProfile::from_settings(&settings.permissions)
            .with_filesystem_profile(
                FilesystemPermissionProfile {
                    workspace_roots: vec![working_dir.to_string_lossy().to_string()],
                    ..Default::default()
                },
                PermissionProfileSource::Local,
            );
        for managed in &settings.managed_configs {
            managed.config.apply_restrictive_to(&mut profile);
        }
        let has_configured_rules = profile.has_configured_rules();
        if !has_configured_rules && settings.managed_configs.is_empty() {
            return None;
        }

        let key = keys
            .first()
            .cloned()
            .unwrap_or_else(|| tool_name.to_string());

        let mut preflight_denies = Vec::new();
        let deny_rules = settings_permission_policy::deny_rules(settings);
        if tool_name == "Grep" && matches!(key.as_str(), "Grep" | "Grep()") {
            if let Some(rule) = deny_rules.iter().find(|rule| {
                let lower = rule.pattern.to_ascii_lowercase();
                lower == "grep" || lower.starts_with("grep(")
            }) {
                preflight_denies.push(
                    PermissionPreflight::new(
                        format!(
                            "workspace-wide Grep search overlaps deny rule '{}'",
                            rule.pattern
                        ),
                        Some(rule.pattern.clone()),
                    )
                    .with_source(rule.source),
                );
            }
        }

        if tool_name == "Grep" {
            if let Some(rule) = deny_rules.iter().find(|rule| {
                settings_permission_paths::grep_search_overlaps_deny_rule(&key, &rule.pattern)
            }) {
                preflight_denies.push(
                    PermissionPreflight::new(
                        format!("Grep search overlaps deny rule '{}'", rule.pattern),
                        Some(rule.pattern.clone()),
                    )
                    .with_source(rule.source),
                );
            }
        }

        if tool_name == "Glob" {
            if let Some(rule) = deny_rules.iter().find(|rule| {
                settings_permission_paths::glob_search_overlaps_deny_rule(&key, &rule.pattern)
            }) {
                preflight_denies.push(
                    PermissionPreflight::new(
                        format!("Glob search overlaps deny rule '{}'", rule.pattern),
                        Some(rule.pattern.clone()),
                    )
                    .with_source(rule.source),
                );
            }
        }

        if tool_name == "http_client"
            && settings_permission_policy::http_client_may_follow_redirects(tool_call)
            && settings_permission_policy::http_client_redirects_require_disabled(
                settings,
                &deny_rules,
            )
        {
            preflight_denies.push(PermissionPreflight::new(
                "http_client must set follow_redirects=false when settings URL policy can prompt or block redirects".to_string(),
                None,
            ));
        }

        let mut scoped_allows = Vec::new();
        if tool_name == "Grep" {
            if let Some(pattern) = settings.permissions.allow.iter().find(|pattern| {
                settings_permission_paths::grep_search_within_allow_rule(&key, pattern)
            }) {
                scoped_allows.push(PermissionPreflight::new(
                    format!("Grep search is within allow rule '{}'", pattern),
                    Some(pattern.clone()),
                ));
            }
        }

        let approval_cache_ttl_ms = profile.approval.cache_ttl_ms;
        let decisions = settings_permission_inputs::settings_permission_inputs(
            &tool_name,
            tool_call,
            working_dir,
            keys,
            preflight_denies,
            scoped_allows,
        )
        .into_iter()
        .map(|input| {
            let input = if profile.sandbox.required {
                input.with_required_sandbox(sandbox_support)
            } else {
                input
            };
            let mut input_profile = profile.clone();
            input_profile.filesystem.allow_outside_workspace =
                settings_permission_policy::settings_allow_outside_for_input(
                    &settings.permissions.allow,
                    &input,
                );
            PermissionDecisionEngine::new(input_profile).decide(input)
        });

        let mut first_ask = None;
        for decision in decisions {
            match decision.kind {
                PermissionDecisionKind::Allow => {}
                PermissionDecisionKind::Deny => {
                    return Some(SettingsPermissionDecision::Deny(
                        settings_permission_policy::decision_reason(&decision),
                    ));
                }
                PermissionDecisionKind::Ask => {
                    first_ask.get_or_insert_with(|| SettingsPermissionAsk {
                        reason: settings_permission_policy::decision_reason(&decision),
                        audit_key: decision.audit_key,
                        cache_ttl_ms: approval_cache_ttl_ms,
                    });
                }
                PermissionDecisionKind::Unsupported => {
                    return Some(SettingsPermissionDecision::Deny(format!(
                        "unsupported permission request: {}",
                        decision.reason
                    )));
                }
            }
        }

        if !has_configured_rules {
            None
        } else if let Some(ask) = first_ask {
            Some(SettingsPermissionDecision::Ask(ask))
        } else {
            Some(SettingsPermissionDecision::Allow)
        }
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

fn approval_cache_ttl(cache_ttl_ms: Option<u64>) -> Option<Duration> {
    cache_ttl_ms
        .filter(|ttl_ms| *ttl_ms > 0)
        .map(Duration::from_millis)
}

#[cfg(test)]
#[path = "settings_permission_test_support.rs"]
mod settings_permission_test_support;

#[cfg(test)]
#[path = "settings_permission_tests.rs"]
mod settings_permission_tests;

#[cfg(test)]
#[path = "settings_permission_shell_tests.rs"]
mod settings_permission_shell_tests;

#[cfg(test)]
#[path = "settings_permission_path_tests.rs"]
mod settings_permission_path_tests;

#[cfg(test)]
#[path = "settings_permission_network_tests.rs"]
mod settings_permission_network_tests;

#[cfg(test)]
#[path = "settings_permission_review_tests.rs"]
mod settings_permission_review_tests;
