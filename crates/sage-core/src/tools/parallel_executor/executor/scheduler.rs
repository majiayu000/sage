//! Scheduling logic for permit acquisition and result ordering

use crate::tools::base::{ConcurrencyMode, Tool, ToolError};
use crate::tools::types::ToolCall;
use std::collections::HashMap;
use std::sync::Arc;

use super::super::config::ExecutionResult;
use super::executor::ParallelToolExecutor;
use super::types::PermitGuard;

impl ParallelToolExecutor {
    /// Partition tool calls by concurrency mode
    pub(super) fn partition_by_concurrency(
        &self,
        calls: &[ToolCall],
    ) -> (Vec<ToolCall>, Vec<ToolCall>) {
        let mut parallel = Vec::new();
        let mut sequential = Vec::new();

        for call in calls {
            let is_parallel = self
                .tools
                .get(&call.name)
                .map(|tool| {
                    matches!(
                        tool.concurrency_mode(),
                        ConcurrencyMode::Parallel | ConcurrencyMode::Limited(_)
                    )
                })
                .unwrap_or(false);

            if is_parallel {
                parallel.push(call.clone());
            } else {
                sequential.push(call.clone());
            }
        }

        (parallel, sequential)
    }

    /// Acquire necessary permits for tool execution
    pub(super) async fn acquire_permits(
        &self,
        tool: &Arc<dyn Tool>,
    ) -> Result<PermitGuard, ToolError> {
        let mut permits = PermitGuard::new();

        let global_permit = self
            .global_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ToolError::Other("Failed to acquire global permit".into()))?;
        permits.add_global(global_permit);

        match tool.concurrency_mode() {
            ConcurrencyMode::Sequential => {
                let _lock = self.sequential_lock.lock().await;
            }
            ConcurrencyMode::ExclusiveByType => {
                if let Some(semaphore) = self.type_semaphores.get(tool.name()) {
                    let permit =
                        semaphore.clone().acquire_owned().await.map_err(|_| {
                            ToolError::Other("Failed to acquire type permit".into())
                        })?;
                    permits.add_type(permit);
                }
            }
            ConcurrencyMode::Limited(_) => {
                if let Some(semaphore) = self.limited_semaphores.get(tool.name()) {
                    let permit =
                        semaphore.clone().acquire_owned().await.map_err(|_| {
                            ToolError::Other("Failed to acquire limited permit".into())
                        })?;
                    permits.add_limited(permit);
                }
            }
            ConcurrencyMode::Parallel => {}
        }

        Ok(permits)
    }

    /// Reorder results to match original call order
    pub(super) fn reorder_results(
        &self,
        original_calls: &[ToolCall],
        mut results: Vec<ExecutionResult>,
    ) -> Vec<ExecutionResult> {
        use crate::tools::types::ToolResult;
        use std::time::Duration;

        let mut ordered = Vec::with_capacity(original_calls.len());
        let mut result_map: HashMap<String, ExecutionResult> = results
            .drain(..)
            .map(|r| (r.result.call_id.clone(), r))
            .collect();

        for call in original_calls {
            if let Some(result) = result_map.remove(&call.id) {
                ordered.push(result);
            } else {
                ordered.push(ExecutionResult {
                    result: ToolResult::error(&call.id, &call.name, "Result not found"),
                    wait_time: Duration::ZERO,
                    execution_time: Duration::ZERO,
                    permission_checked: false,
                });
            }
        }

        ordered
    }
}
