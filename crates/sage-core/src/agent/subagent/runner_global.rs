//! Global sub-agent runner registry.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use super::SubAgentRunner;
use crate::agent::subagent::types::{SubAgentConfig, SubAgentResult};
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::tools::base::Tool;

static GLOBAL_RUNNER: std::sync::OnceLock<Arc<RwLock<Option<SubAgentRunner>>>> =
    std::sync::OnceLock::new();

pub fn init_global_runner_from_config(
    config: &Config,
    tools: Vec<Arc<dyn Tool>>,
    working_directory: Option<PathBuf>,
) -> SageResult<()> {
    let runner = SubAgentRunner::from_config(config, tools, working_directory)?;
    init_global_runner(runner);
    Ok(())
}

pub fn init_global_runner(runner: SubAgentRunner) {
    let lock = GLOBAL_RUNNER.get_or_init(|| Arc::new(RwLock::new(None)));
    if let Ok(mut guard) = lock.try_write() {
        *guard = Some(runner);
        tracing::info!("Global sub-agent runner initialized");
    }
}

pub async fn update_global_runner_tools(tools: Vec<Arc<dyn Tool>>) {
    if let Some(lock) = GLOBAL_RUNNER.get() {
        let mut guard = lock.write().await;
        if let Some(runner) = guard.as_mut() {
            runner.update_tools(tools);
            tracing::debug!("Updated global sub-agent runner tools");
        }
    }
}

pub async fn update_global_runner_cwd(cwd: PathBuf) {
    if let Some(lock) = GLOBAL_RUNNER.get() {
        let mut guard = lock.write().await;
        if let Some(runner) = guard.as_mut() {
            runner.set_working_directory(cwd.clone());
            tracing::debug!(
                "Updated global sub-agent runner working directory: {:?}",
                cwd
            );
        }
    }
}

pub async fn get_global_runner_cwd() -> Option<PathBuf> {
    if let Some(lock) = GLOBAL_RUNNER.get() {
        let guard = lock.read().await;
        guard.as_ref().map(|r| r.working_directory().clone())
    } else {
        None
    }
}

pub fn get_global_runner() -> Option<Arc<RwLock<Option<SubAgentRunner>>>> {
    GLOBAL_RUNNER.get().cloned()
}

pub async fn execute_subagent(config: SubAgentConfig) -> SageResult<SubAgentResult> {
    let runner_lock = get_global_runner().ok_or_else(|| {
        SageError::agent(
            "Sub-agent runner not initialized. Call init_global_runner_from_config first.",
        )
    })?;

    let guard = runner_lock.read().await;
    let runner = guard
        .as_ref()
        .ok_or_else(|| SageError::agent("Sub-agent runner not available"))?;

    let cancel = CancellationToken::new();
    runner.execute(config, cancel).await
}
