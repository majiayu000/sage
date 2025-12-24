//! JSON-RPC server for UI communication

use jsonrpc_core::{IoHandler, Result as RpcResult, Error as RpcError};
use jsonrpc_derive::rpc;
use jsonrpc_http_server::{ServerBuilder, Server};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;
use sage_core::error::{SageError, SageResult};
use sage_core::llm::messages::LlmMessage;
use sage_sdk::{RunOptions, SageAgentSdk};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub config_file: String,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub provider: String,
    pub model: String,
    pub max_steps: u32,
    pub working_directory: String,
    pub verbose: bool,
}

/// RPC trait defining the API for UI communication
#[rpc]
pub trait SageRpc {
    /// Send a chat message and get AI response
    #[rpc(name = "chat")]
    fn chat(&self, request: ChatRequest) -> RpcResult<ChatResponse>;

    /// Get current configuration
    #[rpc(name = "get_config")]
    fn get_config(&self, config_file: String) -> RpcResult<ConfigInfo>;

    /// Validate configuration
    #[rpc(name = "validate_config")]
    fn validate_config(&self, config_file: String) -> RpcResult<bool>;

    /// List available tools
    #[rpc(name = "list_tools")]
    fn list_tools(&self) -> RpcResult<Vec<String>>;
}

/// Implementation of the RPC service
pub struct SageRpcImpl {
    sdk: Arc<Mutex<Option<SageAgentSdk>>>,
}

impl SageRpcImpl {
    pub fn new() -> Self {
        Self {
            sdk: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_or_create_sdk(&self, config_file: &str) -> SageResult<SageAgentSdk> {
        let mut sdk_guard = self.sdk.lock().await;
        
        if sdk_guard.is_none() {
            // Initialize SDK with config
            let sdk = SageAgentSdk::from_config_file(config_file)?;
            *sdk_guard = Some(sdk);
        }
        
        // SAFETY: We just ensured sdk_guard is Some in the if block above
        Ok(sdk_guard.as_ref().expect("SDK was just initialized").clone())
    }
}

impl SageRpc for SageRpcImpl {
    fn chat(&self, request: ChatRequest) -> RpcResult<ChatResponse> {
        // Use tokio runtime to handle async operations
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RpcError::internal_error_with_data("Failed to create runtime", e.to_string()))?;
        
        rt.block_on(async {
            match self.process_chat_request(request).await {
                Ok(response) => Ok(response),
                Err(e) => Ok(ChatResponse {
                    message: ChatMessage {
                        role: "assistant".to_string(),
                        content: format!("Error: {}", e),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    },
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        })
    }

    fn get_config(&self, config_file: String) -> RpcResult<ConfigInfo> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| RpcError::internal_error_with_data("Failed to create runtime", e.to_string()))?;
        
        rt.block_on(async {
            match self.load_config_info(&config_file).await {
                Ok(config) => Ok(config),
                Err(e) => Err(RpcError::internal_error_with_data("Failed to load config", e.to_string()))
            }
        })
    }

    fn validate_config(&self, config_file: String) -> RpcResult<bool> {
        match SageAgentSdk::from_config_file(&config_file) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn list_tools(&self) -> RpcResult<Vec<String>> {
        // Return available tools
        Ok(vec![
            "file_ops".to_string(),
            "process".to_string(),
            "task_mgmt".to_string(),
            "utils".to_string(),
        ])
    }
}

impl SageRpcImpl {
    async fn process_chat_request(&self, request: ChatRequest) -> SageResult<ChatResponse> {
        let sdk = self.get_or_create_sdk(&request.config_file).await?;
        
        // Create run options
        let mut run_options = RunOptions::default();
        if let Some(work_dir) = request.working_dir {
            run_options.working_directory = Some(work_dir.into());
        }
        
        // Execute the task
        let result = sdk.run(&request.message, run_options).await?;
        
        // Extract the response content
        let response_content = result.messages
            .iter()
            .filter(|msg| matches!(msg, LlmMessage::Assistant { .. }))
            .last()
            .map(|msg| match msg {
                LlmMessage::Assistant { content, .. } => content.clone(),
                _ => "No response".to_string(),
            })
            .unwrap_or_else(|| "No response from agent".to_string());
        
        Ok(ChatResponse {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: response_content,
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
            success: true,
            error: None,
        })
    }

    async fn load_config_info(&self, config_file: &str) -> SageResult<ConfigInfo> {
        let sdk = self.get_or_create_sdk(config_file).await?;
        let config = sdk.get_config();
        
        Ok(ConfigInfo {
            provider: config.llm.provider.clone(),
            model: config.llm.model.clone(),
            max_steps: config.max_steps.unwrap_or(u32::MAX), // No limit by default
            working_directory: config.working_directory
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()),
            verbose: config.verbose.unwrap_or(false),
        })
    }
}

/// Start the RPC server
pub async fn start_rpc_server(port: u16) -> SageResult<Server> {
    let mut io = IoHandler::new();
    let rpc_impl = SageRpcImpl::new();
    io.extend_with(rpc_impl.to_delegate());
    
    // Construct SocketAddr directly to avoid parse().unwrap()
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let server = ServerBuilder::new(io)
        .cors_allow_all()
        .start_http(&addr)
        .map_err(|e| SageError::other(format!("Failed to start RPC server: {}", e)))?;
    
    println!("RPC server started on http://127.0.0.1:{}", port);
    Ok(server)
}
