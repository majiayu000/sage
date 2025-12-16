//! Cloud Provider Tool
//!
//! This tool provides cloud provider management for AWS, Azure, and GCP including:
//! - Resource management
//! - Service configuration
//! - Cost monitoring
//! - Security management

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info};

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Cloud provider management tool
#[derive(Debug, Clone)]
pub struct CloudTool {
    name: String,
    description: String,
}

impl CloudTool {
    /// Create a new cloud tool
    pub fn new() -> Self {
        Self {
            name: "cloud".to_string(),
            description:
                "Multi-cloud provider management for AWS, Azure, and GCP resources and services"
                    .to_string(),
        }
    }

    /// Execute AWS CLI command
    async fn execute_aws_cli(&self, args: &[&str]) -> Result<String, ToolError> {
        let mut cmd = Command::new("aws");
        cmd.args(args);

        debug!("Executing AWS CLI command: aws {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute AWS CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "AWS CLI command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Execute Azure CLI command
    async fn execute_azure_cli(&self, args: &[&str]) -> Result<String, ToolError> {
        let mut cmd = Command::new("az");
        cmd.args(args);

        debug!("Executing Azure CLI command: az {}", args.join(" "));

        let output = cmd.output().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to execute Azure CLI: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "Azure CLI command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Execute GCP CLI command
    async fn execute_gcp_cli(&self, args: &[&str]) -> Result<String, ToolError> {
        let mut cmd = Command::new("gcloud");
        cmd.args(args);

        debug!("Executing GCP CLI command: gcloud {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute GCP CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "GCP CLI command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Manage AWS resources
    async fn manage_aws(
        &self,
        service: &str,
        action: &str,
        resource_name: Option<&str>,
    ) -> Result<String, ToolError> {
        match service {
            "ec2" => {
                match action {
                    "list" => {
                        let result = self
                            .execute_aws_cli(&["ec2", "describe-instances", "--output", "table"])
                            .await?;
                        Ok(format!("AWS EC2 Instances:\n{}", result))
                    }
                    "create" => {
                        let name = resource_name.unwrap_or("sage-instance");
                        let result = self
                            .execute_aws_cli(&[
                                "ec2",
                                "run-instances",
                                "--image-id",
                                "ami-0abcdef1234567890", // Example AMI ID
                                "--instance-type",
                                "t2.micro",
                                "--tag-specifications",
                                &format!(
                                    "ResourceType=instance,Tags=[{{Key=Name,Value={}}}]",
                                    name
                                ),
                            ])
                            .await?;
                        Ok(format!("Created AWS EC2 instance '{}':\n{}", name, result))
                    }
                    _ => Err(ToolError::InvalidArguments(format!(
                        "Unknown EC2 action: {}",
                        action
                    ))),
                }
            }
            "s3" => match action {
                "list" => {
                    let result = self.execute_aws_cli(&["s3", "ls"]).await?;
                    Ok(format!("AWS S3 Buckets:\n{}", result))
                }
                "create" => {
                    let name = resource_name.unwrap_or("sage-bucket");
                    let result = self
                        .execute_aws_cli(&["s3", "mb", &format!("s3://{}", name)])
                        .await?;
                    Ok(format!("Created AWS S3 bucket '{}':\n{}", name, result))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown S3 action: {}",
                    action
                ))),
            },
            "lambda" => match action {
                "list" => {
                    let result = self
                        .execute_aws_cli(&["lambda", "list-functions", "--output", "table"])
                        .await?;
                    Ok(format!("AWS Lambda Functions:\n{}", result))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown Lambda action: {}",
                    action
                ))),
            },
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown AWS service: {}",
                service
            ))),
        }
    }

    /// Manage Azure resources
    async fn manage_azure(
        &self,
        service: &str,
        action: &str,
        resource_name: Option<&str>,
    ) -> Result<String, ToolError> {
        match service {
            "vm" => match action {
                "list" => {
                    let result = self
                        .execute_azure_cli(&["vm", "list", "--output", "table"])
                        .await?;
                    Ok(format!("Azure Virtual Machines:\n{}", result))
                }
                "create" => {
                    let name = resource_name.unwrap_or("sage-vm");
                    let result = self
                        .execute_azure_cli(&[
                            "vm",
                            "create",
                            "--resource-group",
                            "sage-rg",
                            "--name",
                            name,
                            "--image",
                            "Ubuntu2204",
                            "--admin-username",
                            "azureuser",
                        ])
                        .await?;
                    Ok(format!("Created Azure VM '{}':\n{}", name, result))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown VM action: {}",
                    action
                ))),
            },
            "storage" => match action {
                "list" => {
                    let result = self
                        .execute_azure_cli(&["storage", "account", "list", "--output", "table"])
                        .await?;
                    Ok(format!("Azure Storage Accounts:\n{}", result))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown Storage action: {}",
                    action
                ))),
            },
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown Azure service: {}",
                service
            ))),
        }
    }

    /// Manage GCP resources
    async fn manage_gcp(
        &self,
        service: &str,
        action: &str,
        resource_name: Option<&str>,
    ) -> Result<String, ToolError> {
        match service {
            "compute" => match action {
                "list" => {
                    let result = self
                        .execute_gcp_cli(&["compute", "instances", "list"])
                        .await?;
                    Ok(format!("GCP Compute Instances:\n{}", result))
                }
                "create" => {
                    let name = resource_name.unwrap_or("sage-instance");
                    let result = self
                        .execute_gcp_cli(&[
                            "compute",
                            "instances",
                            "create",
                            name,
                            "--machine-type",
                            "e2-micro",
                            "--zone",
                            "us-central1-a",
                            "--image-family",
                            "debian-11",
                            "--image-project",
                            "debian-cloud",
                        ])
                        .await?;
                    Ok(format!(
                        "Created GCP Compute instance '{}':\n{}",
                        name, result
                    ))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown Compute action: {}",
                    action
                ))),
            },
            "storage" => match action {
                "list" => {
                    let result = self
                        .execute_gcp_cli(&["storage", "buckets", "list"])
                        .await?;
                    Ok(format!("GCP Storage Buckets:\n{}", result))
                }
                "create" => {
                    let name = resource_name.unwrap_or("sage-bucket");
                    let result = self
                        .execute_gcp_cli(&[
                            "storage",
                            "buckets",
                            "create",
                            &format!("gs://{}", name),
                        ])
                        .await?;
                    Ok(format!(
                        "Created GCP Storage bucket '{}':\n{}",
                        name, result
                    ))
                }
                _ => Err(ToolError::InvalidArguments(format!(
                    "Unknown Storage action: {}",
                    action
                ))),
            },
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown GCP service: {}",
                service
            ))),
        }
    }

    /// Get cloud cost information
    async fn get_cost_info(&self, provider: &str) -> Result<String, ToolError> {
        match provider {
            "aws" => {
                let result = self
                    .execute_aws_cli(&[
                        "ce",
                        "get-cost-and-usage",
                        "--time-period",
                        "Start=2024-01-01,End=2024-01-31",
                        "--granularity",
                        "MONTHLY",
                        "--metrics",
                        "BlendedCost",
                    ])
                    .await?;
                Ok(format!("AWS Cost Information:\n{}", result))
            }
            "azure" => {
                let result = self
                    .execute_azure_cli(&["consumption", "usage", "list"])
                    .await?;
                Ok(format!("Azure Cost Information:\n{}", result))
            }
            "gcp" => {
                let result = self
                    .execute_gcp_cli(&["billing", "budgets", "list"])
                    .await?;
                Ok(format!("GCP Billing Information:\n{}", result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown provider: {}",
                provider
            ))),
        }
    }
}

