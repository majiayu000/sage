//! Simple HTTP server for UI communication

use std::sync::Arc;
use tokio::sync::Mutex;
use sage_core::error::{SageError, SageResult};
use sage_sdk::SageAgentSDK;
use serde::{Deserialize, Serialize};
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub config_file: String,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub success: bool,
    pub error: Option<String>,
    pub tool_calls: Vec<ToolCallStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStatus {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
    pub status: String,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

pub struct SageHttpServer {
    sdk: Arc<Mutex<Option<SageAgentSDK>>>,
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
            let sdk = SageAgentSDK::with_config_file(config_file)?;
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
        let sdk = sdk_guard.as_ref().unwrap();

        // Execute the task
        match sdk.run(&request.message).await {
            Ok(result) => {
                // Extract the response content
                let response_content = result.messages
                    .iter()
                    .filter(|msg| matches!(msg, sage_core::llm::messages::LLMMessage::Assistant { .. }))
                    .last()
                    .map(|msg| match msg {
                        sage_core::llm::messages::LLMMessage::Assistant { content, .. } => content.clone(),
                        _ => "No response".to_string(),
                    })
                    .unwrap_or_else(|| "No response from agent".to_string());

                // TODO: Extract actual tool calls from result
                let mock_tool_calls = vec![
                    ToolCallStatus {
                        id: "1".to_string(),
                        name: "file_read".to_string(),
                        args: serde_json::json!({"path": "Cargo.toml"}),
                        status: "completed".to_string(),
                        start_time: Some(chrono::Utc::now().timestamp_millis() as u64 - 2000),
                        end_time: Some(chrono::Utc::now().timestamp_millis() as u64),
                        result: Some("File read successfully".to_string()),
                        error: None,
                    }
                ];

                Ok(ChatResponse {
                    role: "assistant".to_string(),
                    content: response_content,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    success: true,
                    error: None,
                    tool_calls: mock_tool_calls,
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
