//! Shared executor factory for creating UnifiedExecutor instances

use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config;
use sage_core::error::SageResult;
use sage_core::output::OutputMode;
use sage_core::ui::traits::UiContext;
use sage_tools::get_default_tools;

/// Create and configure a UnifiedExecutor with the given output mode and optional UI context.
pub async fn create_executor(
    output_mode: OutputMode,
    ui_context: Option<UiContext>,
) -> SageResult<UnifiedExecutor> {
    let config = load_config()?;
    let working_dir = std::env::current_dir().unwrap_or_default();
    let mode = ExecutionMode::interactive();
    let options = ExecutionOptions::default()
        .with_mode(mode)
        .with_working_directory(&working_dir);

    let mut executor = UnifiedExecutor::with_options(config, options)?;

    if let Some(ctx) = ui_context {
        executor.set_ui_context(ctx);
    }

    executor.set_output_mode(output_mode);
    executor.register_tools(get_default_tools());
    let _ = executor.init_subagent_support();

    Ok(executor)
}