impl Default for CloudTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CloudTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![
                ToolParameter::string("provider", "Cloud provider (aws, azure, gcp)"),
                ToolParameter::string("command", "Command type (manage, cost)"),
                ToolParameter::optional_string(
                    "service",
                    "Cloud service (ec2, s3, lambda, vm, storage, compute)",
                ),
                ToolParameter::optional_string(
                    "action",
                    "Action to perform (list, create, delete, etc.)",
                ),
                ToolParameter::optional_string("resource_name", "Name of the resource"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let provider = call.get_string("provider").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'provider' parameter".to_string())
        })?;

        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        info!(
            "Executing cloud command: {} on provider: {}",
            command, provider
        );

        let result = match command.as_str() {
            "manage" => {
                let service = call.get_string("service").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'service' parameter for manage command".to_string(),
                    )
                })?;
                let action = call.get_string("action").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'action' parameter for manage command".to_string(),
                    )
                })?;
                let resource_name = call.get_string("resource_name");

                match provider.as_str() {
                    "aws" => {
                        self.manage_aws(&service, &action, resource_name.as_deref())
                            .await?
                    }
                    "azure" => {
                        self.manage_azure(&service, &action, resource_name.as_deref())
                            .await?
                    }
                    "gcp" => {
                        self.manage_gcp(&service, &action, resource_name.as_deref())
                            .await?
                    }
                    _ => {
                        return Err(ToolError::InvalidArguments(format!(
                            "Unknown provider: {}",
                            provider
                        )));
                    }
                }
            }
            "cost" => self.get_cost_info(&provider).await?,
            _ => {
                return Err(ToolError::InvalidArguments(format!(
                    "Unknown command: {}",
                    command
                )));
            }
        };

        Ok(ToolResult::success(call.id.clone(), self.name(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cloud_tool_creation() {
        let tool = CloudTool::new();
        assert_eq!(tool.name(), "cloud");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_cloud_tool_schema() {
        let tool = CloudTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "cloud");
        assert!(!schema.description.is_empty());
    }
}
