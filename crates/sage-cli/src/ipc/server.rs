//! IPC Server - Rust backend that communicates with Node.js UI via stdio
//!
//! This module implements the Rust side of the IPC communication,
//! receiving requests from stdin and sending events to stdout.
//!
//! # Architecture
//!
//! The IPC Server uses `UnifiedExecutor` - the same execution engine used by
//! the CLI. This ensures consistent behavior across all interfaces and avoids
//! duplicating execution logic.

use super::protocol::{
    ChatParams, ConfigInfo, IpcEvent, IpcRequest, ToolCallInfo, ToolInfo, ToolResultInfo,
};
use sage_core::agent::{ExecutionMode, ExecutionOptions, ExecutionOutcome, UnifiedExecutor};
use sage_core::config::Config;
use sage_core::error::{SageError, SageResult};
use sage_core::tools::types::ToolSchema;
use sage_core::types::TaskMetadata;
use sage_tools::get_default_tools;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use uuid::Uuid;

/// IPC Server that handles communication with Node.js UI
pub struct IpcServer {
    config: Config,
    executor: Arc<Mutex<Option<UnifiedExecutor>>>,
    event_tx: mpsc::UnboundedSender<IpcEvent>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(config: Config) -> SageResult<(Self, mpsc::UnboundedReceiver<IpcEvent>)> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                config,
                executor: Arc::new(Mutex::new(None)),
                event_tx,
            },
            event_rx,
        ))
    }

    /// Initialize the unified executor lazily
    async fn ensure_executor(&self) -> SageResult<()> {
        let mut executor = self.executor.lock().await;
        if executor.is_none() {
            // Create execution options for interactive mode
            let options = ExecutionOptions::default().with_mode(ExecutionMode::non_interactive()); // IPC handles interaction

            let mut exec = UnifiedExecutor::with_options(self.config.clone(), options)?;

            // Register default tools - same as unified.rs
            exec.register_tools(get_default_tools());

            // Initialize sub-agent support
            if let Err(e) = exec.init_subagent_support() {
                eprintln!(
                    "[IPC] Warning: Failed to initialize sub-agent support: {}",
                    e
                );
            }

            *executor = Some(exec);
        }
        Ok(())
    }

    /// Send an event to the UI
    fn send_event(&self, event: IpcEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Handle a chat request
    async fn handle_chat(&self, params: ChatParams) -> SageResult<()> {
        let request_id = params
            .request_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let start_time = Instant::now();

        // Acknowledge the request
        self.send_event(IpcEvent::Ack {
            request_id: request_id.clone(),
        });

        // Signal LLM thinking
        self.send_event(IpcEvent::LlmThinking {
            request_id: request_id.clone(),
        });

        // Ensure executor is initialized
        self.ensure_executor().await?;

        // Get working directory
        let working_dir = params.working_dir.unwrap_or_else(|| ".".to_string());

        // Create task metadata
        let task = TaskMetadata::new(&params.message, &working_dir);

        // Execute using UnifiedExecutor
        let mut executor_guard = self.executor.lock().await;
        let executor = executor_guard
            .as_mut()
            .ok_or_else(|| SageError::agent("Executor not initialized".to_string()))?;

        match executor.execute(task).await {
            Ok(outcome) => {
                // Extract information from outcome
                let execution = outcome.execution();

                // Send tool events for each step
                for step in &execution.steps {
                    // Send tool started/completed events
                    for tool_call in &step.tool_calls {
                        let tool_id = format!("{}-{}", request_id, tool_call.id);

                        self.send_event(IpcEvent::ToolStarted {
                            request_id: request_id.clone(),
                            tool_id: tool_id.clone(),
                            tool_name: tool_call.name.clone(),
                            args: Some(
                                serde_json::to_value(&tool_call.arguments).unwrap_or_default(),
                            ),
                        });
                    }

                    for tool_result in &step.tool_results {
                        let tool_id = format!("{}-{}", request_id, tool_result.call_id);

                        self.send_event(IpcEvent::ToolCompleted {
                            request_id: request_id.clone(),
                            tool_id,
                            success: tool_result.success,
                            output: tool_result.output.clone(),
                            error: tool_result.error.clone(),
                            duration_ms: tool_result
                                .metadata
                                .get("duration_ms")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                        });
                    }
                }

                // Get final content
                let content = execution.final_result.clone().unwrap_or_default();

                // Send LLM done event
                let tool_calls: Vec<ToolCallInfo> = execution
                    .steps
                    .iter()
                    .flat_map(|s| &s.tool_calls)
                    .map(|tc| ToolCallInfo {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        args: tc.arguments.clone(),
                    })
                    .collect();

                self.send_event(IpcEvent::LlmDone {
                    request_id: request_id.clone(),
                    content: content.clone(),
                    tool_calls,
                });

                // Send final completion event
                let tool_results: Vec<ToolResultInfo> = execution
                    .steps
                    .iter()
                    .flat_map(|s| &s.tool_results)
                    .map(|r| ToolResultInfo {
                        tool_id: r.call_id.clone(),
                        tool_name: r.tool_name.clone(),
                        success: r.success,
                        output: r.output.clone(),
                        error: r.error.clone(),
                        duration_ms: r
                            .metadata
                            .get("duration_ms")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                    })
                    .collect();

                let completed = matches!(outcome, ExecutionOutcome::Success(_));

                self.send_event(IpcEvent::ChatCompleted {
                    request_id,
                    content,
                    completed,
                    tool_results,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                });
            }
            Err(e) => {
                self.send_event(IpcEvent::Error {
                    request_id: Some(request_id),
                    code: "chat_error".to_string(),
                    message: e.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Handle get config request
    fn handle_get_config(&self, _config_file: &str) -> SageResult<()> {
        let params = self.config.default_model_parameters()?;

        let working_dir: PathBuf = self
            .config
            .working_directory
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));

        self.send_event(IpcEvent::Config(ConfigInfo {
            provider: self.config.get_default_provider().to_string(),
            model: params.model.clone(),
            max_steps: self.config.max_steps,
            working_directory: working_dir.to_string_lossy().to_string(),
            total_token_budget: self.config.total_token_budget,
        }));

        Ok(())
    }

    /// Handle list tools request
    async fn handle_list_tools(&self) -> SageResult<()> {
        // Get tool schemas directly from the default tools
        // This doesn't require initializing the executor
        let tools: Vec<ToolInfo> = get_default_tools()
            .iter()
            .map(|tool| {
                let schema = tool.schema();
                schema_to_tool_info(&schema)
            })
            .collect();

        self.send_event(IpcEvent::Tools { tools });

        Ok(())
    }

    /// Process a single request
    pub async fn process_request(&self, request: IpcRequest) -> SageResult<bool> {
        match request.method() {
            "ping" => {
                self.send_event(IpcEvent::Pong);
                Ok(false)
            }
            "shutdown" => {
                self.send_event(IpcEvent::ShutdownAck);
                Ok(true) // Signal to stop the server
            }
            "chat" => {
                if let Some(params) = request.as_chat_params() {
                    self.handle_chat(params).await?;
                } else {
                    self.send_event(IpcEvent::Error {
                        request_id: None,
                        code: "invalid_params".to_string(),
                        message: "Invalid chat parameters".to_string(),
                    });
                }
                Ok(false)
            }
            "get_config" => {
                let config_file = request.get_config_file().unwrap_or_default();
                self.handle_get_config(&config_file)?;
                Ok(false)
            }
            "list_tools" => {
                self.handle_list_tools().await?;
                Ok(false)
            }
            "cancel" => {
                let task_id = request.get_task_id().unwrap_or_default();
                // Trigger task cancellation via global interrupt manager
                sage_core::interrupt::interrupt_current_task(
                    sage_core::interrupt::InterruptReason::Manual
                );

                self.send_event(IpcEvent::Error {
                    request_id: Some(task_id),
                    code: "cancelled".to_string(),
                    message: "Task cancellation requested".to_string(),
                });
                Ok(false)
            }
            unknown => {
                self.send_event(IpcEvent::Error {
                    request_id: None,
                    code: "unknown_method".to_string(),
                    message: format!("Unknown method: {}", unknown),
                });
                Ok(false)
            }
        }
    }

    /// Get tool schemas
    #[allow(dead_code)]
    pub async fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        get_default_tools()
            .iter()
            .map(|tool| tool.schema())
            .collect()
    }
}

/// Convert a ToolSchema to ToolInfo for IPC protocol
fn schema_to_tool_info(schema: &ToolSchema) -> ToolInfo {
    let parameters = extract_parameters_from_schema(&schema.parameters);

    ToolInfo {
        name: schema.name.clone(),
        description: schema.description.clone(),
        parameters,
    }
}

/// Extract parameter info from JSON Schema format
fn extract_parameters_from_schema(
    schema: &serde_json::Value,
) -> Vec<super::protocol::ToolParameter> {
    let mut params = Vec::new();

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        let required: Vec<&str> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        for (name, prop) in properties {
            let description = prop
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let param_type = prop
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string")
                .to_string();

            params.push(super::protocol::ToolParameter {
                name: name.clone(),
                description,
                required: required.contains(&name.as_str()),
                param_type,
            });
        }
    }

    params
}

/// Event writer that runs in a separate task
async fn run_event_writer(mut event_rx: mpsc::UnboundedReceiver<IpcEvent>) {
    let stdout = io::stdout();

    while let Some(event) = event_rx.recv().await {
        let line = event.to_json_line();
        let mut handle = stdout.lock();
        if handle.write_all(line.as_bytes()).is_err() {
            break;
        }
        if handle.flush().is_err() {
            break;
        }
    }
}

/// Run the IPC server (main loop)
pub async fn run_ipc_server(config: Config) -> SageResult<()> {
    let (server, event_rx) = IpcServer::new(config)?;
    let server = Arc::new(server);

    // Spawn event writer task
    let writer_handle = tokio::spawn(run_event_writer(event_rx));

    // Send ready event
    server.send_event(IpcEvent::Ready {
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    // Simple request processing loop
    // Read from stdin using spawn_blocking to avoid blocking the async runtime
    loop {
        match tokio::task::spawn_blocking(|| {
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            let mut buf = String::new();
            match handle.read_line(&mut buf) {
                Ok(0) => None, // EOF
                Ok(_) => Some(buf),
                Err(_) => None,
            }
        })
        .await
        {
            Ok(Some(line)) if !line.trim().is_empty() => match IpcRequest::from_json_line(&line) {
                Ok(request) => {
                    let should_shutdown = server.process_request(request).await?;
                    if should_shutdown {
                        break;
                    }
                }
                Err(e) => {
                    server.send_event(IpcEvent::Error {
                        request_id: None,
                        code: "parse_error".to_string(),
                        message: format!("Failed to parse request: {}", e),
                    });
                }
            },
            Ok(Some(_)) => continue,    // Empty line
            Ok(None) | Err(_) => break, // EOF or error
        }
    }

    // Clean up
    drop(server);
    let _ = writer_handle.await;

    Ok(())
}

/// Load configuration and run IPC server
pub async fn run_ipc_mode(config_file: Option<&str>) -> SageResult<()> {
    let config_path = config_file.unwrap_or("sage_config.json");

    eprintln!("[IPC] Looking for config at: {}", config_path);

    let config = if std::path::Path::new(config_path).exists() {
        eprintln!("[IPC] Config file found, loading...");
        let content = std::fs::read_to_string(config_path)?;
        let cfg: Config = serde_json::from_str(&content)?;
        eprintln!(
            "[IPC] Loaded config: provider={}, has_api_key={}",
            cfg.get_default_provider(),
            cfg.model_providers
                .get(cfg.get_default_provider())
                .map(|p| p.api_key.is_some())
                .unwrap_or(false)
        );
        cfg
    } else {
        eprintln!("[IPC] Config file not found, using defaults");
        Config::default()
    };

    run_ipc_server(config).await
}
