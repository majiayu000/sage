//! UI Backend Interface - Gemini CLI style architecture

use chrono::Utc;
use sage_core::error::SageResult;
use sage_sdk::SageAgentSdk;
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
#[allow(dead_code)]
pub struct SageUiBackend {
    sdk: Arc<Mutex<Option<SageAgentSdk>>>,
}

#[allow(dead_code)]
impl SageUiBackend {
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

    pub async fn chat(&self, request: ChatRequest) -> SageResult<ChatResponse> {
        self.get_or_create_sdk(&request.config_file).await?;

        let sdk_guard = self.sdk.lock().await;
        // SAFETY: get_or_create_sdk() ensures SDK is initialized before returning Ok
        let sdk = sdk_guard
            .as_ref()
            .expect("SDK must be initialized after get_or_create_sdk");

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
                // Extract from execution steps to get both calls and results with proper timing
                let tool_calls: Vec<ToolCallStatus> = execution
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
        let config = sage_core::config::load_config_from_file(config_file)?;

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
static GLOBAL_BACKEND: OnceLock<Arc<SageUiBackend>> = OnceLock::new();

pub fn get_global_backend() -> Arc<SageUiBackend> {
    GLOBAL_BACKEND
        .get_or_init(|| Arc::new(SageUiBackend::new()))
        .clone()
}

/// Initialize the global backend for UI communication
pub fn init_ui_backend() {
    let _ = get_global_backend();
}
