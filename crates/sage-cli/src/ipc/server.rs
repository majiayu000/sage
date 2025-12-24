//! IPC Server - Rust backend that communicates with Node.js UI via stdio
//!
//! This module implements the Rust side of the IPC communication,
//! receiving requests from stdin and sending events to stdout.

use super::protocol::{
    ChatParams, ConfigInfo, IpcEvent, IpcRequest, ToolCallInfo, ToolInfo, ToolResultInfo,
};
use sage_core::config::Config;
use sage_core::error::{SageError, SageResult};
use sage_core::tools::types::ToolSchema;
use sage_core::ReactiveExecutionManager;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// IPC Server that handles communication with Node.js UI
pub struct IpcServer {
    config: Config,
    execution_manager: Arc<Mutex<Option<ReactiveExecutionManager>>>,
    event_tx: mpsc::UnboundedSender<IpcEvent>,
}

impl IpcServer {
    /// Create a new IPC server
    pub fn new(config: Config) -> SageResult<(Self, mpsc::UnboundedReceiver<IpcEvent>)> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Ok((
            Self {
                config,
                execution_manager: Arc::new(Mutex::new(None)),
                event_tx,
            },
            event_rx,
        ))
    }

    /// Initialize the execution manager lazily
    async fn ensure_execution_manager(&self) -> SageResult<()> {
        let mut manager = self.execution_manager.lock().await;
        if manager.is_none() {
            *manager = Some(ReactiveExecutionManager::new(self.config.clone())?);
        }
        Ok(())
    }

    /// Send an event to the UI
    fn send_event(&self, event: IpcEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Handle a chat request
    async fn handle_chat(&self, params: ChatParams) -> SageResult<()> {
        let request_id = params.request_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let start_time = Instant::now();

        // Acknowledge the request
        self.send_event(IpcEvent::Ack {
            request_id: request_id.clone(),
        });

        // Signal LLM thinking
        self.send_event(IpcEvent::LlmThinking {
            request_id: request_id.clone(),
        });

        // Ensure execution manager is initialized
        self.ensure_execution_manager().await?;

        // Execute the chat
        let mut manager = self.execution_manager.lock().await;
        let manager = manager
            .as_mut()
            .ok_or_else(|| SageError::agent("Execution manager not initialized".to_string()))?;

        match manager.interactive_mode(&params.message).await {
            Ok(response) => {
                // Send tool events
                for (i, tool_call) in response.tool_calls.iter().enumerate() {
                    let tool_id = format!("{}-tool-{}", request_id, i);

                    // Tool started
                    self.send_event(IpcEvent::ToolStarted {
                        request_id: request_id.clone(),
                        tool_id: tool_id.clone(),
                        tool_name: tool_call.name.clone(),
                        args: Some(
                            serde_json::to_value(&tool_call.arguments).unwrap_or_default(),
                        ),
                    });

                    // Tool completed (with result if available)
                    if let Some(result) = response.tool_results.get(i) {
                        self.send_event(IpcEvent::ToolCompleted {
                            request_id: request_id.clone(),
                            tool_id: tool_id.clone(),
                            success: result.success,
                            output: result.output.clone(),
                            error: result.error.clone(),
                            duration_ms: result
                                .metadata
                                .get("duration_ms")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0),
                        });
                    }
                }

                // Send LLM done event
                self.send_event(IpcEvent::LlmDone {
                    request_id: request_id.clone(),
                    content: response.content.clone(),
                    tool_calls: response
                        .tool_calls
                        .iter()
                        .map(|tc| ToolCallInfo {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            args: tc.arguments.clone(),
                        })
                        .collect(),
                });

                // Send final completion event
                self.send_event(IpcEvent::ChatCompleted {
                    request_id,
                    content: response.content,
                    completed: response.completed,
                    tool_results: response
                        .tool_results
                        .iter()
                        .enumerate()
                        .map(|(i, r)| ToolResultInfo {
                            tool_id: format!("tool-{}", i),
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
                        .collect(),
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
        self.ensure_execution_manager().await?;

        let manager = self.execution_manager.lock().await;
        let tools: Vec<ToolInfo> = if let Some(mgr) = manager.as_ref() {
            // Get tool schemas from the agent
            let schemas = mgr.get_tool_schemas();
            schemas
                .iter()
                .map(|schema| schema_to_tool_info(schema))
                .collect()
        } else {
            Vec::new()
        };

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
                // TODO: Implement task cancellation
                self.send_event(IpcEvent::Error {
                    request_id: Some(task_id),
                    code: "not_implemented".to_string(),
                    message: "Task cancellation not yet implemented".to_string(),
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

    /// Get tool schemas (delegate to execution manager)
    #[allow(dead_code)]
    pub async fn get_tool_schemas(&self) -> Vec<ToolSchema> {
        if self.ensure_execution_manager().await.is_ok() {
            let manager = self.execution_manager.lock().await;
            if let Some(mgr) = manager.as_ref() {
                return mgr.get_tool_schemas();
            }
        }
        Vec::new()
    }
}

/// Convert a ToolSchema to ToolInfo for IPC protocol
fn schema_to_tool_info(schema: &ToolSchema) -> ToolInfo {
    // The schema.parameters is a JSON Value with JSON Schema format
    // We'll extract parameter info from it
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
            Ok(Some(line)) if !line.trim().is_empty() => {
                match IpcRequest::from_json_line(&line) {
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
                }
            }
            Ok(Some(_)) => continue, // Empty line
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

    let config = if std::path::Path::new(config_path).exists() {
        let content = std::fs::read_to_string(config_path)?;
        serde_json::from_str(&content)?
    } else {
        Config::default()
    };

    run_ipc_server(config).await
}
