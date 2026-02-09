//! Main execute function for the unified command

use crate::console::CliConsole;
use crate::signal_handler::start_global_signal_handling;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::load_config_from_file;
use sage_core::error::SageResult;
use sage_core::input::InputChannel;
use sage_core::output::OutputMode;
use sage_tools::get_default_tools;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinHandle;

use super::args::{OutputModeArg, UnifiedArgs};
use super::input::handle_user_input;
use super::mcp::build_mcp_registry_from_config;
use super::session::{execute_session_resume, execute_single_task};
use super::stream::execute_stream_json;
use super::utils::load_task_from_arg;

/// Execute a task using the unified execution loop
pub async fn execute(args: UnifiedArgs) -> SageResult<()> {
    let console = CliConsole::new(args.verbose);

    // Initialize signal handling
    if let Err(e) = start_global_signal_handling().await {
        console.warn(&format!("Failed to initialize signal handling: {}", e));
    }

    // Load configuration
    let mut config = if std::path::Path::new(&args.config_file).exists() {
        load_config_from_file(&args.config_file)?
    } else {
        let global_config = dirs::home_dir().map(|h| h.join(".sage").join("config.json"));
        if global_config.as_ref().is_none_or(|path| !path.exists()) {
            console.warn(&format!(
                "Configuration file not found: {}, using defaults",
                args.config_file
            ));
        }
        sage_core::config::load_config()?
    };

    // If the default provider has no key, pick the first provider that does.
    if let Some(params) = config.model_providers.get(&config.default_provider) {
        if params
            .get_api_key_info_for_provider(&config.default_provider)
            .key
            .is_none()
        {
            if let Some((provider, _)) = config.model_providers.iter().find(|(provider, params)| {
                params.get_api_key_info_for_provider(provider).key.is_some()
                    || provider.as_str() == "ollama"
            }) {
                config.default_provider = provider.clone();
            }
        }
    }

    // Determine working directory
    let working_dir = args
        .working_dir
        .clone()
        .or_else(|| config.working_directory.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Set up execution options
    let mode = if args.non_interactive {
        ExecutionMode::non_interactive()
    } else {
        ExecutionMode::interactive()
    };

    let mut options = ExecutionOptions::default().with_mode(mode);
    if let Some(max_steps) = args.max_steps {
        options = options.with_step_limit(max_steps);
    }
    options = options.with_working_directory(&working_dir);

    // Create the unified executor
    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;

    // Set output mode based on args
    let output_mode = match args.output_mode {
        OutputModeArg::Streaming => OutputMode::Streaming,
        OutputModeArg::Batch => OutputMode::Batch,
        OutputModeArg::Silent => OutputMode::Silent,
    };
    executor.set_output_mode(output_mode);

    // Register default tools
    let mut all_tools = get_default_tools();

    // Load MCP tools if MCP is enabled
    if config.mcp.enabled {
        tracing::info!("MCP is enabled, building MCP registry...");
        match build_mcp_registry_from_config(&config).await {
            Ok(mcp_registry) => {
                let mcp_tools = mcp_registry.as_tools().await;
                tracing::info!(
                    "Loaded {} MCP tools from {} servers",
                    mcp_tools.len(),
                    mcp_registry.server_names().len()
                );
                if !mcp_tools.is_empty() {
                    all_tools.extend(mcp_tools);
                }
            }
            Err(e) => {
                tracing::error!("Failed to build MCP registry: {}", e);
            }
        }
    } else {
        tracing::debug!("MCP is disabled in configuration");
    }

    executor.register_tools(all_tools);

    // Initialize sub-agent support for Task tool
    if let Err(e) = executor.init_subagent_support() {
        console.warn(&format!("Failed to initialize sub-agent support: {}", e));
    }

    // Set up JSONL storage for session management
    let jsonl_storage = sage_core::session::JsonlSessionStorage::default_path()?;
    let jsonl_storage = Arc::new(jsonl_storage);
    executor.set_jsonl_storage(jsonl_storage.clone());

    // Enable JSONL session recording
    if let Err(e) = executor.enable_session_recording().await {
        console.warn(&format!("Failed to enable session recording: {}", e));
    }

    let config_file = args.config_file.clone();

    // Handle session resume (-c or -r flags)
    if args.continue_recent || args.resume_session_id.is_some() {
        return execute_session_resume(args, executor, console, config, working_dir, config_file)
            .await;
    }

    // Handle stream JSON mode (for SDK/programmatic use)
    if args.stream_json {
        return execute_stream_json(args, executor, config, working_dir).await;
    }

    // Set up session recording
    let session_recorder = if config.trajectory.is_enabled() {
        let recorder = sage_core::trajectory::init_session_recorder(&working_dir);
        if let Some(ref r) = recorder {
            executor.set_session_recorder(r.clone());
        }
        recorder
    } else {
        None
    };

    // Set up input channel for interactive mode
    let mut input_task: Option<JoinHandle<()>> = None;
    let verbose = args.verbose;
    if !args.non_interactive {
        let (input_channel, input_handle) = InputChannel::new(16);
        executor.set_input_channel(input_channel);
        input_task = Some(tokio::spawn(handle_user_input(input_handle, verbose)));
    }

    // Determine execution mode based on whether task was provided
    if let Some(task) = args.task {
        let task_description = load_task_from_arg(&task, &console).await?;
        let result = execute_single_task(
            &mut executor,
            &console,
            &working_dir,
            &jsonl_storage,
            &session_recorder,
            &task_description,
            &args.config_file,
        )
        .await;

        if let Some(handle) = input_task.take() {
            handle.abort();
        }

        return result;
    }

    if let Some(handle) = input_task.take() {
        handle.abort();
    }

    Err(sage_core::error::SageError::invalid_input(
        "Interactive mode requires a TTY. Run without piping, or provide a task with `sage \"task\"` or `sage -p \"task\"`.",
    ))
}
