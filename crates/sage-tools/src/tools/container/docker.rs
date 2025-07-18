//! Docker Tool
//!
//! This tool provides Docker container management operations including:
//! - Container lifecycle management
//! - Image building and management
//! - Volume and network operations
//! - Docker Compose integration
//! - Registry operations

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tokio::process::Command;
use tracing::{info, debug, error};

use sage_core::tools::{Tool, ToolResult};

/// Docker operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerOperation {
    /// List containers
    ListContainers { all: bool },
    /// Run a container
    RunContainer {
        image: String,
        name: Option<String>,
        ports: Option<Vec<String>>,
        volumes: Option<Vec<String>>,
        environment: Option<HashMap<String, String>>,
        detach: bool,
        remove: bool,
        command: Option<String>,
    },
    /// Stop a container
    StopContainer { container: String },
    /// Start a container
    StartContainer { container: String },
    /// Remove a container
    RemoveContainer { container: String, force: bool },
    /// Get container logs
    ContainerLogs { container: String, follow: bool, tail: Option<usize> },
    /// Execute command in container
    Exec { container: String, command: String, interactive: bool },
    /// Inspect container
    InspectContainer { container: String },
    /// List images
    ListImages { all: bool },
    /// Build image
    BuildImage {
        dockerfile_path: String,
        tag: String,
        context: Option<String>,
        build_args: Option<HashMap<String, String>>,
    },
    /// Pull image
    PullImage { image: String },
    /// Push image
    PushImage { image: String },
    /// Remove image
    RemoveImage { image: String, force: bool },
    /// Tag image
    TagImage { source: String, target: String },
    /// List volumes
    ListVolumes,
    /// Create volume
    CreateVolume { name: String },
    /// Remove volume
    RemoveVolume { name: String },
    /// List networks
    ListNetworks,
    /// Create network
    CreateNetwork { name: String, driver: Option<String> },
    /// Remove network
    RemoveNetwork { name: String },
    /// Docker system info
    SystemInfo,
    /// Docker system prune
    SystemPrune { volumes: bool },
    /// Docker Compose up
    ComposeUp { file: Option<String>, detach: bool },
    /// Docker Compose down
    ComposeDown { file: Option<String>, volumes: bool },
    /// Docker Compose logs
    ComposeLogs { file: Option<String>, follow: bool },
}

