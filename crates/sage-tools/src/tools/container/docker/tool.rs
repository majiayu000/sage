//! Docker tool implementation

use std::collections::HashMap;
use async_trait::async_trait;
use anyhow::{Result, Context};
use tracing::info;

use sage_core::tools::{Tool, ToolResult};

use super::types::{DockerOperation, DockerParams};
use super::schema::parameters_json_schema;
use super::commands::{
    handle_container_operation,
    handle_image_operation,
    handle_volume_operation,
    handle_network_operation,
    handle_system_operation,
    handle_compose_operation,
};

/// Docker tool for container operations
#[derive(Debug, Clone)]
pub struct DockerTool {
    name: String,
    description: String,
}

impl DockerTool {
    /// Create a new Docker tool
    pub fn new() -> Self {
        Self {
            name: "docker".to_string(),
            description: "Docker container management including lifecycle, images, volumes, networks, and Docker Compose".to_string(),
        }
    }
}

impl Default for DockerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DockerTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        parameters_json_schema()
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let params: DockerParams = serde_json::from_value(params)
            .context("Failed to parse Docker parameters")?;

        let working_dir = params.working_dir.as_deref();

        info!("Executing Docker operation: {:?}", params.operation);

        let result = match &params.operation {
            // Container operations
            DockerOperation::ListContainers { .. } |
            DockerOperation::RunContainer { .. } |
            DockerOperation::StopContainer { .. } |
            DockerOperation::StartContainer { .. } |
            DockerOperation::RemoveContainer { .. } |
            DockerOperation::ContainerLogs { .. } |
            DockerOperation::Exec { .. } |
            DockerOperation::InspectContainer { .. } => {
                handle_container_operation(&params.operation, working_dir).await?
            }

            // Image operations
            DockerOperation::ListImages { .. } |
            DockerOperation::BuildImage { .. } |
            DockerOperation::PullImage { .. } |
            DockerOperation::PushImage { .. } |
            DockerOperation::RemoveImage { .. } |
            DockerOperation::TagImage { .. } => {
                handle_image_operation(&params.operation, working_dir).await?
            }

            // Volume operations
            DockerOperation::ListVolumes |
            DockerOperation::CreateVolume { .. } |
            DockerOperation::RemoveVolume { .. } => {
                handle_volume_operation(&params.operation, working_dir).await?
            }

            // Network operations
            DockerOperation::ListNetworks |
            DockerOperation::CreateNetwork { .. } |
            DockerOperation::RemoveNetwork { .. } => {
                handle_network_operation(&params.operation, working_dir).await?
            }

            // System operations
            DockerOperation::SystemInfo |
            DockerOperation::SystemPrune { .. } => {
                handle_system_operation(&params.operation, working_dir).await?
            }

            // Compose operations
            DockerOperation::ComposeUp { .. } |
            DockerOperation::ComposeDown { .. } |
            DockerOperation::ComposeLogs { .. } => {
                handle_compose_operation(&params.operation, working_dir).await?
            }
        };

        let metadata = HashMap::new();
        Ok(ToolResult::new(result, metadata))
    }
}
