//! UI Backend Interface - Gemini CLI style architecture

use chrono::Utc;
use sage_core::error::SageResult;
use sage_sdk::SageAgentSDK;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub verbose: bool,
}

/// UI Backend - similar to Gemini CLI's core package
pub struct SageUIBackend {
    sdk: Arc<Mutex<Option<SageAgentSDK>>>,
}

impl SageUIBackend {
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

    pub async fn chat(&self, request: ChatRequest) -> SageResult<ChatResponse> {
        self.get_or_create_sdk(&request.config_file).await?;

        let sdk_guard = self.sdk.lock().await;
        let sdk = sdk_guard.as_ref().unwrap();

        // Execute the task
        match sdk.run(&request.message).await {
            Ok(result) => {
                // Extract the response content from the final result or last step
                let response_content =
                    result
                        .final_result()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            // If no final result, try to get content from the last step
                            result
                                .execution()
                                .steps
                                .last()
                                .and_then(|step| step.llm_response.as_ref())
                                .map(|response| response.content.clone())
                                .unwrap_or_else(|| "No response from agent".to_string())
                        });

                // Convert tool execution results to UI format
                let tool_calls = result
                    .tool_results()
                    .iter()
                    .enumerate()
                    .map(|(i, tool_result)| ToolCallStatus {
                        id: (i + 1).to_string(),
                        name: tool_result.tool_name.clone(),
                        args: serde_json::json!({}), // TODO: Extract actual args from tool calls
                        status: if tool_result.success {
                            "completed".to_string()
                        } else {
                            "failed".to_string()
                        },
                        start_time: Some(Utc::now().timestamp_millis() as u64 - 2000), // Mock timing
                        end_time: Some(Utc::now().timestamp_millis() as u64),
                        result: tool_result.output.clone(),
                        error: tool_result.error.clone(),
                    })
                    .collect();

                Ok(ChatResponse {
                    role: "assistant".to_string(),
                    content: response_content,
                    timestamp: Utc::now().to_rfc3339(),
                    success: true,
                    error: None,
                    tool_calls,
                })
            }
            Err(e) => Ok(ChatResponse {
                role: "assistant".to_string(),
                content: format!("Error: {}", e),
                timestamp: Utc::now().to_rfc3339(),
                success: false,
                error: Some(e.to_string()),
                tool_calls: vec![],
            }),
        }
    }

    pub async fn get_config(&self, config_file: &str) -> SageResult<ConfigInfo> {
        // Load config and return basic info
        let config = sage_core::config::loader::load_config_from_file(config_file)?;

        // Get model parameters from the default provider
        let default_params = config
            .model_providers
            .get(&config.default_provider)
            .cloned()
            .unwrap_or_default();

        Ok(ConfigInfo {
            model: default_params.model,
            temperature: default_params.temperature.unwrap_or(0.7),
            max_tokens: default_params.max_tokens,
            verbose: config.logging.level == "debug",
        })
    }

    pub async fn list_tools(&self) -> SageResult<Vec<String>> {
        // Return list of available tools
        Ok(vec![
            "file_read".to_string(),
            "file_write".to_string(),
            "directory_list".to_string(),
            "grep_search".to_string(),
            "shell_command".to_string(),
            "web_search".to_string(),
        ])
    }
}

use std::sync::OnceLock;

/// Global backend instance for UI communication
static GLOBAL_BACKEND: OnceLock<Arc<SageUIBackend>> = OnceLock::new();

pub fn get_global_backend() -> Arc<SageUIBackend> {
    GLOBAL_BACKEND
        .get_or_init(|| Arc::new(SageUIBackend::new()))
        .clone()
}

/// Initialize the global backend for UI communication
pub fn init_ui_backend() {
    let _ = get_global_backend();
}