/// Docker tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerParams {
    /// Docker operation
    pub operation: DockerOperation,
    /// Working directory for Docker commands
    pub working_dir: Option<String>,
}

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

    /// Execute a docker command
    async fn execute_docker_command(&self, args: &[&str], working_dir: Option<&str>) -> Result<String> {
        let mut cmd = Command::new("docker");
        cmd.args(args);
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing docker command: docker {}", args.join(" "));
        
        let output = cmd.output().await
            .with_context(|| format!("Failed to execute docker command: docker {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Docker command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Execute a docker-compose command
    async fn execute_compose_command(&self, args: &[&str], working_dir: Option<&str>) -> Result<String> {
        let mut cmd = Command::new("docker-compose");
        cmd.args(args);
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing docker-compose command: docker-compose {}", args.join(" "));
        
        let output = cmd.output().await
            .with_context(|| format!("Failed to execute docker-compose command: docker-compose {}", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Docker Compose command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Handle container operations
    async fn handle_container_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::ListContainers { all } => {
                let mut args = vec!["ps"];
                if *all {
                    args.push("-a");
                }
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::RunContainer { image, name, ports, volumes, environment, detach, remove, command } => {
                let mut args = vec!["run"];
                
                if *detach {
                    args.push("-d");
                }
                if *remove {
                    args.push("--rm");
                }
                
                if let Some(name) = name {
                    args.extend(vec!["--name", name]);
                }
                
                if let Some(ports) = ports {
                    for port in ports {
                        args.extend(vec!["-p", port]);
                    }
                }
                
                if let Some(volumes) = volumes {
                    for volume in volumes {
                        args.extend(vec!["-v", volume]);
                    }
                }
                
                if let Some(env) = environment {
                    for (key, value) in env {
                        args.extend(vec!["-e", &format!("{}={}", key, value)]);
                    }
                }
                
                args.push(image);
                
                if let Some(cmd) = command {
                    args.extend(cmd.split_whitespace());
                }
                
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::StopContainer { container } => {
                self.execute_docker_command(&["stop", container], working_dir).await
            }
            DockerOperation::StartContainer { container } => {
                self.execute_docker_command(&["start", container], working_dir).await
            }
            DockerOperation::RemoveContainer { container, force } => {
                let mut args = vec!["rm"];
                if *force {
                    args.push("-f");
                }
                args.push(container);
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::ContainerLogs { container, follow, tail } => {
                let mut args = vec!["logs"];
                if *follow {
                    args.push("-f");
                }
                if let Some(tail_lines) = tail {
                    args.extend(vec!["--tail", &tail_lines.to_string()]);
                }
                args.push(container);
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::Exec { container, command, interactive } => {
                let mut args = vec!["exec"];
                if *interactive {
                    args.push("-it");
                }
                args.push(container);
                args.extend(command.split_whitespace());
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::InspectContainer { container } => {
                self.execute_docker_command(&["inspect", container], working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid container operation")),
        }
    }

    /// Handle image operations
    async fn handle_image_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::ListImages { all } => {
                let mut args = vec!["images"];
                if *all {
                    args.push("-a");
                }
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::BuildImage { dockerfile_path, tag, context, build_args } => {
                let mut args = vec!["build"];
                args.extend(vec!["-t", tag]);
                
                if let Some(build_args) = build_args {
                    for (key, value) in build_args {
                        args.extend(vec!["--build-arg", &format!("{}={}", key, value)]);
                    }
                }
                
                args.extend(vec!["-f", dockerfile_path]);
                
                let build_context = context.as_deref().unwrap_or(".");
                args.push(build_context);
                
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::PullImage { image } => {
                self.execute_docker_command(&["pull", image], working_dir).await
            }
            DockerOperation::PushImage { image } => {
                self.execute_docker_command(&["push", image], working_dir).await
            }
            DockerOperation::RemoveImage { image, force } => {
                let mut args = vec!["rmi"];
                if *force {
                    args.push("-f");
                }
                args.push(image);
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::TagImage { source, target } => {
                self.execute_docker_command(&["tag", source, target], working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid image operation")),
        }
    }

    /// Handle volume operations
    async fn handle_volume_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::ListVolumes => {
                self.execute_docker_command(&["volume", "ls"], working_dir).await
            }
            DockerOperation::CreateVolume { name } => {
                self.execute_docker_command(&["volume", "create", name], working_dir).await
            }
            DockerOperation::RemoveVolume { name } => {
                self.execute_docker_command(&["volume", "rm", name], working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid volume operation")),
        }
    }

    /// Handle network operations
    async fn handle_network_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::ListNetworks => {
                self.execute_docker_command(&["network", "ls"], working_dir).await
            }
            DockerOperation::CreateNetwork { name, driver } => {
                let mut args = vec!["network", "create"];
                if let Some(driver) = driver {
                    args.extend(vec!["--driver", driver]);
                }
                args.push(name);
                self.execute_docker_command(&args, working_dir).await
            }
            DockerOperation::RemoveNetwork { name } => {
                self.execute_docker_command(&["network", "rm", name], working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid network operation")),
        }
    }

    /// Handle system operations
    async fn handle_system_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::SystemInfo => {
                self.execute_docker_command(&["system", "info"], working_dir).await
            }
            DockerOperation::SystemPrune { volumes } => {
                let mut args = vec!["system", "prune", "-f"];
                if *volumes {
                    args.push("--volumes");
                }
                self.execute_docker_command(&args, working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid system operation")),
        }
    }

    /// Handle Docker Compose operations
    async fn handle_compose_operation(&self, operation: &DockerOperation, working_dir: Option<&str>) -> Result<String> {
        match operation {
            DockerOperation::ComposeUp { file, detach } => {
                let mut args = vec![];
                if let Some(file) = file {
                    args.extend(vec!["-f", file]);
                }
                args.push("up");
                if *detach {
                    args.push("-d");
                }
                self.execute_compose_command(&args, working_dir).await
            }
            DockerOperation::ComposeDown { file, volumes } => {
                let mut args = vec![];
                if let Some(file) = file {
                    args.extend(vec!["-f", file]);
                }
                args.push("down");
                if *volumes {
                    args.push("--volumes");
                }
                self.execute_compose_command(&args, working_dir).await
            }
            DockerOperation::ComposeLogs { file, follow } => {
                let mut args = vec![];
                if let Some(file) = file {
                    args.extend(vec!["-f", file]);
                }
                args.push("logs");
                if *follow {
                    args.push("-f");
                }
                self.execute_compose_command(&args, working_dir).await
            }
            _ => Err(anyhow::anyhow!("Invalid compose operation")),
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
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "object",
                    "oneOf": [
                        {
                            "properties": {
                                "list_containers": {
                                    "type": "object",
                                    "properties": {
                                        "all": { "type": "boolean", "default": false }
                                    }
                                }
                            },
                            "required": ["list_containers"]
                        },
                        {
                            "properties": {
                                "run_container": {
                                    "type": "object",
                                    "properties": {
                                        "image": { "type": "string" },
                                        "name": { "type": "string" },
                                        "ports": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "volumes": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "environment": {
                                            "type": "object",
                                            "additionalProperties": { "type": "string" }
                                        },
                                        "detach": { "type": "boolean", "default": false },
                                        "remove": { "type": "boolean", "default": false },
                                        "command": { "type": "string" }
                                    },
                                    "required": ["image"]
                                }
                            },
                            "required": ["run_container"]
                        },
                        {
                            "properties": {
                                "stop_container": {
                                    "type": "object",
                                    "properties": {
                                        "container": { "type": "string" }
                                    },
                                    "required": ["container"]
                                }
                            },
                            "required": ["stop_container"]
                        },
                        {
                            "properties": {
                                "build_image": {
                                    "type": "object",
                                    "properties": {
                                        "dockerfile_path": { "type": "string" },
                                        "tag": { "type": "string" },
                                        "context": { "type": "string" },
                                        "build_args": {
                                            "type": "object",
                                            "additionalProperties": { "type": "string" }
                                        }
                                    },
                                    "required": ["dockerfile_path", "tag"]
                                }
                            },
                            "required": ["build_image"]
                        },
                        {
                            "properties": {
                                "system_info": { "type": "null" }
                            },
                            "required": ["system_info"]
                        }
                    ]
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for Docker commands"
                }
            },
            "required": ["operation"],
            "additionalProperties": false
        })
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
                self.handle_container_operation(&params.operation, working_dir).await?
            }
            
            // Image operations
            DockerOperation::ListImages { .. } |
            DockerOperation::BuildImage { .. } |
            DockerOperation::PullImage { .. } |
            DockerOperation::PushImage { .. } |
            DockerOperation::RemoveImage { .. } |
            DockerOperation::TagImage { .. } => {
                self.handle_image_operation(&params.operation, working_dir).await?
            }
            
            // Volume operations
            DockerOperation::ListVolumes |
            DockerOperation::CreateVolume { .. } |
            DockerOperation::RemoveVolume { .. } => {
                self.handle_volume_operation(&params.operation, working_dir).await?
            }
            
            // Network operations
            DockerOperation::ListNetworks |
            DockerOperation::CreateNetwork { .. } |
            DockerOperation::RemoveNetwork { .. } => {
                self.handle_network_operation(&params.operation, working_dir).await?
            }
            
            // System operations
            DockerOperation::SystemInfo |
            DockerOperation::SystemPrune { .. } => {
                self.handle_system_operation(&params.operation, working_dir).await?
            }
            
            // Compose operations
            DockerOperation::ComposeUp { .. } |
            DockerOperation::ComposeDown { .. } |
            DockerOperation::ComposeLogs { .. } => {
                self.handle_compose_operation(&params.operation, working_dir).await?
            }
        };

        let metadata = HashMap::new();
        Ok(ToolResult::new(result, metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_tool_creation() {
        let tool = DockerTool::new();
        assert_eq!(tool.name(), "docker");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_docker_tool_schema() {
        let tool = DockerTool::new();
        let schema = tool.parameters_json_schema();
        
        assert!(schema.is_object());
        assert!(schema["properties"]["operation"].is_object());
    }
}