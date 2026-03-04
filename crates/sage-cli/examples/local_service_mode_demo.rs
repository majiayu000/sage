//! Local service mode demo for external GUI integration.
//!
//! Protocol: JSON lines over TCP (default `127.0.0.1:7878`).
//!
//! Example commands:
//! - `{"cmd":"ping"}`
//! - `{"cmd":"get_state"}`
//! - `{"cmd":"start_task","task":"Explain this repository"}`
//! - `{"cmd":"cancel_task"}`
//! - `{"cmd":"switch_model","model":"claude-sonnet-4-20250514"}`
//! - `{"cmd":"respond_input","response":{...InputResponseDto...}}`

use sage_core::InputResponseDto;
use sage_core::agent::{ExecutionMode, ExecutionOptions, UnifiedExecutor};
use sage_core::config::{load_config, load_config_from_file};
use sage_core::error::SageResult;
use sage_core::mcp::{
    build_mcp_registry_from_config, clear_active_mcp_registry, set_active_mcp_registry,
};
use sage_core::session::JsonlSessionStorage;
use sage_core::ui::{AppStateDto, ExternalUiRuntime};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum ServiceCommand {
    Ping,
    GetState,
    StartTask { task: String },
    CancelTask,
    SwitchModel { model: String },
    RespondInput { response: InputResponseDto },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServiceEvent {
    Ready {
        message: String,
    },
    CommandResult {
        cmd: String,
        ok: bool,
        message: String,
    },
    State {
        state: AppStateDto,
    },
    InputRequest {
        request: sage_core::input::InputRequestDto,
    },
    TaskOutcome {
        success: bool,
        status: String,
        final_result: Option<String>,
    },
    Error {
        message: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());
    let config_file = std::env::args().nth(2);

    let runtime = build_runtime(config_file, None, None).await?;

    let listener = TcpListener::bind(&addr).await?;
    println!("Service mode demo listening on {}", addr);

    loop {
        let (socket, peer) = listener.accept().await?;
        println!("Client connected: {}", peer);
        if let Err(err) = handle_client(socket, runtime.clone()).await {
            eprintln!("Client session error: {}", err);
        }
        println!("Client disconnected: {}", peer);
    }
}

async fn build_runtime(
    config_file: Option<String>,
    working_dir: Option<PathBuf>,
    max_steps: Option<u32>,
) -> SageResult<ExternalUiRuntime> {
    let config = match config_file {
        Some(path) if std::path::Path::new(&path).exists() => load_config_from_file(&path)?,
        _ => load_config()?,
    };

    let resolved_working_dir = working_dir
        .or_else(|| config.working_directory.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut options = ExecutionOptions::default()
        .with_mode(ExecutionMode::interactive())
        .with_working_directory(&resolved_working_dir);
    if let Some(steps) = max_steps {
        options = options.with_step_limit(steps);
    }

    let mut executor = UnifiedExecutor::with_options(config.clone(), options)?;
    let mut tools = sage_tools::get_default_tools();

    if config.mcp.enabled {
        match build_mcp_registry_from_config(&config).await {
            Ok(mcp_registry) => {
                let mcp_registry = Arc::new(mcp_registry);
                set_active_mcp_registry(Arc::clone(&mcp_registry));
                let mcp_tools = mcp_registry.as_tools().await;
                if !mcp_tools.is_empty() {
                    tools.extend(mcp_tools);
                }
            }
            Err(err) => {
                clear_active_mcp_registry();
                eprintln!("Failed to load MCP tools: {}", err);
            }
        }
    } else {
        clear_active_mcp_registry();
    }

    executor.register_tools(tools);
    if let Err(err) = executor.init_subagent_support() {
        eprintln!("Failed to initialize subagent support: {}", err);
    }

    let storage = Arc::new(JsonlSessionStorage::default_path()?);
    executor.set_jsonl_storage(storage);
    if let Err(err) = executor.enable_session_recording().await {
        eprintln!("Failed to enable session recording: {}", err);
    }

    Ok(ExternalUiRuntime::new(executor))
}

async fn handle_client(socket: TcpStream, runtime: ExternalUiRuntime) -> SageResult<()> {
    let (read_half, mut write_half) = socket.into_split();
    let mut lines = BufReader::new(read_half).lines();

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<ServiceEvent>();
    let writer_task = tokio::spawn(async move {
        while let Some(event) = out_rx.recv().await {
            let serialized = serde_json::to_string(&event).unwrap_or_else(|_| {
                "{\"type\":\"error\",\"message\":\"failed to serialize\"}".to_string()
            });
            if write_half.write_all(serialized.as_bytes()).await.is_err() {
                break;
            }
            if write_half.write_all(b"\n").await.is_err() {
                break;
            }
            if write_half.flush().await.is_err() {
                break;
            }
        }
    });

    let _ = out_tx.send(ServiceEvent::Ready {
        message: "sage service demo connected".to_string(),
    });

    // Push state updates continuously.
    {
        let mut state_rx = runtime.subscribe_state();
        let state_tx = out_tx.clone();
        tokio::spawn(async move {
            loop {
                if state_rx.changed().await.is_err() {
                    break;
                }
                let snapshot = AppStateDto::from(state_rx.borrow().clone());
                if state_tx
                    .send(ServiceEvent::State { state: snapshot })
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    // Push input requests from executor.
    {
        let input_runtime = runtime.clone();
        let input_tx = out_tx.clone();
        tokio::spawn(async move {
            while let Some(request) = input_runtime.recv_input_request().await {
                if input_tx
                    .send(ServiceEvent::InputRequest { request })
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    // Push completed outcomes.
    {
        let outcome_runtime = runtime.clone();
        let outcome_tx = out_tx.clone();
        tokio::spawn(async move {
            loop {
                if let Some(outcome_result) = outcome_runtime.take_finished_outcome().await {
                    match outcome_result {
                        Ok(outcome) => {
                            let _ = outcome_tx.send(ServiceEvent::TaskOutcome {
                                success: outcome.is_success(),
                                status: outcome.status_message().to_string(),
                                final_result: outcome.execution().final_result.clone(),
                            });
                        }
                        Err(err) => {
                            let _ = outcome_tx.send(ServiceEvent::Error {
                                message: format!("Task failed: {}", err),
                            });
                        }
                    }
                }
                sleep(Duration::from_millis(200)).await;
            }
        });
    }

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let parsed: Result<ServiceCommand, _> = serde_json::from_str(&line);
        let command = match parsed {
            Ok(cmd) => cmd,
            Err(err) => {
                let _ = out_tx.send(ServiceEvent::Error {
                    message: format!("Invalid command JSON: {}", err),
                });
                continue;
            }
        };

        match command {
            ServiceCommand::Ping => {
                let _ = out_tx.send(ServiceEvent::CommandResult {
                    cmd: "ping".to_string(),
                    ok: true,
                    message: "pong".to_string(),
                });
            }
            ServiceCommand::GetState => {
                let snapshot = AppStateDto::from(runtime.state_snapshot());
                let _ = out_tx.send(ServiceEvent::State { state: snapshot });
            }
            ServiceCommand::StartTask { task } => {
                let result = runtime.start_task(task).await;
                let (ok, message) = match result {
                    Ok(_) => (true, "task started".to_string()),
                    Err(err) => (false, err.to_string()),
                };
                let _ = out_tx.send(ServiceEvent::CommandResult {
                    cmd: "start_task".to_string(),
                    ok,
                    message,
                });
            }
            ServiceCommand::CancelTask => {
                runtime.cancel_task();
                let _ = out_tx.send(ServiceEvent::CommandResult {
                    cmd: "cancel_task".to_string(),
                    ok: true,
                    message: "cancel signal sent".to_string(),
                });
            }
            ServiceCommand::SwitchModel { model } => {
                let result = runtime.switch_model(&model).await;
                let (ok, message) = match result {
                    Ok(new_model) => (true, format!("switched to {}", new_model)),
                    Err(err) => (false, err.to_string()),
                };
                let _ = out_tx.send(ServiceEvent::CommandResult {
                    cmd: "switch_model".to_string(),
                    ok,
                    message,
                });
            }
            ServiceCommand::RespondInput { response } => {
                let result = runtime.respond_input(response).await;
                let (ok, message) = match result {
                    Ok(_) => (true, "input response accepted".to_string()),
                    Err(err) => (false, err.to_string()),
                };
                let _ = out_tx.send(ServiceEvent::CommandResult {
                    cmd: "respond_input".to_string(),
                    ok,
                    message,
                });
            }
        }
    }

    // Drop sender so writer task exits.
    drop(out_tx);
    let _ = writer_task.await;
    Ok(())
}
