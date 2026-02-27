//! Executor creation logic

use sage_core::agent::UnifiedExecutor;
use sage_core::agent::{ExecutionMode, ExecutionOptions};
use sage_core::error::SageResult;
use sage_core::output::OutputMode;
use sage_core::ui::traits::UiContext;

/// Create executor with unified configuration path
pub async fn create_executor(
    ui_context: Option<UiContext>,
    config_file: &str,
    working_dir: Option<std::path::PathBuf>,
    max_steps: Option<u32>,
) -> SageResult<UnifiedExecutor> {
    let config = if std::path::Path::new(config_file).exists() {
        sage_core::config::load_config_from_file(config_file)?
    } else {
        sage_core::config::load_config()?
    };

    let resolved_working_dir = working_dir
        .or_else(|| config.working_directory.clone())
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        });

    let mut options = ExecutionOptions::default()
        .with_mode(ExecutionMode::interactive())
        .with_working_directory(&resolved_working_dir);

    if let Some(steps) = max_steps {
        options = options.with_step_limit(steps);
    }

    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;

    if let Some(ctx) = ui_context {
        executor.set_ui_context(ctx);
    }

    executor.set_output_mode(OutputMode::Rnk);

    // Register default tools
    let mut all_tools = sage_tools::get_default_tools();

    // Load MCP tools if MCP is enabled
    if config.mcp.enabled {
        match crate::commands::unified::build_mcp_registry_from_config(&config).await {
            Ok(mcp_registry) => {
                let mcp_tools = mcp_registry.as_tools().await;
                if !mcp_tools.is_empty() {
                    all_tools.extend(mcp_tools);
                }
            }
            Err(e) => {
                tracing::error!("Failed to build MCP registry: {}", e);
            }
        }
    }

    executor.register_tools(all_tools);
    if let Err(e) = executor.init_subagent_support() {
        tracing::warn!("Failed to initialize subagent support: {}", e);
    }

    // Set up JSONL storage for session management
    let jsonl_storage = sage_core::session::JsonlSessionStorage::default_path()?;
    executor.set_jsonl_storage(std::sync::Arc::new(jsonl_storage));

    // Enable JSONL session recording
    if let Err(e) = executor.enable_session_recording().await {
        tracing::warn!("Failed to enable session recording: {}", e);
    }

    Ok(executor)
}
