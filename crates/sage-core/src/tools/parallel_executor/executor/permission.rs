//! Permission checking logic for tool execution

use crate::tools::base::Tool;
use crate::tools::permission::{
    PermissionCache, PermissionDecision, PermissionRequest, ToolPermissionResult,
};
use crate::tools::types::{ToolCall, ToolResult};
use std::sync::Arc;
use std::time::Duration;

use super::super::config::ToolExecutionResult;
use super::executor::ParallelToolExecutor;

impl ParallelToolExecutor {
    /// Check permission for a tool call
    pub(super) async fn check_permission(
        &self,
        call: &ToolCall,
        tool: &Arc<dyn Tool>,
    ) -> Option<ToolExecutionResult> {
        let cache_key = PermissionCache::cache_key(&call.name, call);

        if self.config.use_permission_cache {
            if let Some(cached) = self.permission_cache.get(&cache_key).await {
                if !cached {
                    self.stats.write().await.permission_denials += 1;
                    return Some(ToolExecutionResult {
                        result: ToolResult::error(
                            &call.id,
                            &call.name,
                            "Permission denied (cached)",
                        ),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    });
                }
            }
        }

        let context = self.tool_context.read().await;
        let perm_result = tool.check_permission(call, &context).await;

        match perm_result {
            ToolPermissionResult::Allow => None,
            ToolPermissionResult::Deny { reason } => {
                self.stats.write().await.permission_denials += 1;
                if self.config.use_permission_cache {
                    self.permission_cache.set(cache_key, false).await;
                }
                Some(ToolExecutionResult {
                    result: ToolResult::error(&call.id, &call.name, reason),
                    wait_time: Duration::ZERO,
                    execution_time: Duration::ZERO,
                    permission_checked: true,
                })
            }
            ToolPermissionResult::Ask {
                question,
                default,
                risk_level,
            } => {
                self.handle_ask_permission(call, tool, &cache_key, question, default, risk_level)
                    .await
            }
            ToolPermissionResult::Transform { .. } => None, // Transform support planned
        }
    }

    /// Handle interactive permission request
    pub(super) async fn handle_ask_permission(
        &self,
        call: &ToolCall,
        tool: &Arc<dyn Tool>,
        cache_key: &str,
        question: String,
        default: bool,
        risk_level: crate::tools::permission::RiskLevel,
    ) -> Option<ToolExecutionResult> {
        let handler = self.permission_handler.read().await;

        if let Some(ref handler) = *handler {
            let request = PermissionRequest::new(tool.name(), call.clone(), question, risk_level);
            let decision = handler.handle_permission_request(request).await;

            match decision {
                PermissionDecision::Allow => None,
                PermissionDecision::AllowAlways => {
                    if self.config.use_permission_cache {
                        self.permission_cache.set(cache_key.to_string(), true).await;
                    }
                    None
                }
                PermissionDecision::Deny => {
                    self.stats.write().await.permission_denials += 1;
                    Some(ToolExecutionResult {
                        result: ToolResult::error(
                            &call.id,
                            &call.name,
                            "Permission denied by user",
                        ),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    })
                }
                PermissionDecision::DenyAlways => {
                    self.stats.write().await.permission_denials += 1;
                    if self.config.use_permission_cache {
                        self.permission_cache
                            .set(cache_key.to_string(), false)
                            .await;
                    }
                    Some(ToolExecutionResult {
                        result: ToolResult::error(
                            &call.id,
                            &call.name,
                            "Permission denied (permanently)",
                        ),
                        wait_time: Duration::ZERO,
                        execution_time: Duration::ZERO,
                        permission_checked: true,
                    })
                }
                PermissionDecision::Modify { .. } => None, // Modify support planned
            }
        } else if !default {
            self.stats.write().await.permission_denials += 1;
            Some(ToolExecutionResult {
                result: ToolResult::error(&call.id, &call.name, "Permission denied (no handler)"),
                wait_time: Duration::ZERO,
                execution_time: Duration::ZERO,
                permission_checked: true,
            })
        } else {
            None
        }
    }
}
