//! Simple HTTP server for UI communication

use crate::api_types::{ChatRequest, ChatResponse, ToolCallStatus};
use sage_core::error::{SageError, SageResult};
use sage_sdk::SageAgentSdk;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub struct SageHttpServer {
    sdk: Arc<Mutex<Option<SageAgentSdk>>>,
}

impl SageHttpServer {
    pub fn new() -> Self {
        Self {
            sdk: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_or_create_sdk(&self, config_file: &str) -> SageResult<()> {
        let mut sdk_guard = self.sdk.lock().await;
        
        if sdk_guard.is_none() {
            let sdk = SageAgentSdk::with_config_file(config_file)?;
            *sdk_guard = Some(sdk);
        }
        
        Ok(())
    }

    async fn handle_chat(&self, request: ChatRequest) -> Result<impl warp::Reply, warp::Rejection> {
        match self.process_chat_request(request).await {
            Ok(response) => Ok(warp::reply::json(&response)),
            Err(e) => {
                let error_response = ChatResponse {
                    role: "assistant".to_string(),
                    content: format!("Error: {}", e),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    success: false,
                    error: Some(e.to_string()),
                    tool_calls: vec![],
                };
                Ok(warp::reply::json(&error_response))
            }
        }
    }

    async fn process_chat_request(&self, request: ChatRequest) -> SageResult<ChatResponse> {
        self.get_or_create_sdk(&request.config_file).await?;

        let sdk_guard = self.sdk.lock().await;
        // SAFETY: get_or_create_sdk ensures sdk is Some after successful return
        let sdk = sdk_guard.as_ref()
            .ok_or_else(|| SageError::config_error("SDK not initialized"))?;

        // Execute the task
        match sdk.run(&request.message).await {
            Ok(result) => {
                // Extract the response content
                let response_content = result.messages
                    .iter()
                    .filter(|msg| matches!(msg, sage_core::llm::messages::LlmMessage::Assistant { .. }))
                    .last()
                    .map(|msg| match msg {
                        sage_core::llm::messages::LlmMessage::Assistant { content, .. } => content.clone(),
                        _ => "No response".to_string(),
                    })
                    .unwrap_or_else(|| "No response from agent".to_string());

                // Extract actual tool calls from execution steps
                let tool_calls: Vec<ToolCallStatus> = result
                    .execution()
                    .steps
                    .iter()
                    .flat_map(|step| {
                        step.tool_calls.iter().map(move |call| {
                            // Find corresponding result for this tool call
                            let result = step.tool_results
                                .iter()
                                .find(|r| r.call_id == call.id);

                            ToolCallStatus {
                                id: call.id.clone(),
                                name: call.name.clone(),
                                args: serde_json::to_value(&call.arguments).unwrap_or_default(),
                                status: if result.map(|r| r.success).unwrap_or(false) {
                                    "completed".to_string()
                                } else {
                                    "failed".to_string()
                                },
                                start_time: Some(step.started_at.timestamp_millis() as u64),
                                end_time: step.completed_at.map(|t| t.timestamp_millis() as u64),
                                result: result.and_then(|r| r.output.clone()),
                                error: result.and_then(|r| r.error.clone()),
                            }
                        })
                    })
                    .collect();

                Ok(ChatResponse {
                    role: "assistant".to_string(),
                    content: response_content,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    success: true,
                    error: None,
                    tool_calls,
                })
            }
            Err(e) => Ok(ChatResponse {
                role: "assistant".to_string(),
                content: format!("Error: {}", e),
                timestamp: chrono::Utc::now().to_rfc3339(),
                success: false,
                error: Some(e.to_string()),
                tool_calls: vec![],
            })
        }
    }
}

pub async fn start_http_server(port: u16) -> SageResult<()> {
    let server = Arc::new(SageHttpServer::new());
    
    let chat_route = warp::path("chat")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || server.clone()))
        .and_then(|request: ChatRequest, server: Arc<SageHttpServer>| async move {
            server.handle_chat(request).await
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["POST", "GET"]);

    let routes = chat_route.with(cors);

    println!("HTTP server started on http://127.0.0.1:{}", port);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;

    Ok(())
}
