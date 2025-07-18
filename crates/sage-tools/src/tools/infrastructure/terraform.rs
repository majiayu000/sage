//! Terraform Tool
//!
//! This tool provides Terraform infrastructure management including:
//! - Infrastructure planning and provisioning
//! - State management
//! - Resource configuration
//! - Multi-cloud deployments

use async_trait::async_trait;
use tokio::process::Command;
use tokio::fs;
use tracing::{info, debug};

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Terraform infrastructure management tool
#[derive(Debug, Clone)]
pub struct TerraformTool {
    name: String,
    description: String,
}

impl TerraformTool {
    /// Create a new Terraform tool
    pub fn new() -> Self {
        Self {
            name: "terraform".to_string(),
            description: "Terraform infrastructure as code management for provisioning and managing cloud resources".to_string(),
        }
    }

    /// Execute a terraform command
    async fn execute_terraform(&self, args: &[&str], working_dir: Option<&str>) -> Result<String, ToolError> {
        let mut cmd = Command::new("terraform");
        cmd.args(args);
        
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        debug!("Executing terraform command: terraform {}", args.join(" "));
        
        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute terraform: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !output.status.success() {
            return Err(ToolError::ExecutionFailed(format!("Terraform command failed: {}", stderr)));
        }

        let mut result = stdout.to_string();
        if !stderr.is_empty() {
            result.push_str("\nWarnings/Info:\n");
            result.push_str(&stderr);
        }
        
        Ok(result)
    }

    /// Initialize Terraform in a directory
    async fn init_terraform(&self, working_dir: &str) -> Result<String, ToolError> {
        // Check if directory exists
        if !tokio::fs::metadata(working_dir).await.is_ok() {
            tokio::fs::create_dir_all(working_dir).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create directory: {}", e)))?;
        }

        let result = self.execute_terraform(&["init"], Some(working_dir)).await?;
        Ok(format!("Terraform initialized in {}:\n{}", working_dir, result))
    }

    /// Plan Terraform changes
    async fn plan_terraform(&self, working_dir: &str) -> Result<String, ToolError> {
        let result = self.execute_terraform(&["plan"], Some(working_dir)).await?;
        Ok(format!("Terraform plan for {}:\n{}", working_dir, result))
    }

    /// Apply Terraform changes
    async fn apply_terraform(&self, working_dir: &str, auto_approve: bool) -> Result<String, ToolError> {
        let mut args = vec!["apply"];
        if auto_approve {
            args.push("-auto-approve");
        }
        
        let result = self.execute_terraform(&args, Some(working_dir)).await?;
        Ok(format!("Terraform apply for {}:\n{}", working_dir, result))
    }

    /// Destroy Terraform infrastructure
    async fn destroy_terraform(&self, working_dir: &str, auto_approve: bool) -> Result<String, ToolError> {
        let mut args = vec!["destroy"];
        if auto_approve {
            args.push("-auto-approve");
        }
        
        let result = self.execute_terraform(&args, Some(working_dir)).await?;
        Ok(format!("Terraform destroy for {}:\n{}", working_dir, result))
    }

    /// Show Terraform state
    async fn show_state(&self, working_dir: &str) -> Result<String, ToolError> {
        let result = self.execute_terraform(&["show"], Some(working_dir)).await?;
        Ok(format!("Terraform state for {}:\n{}", working_dir, result))
    }

    /// Generate a basic Terraform configuration
    async fn generate_config(&self, resource_type: &str, working_dir: &str) -> Result<String, ToolError> {
        let config = match resource_type {
            "aws_ec2" => r#"
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-west-2"
}

resource "aws_instance" "example" {
  ami           = "ami-0c55b159cbfafe1d0"  # Amazon Linux 2
  instance_type = "t2.micro"

  tags = {
    Name = "terraform-example"
  }
}

output "instance_ip" {
  value = aws_instance.example.public_ip
}
"#,
            "gcp_vm" => r#"
terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "~> 4.0"
    }
  }
}

provider "google" {
  project = var.project_id
  region  = var.region
}

variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "region" {
  description = "GCP region"
  type        = string
  default     = "us-central1"
}

resource "google_compute_instance" "example" {
  name         = "terraform-example"
  machine_type = "e2-micro"
  zone         = "${var.region}-a"

  boot_disk {
    initialize_params {
      image = "debian-cloud/debian-11"
    }
  }

  network_interface {
    network = "default"
    access_config {
      // Ephemeral public IP
    }
  }

  tags = ["terraform-example"]
}
"#,
            "azure_vm" => r#"
terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
  }
}

provider "azurerm" {
  features {}
}

resource "azurerm_resource_group" "example" {
  name     = "terraform-example-rg"
  location = "West Europe"
}

resource "azurerm_virtual_network" "example" {
  name                = "terraform-example-vnet"
  address_space       = ["10.0.0.0/16"]
  location            = azurerm_resource_group.example.location
  resource_group_name = azurerm_resource_group.example.name
}

resource "azurerm_subnet" "internal" {
  name                 = "internal"
  resource_group_name  = azurerm_resource_group.example.name
  virtual_network_name = azurerm_virtual_network.example.name
  address_prefixes     = ["10.0.2.0/24"]
}
"#,
            "kubernetes" => r#"
terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.0"
    }
  }
}

provider "kubernetes" {
  config_path = "~/.kube/config"
}

resource "kubernetes_namespace" "example" {
  metadata {
    name = "terraform-example"
  }
}

resource "kubernetes_deployment" "example" {
  metadata {
    name      = "terraform-example"
    namespace = kubernetes_namespace.example.metadata[0].name
    labels = {
      App = "TerraformExample"
    }
  }

  spec {
    replicas = 2
    selector {
      match_labels = {
        App = "TerraformExample"
      }
    }
    template {
      metadata {
        labels = {
          App = "TerraformExample"
        }
      }
      spec {
        container {
          image = "nginx:1.21.6"
          name  = "example"

          port {
            container_port = 80
          }

          resources {
            limits = {
              cpu    = "0.5"
              memory = "512Mi"
            }
            requests = {
              cpu    = "250m"
              memory = "50Mi"
            }
          }
        }
      }
    }
  }
}
"#,
            _ => &format!("# Terraform configuration for {}\n# TODO: Add specific configuration", resource_type),
        };

        let config_path = format!("{}/main.tf", working_dir);
        
        // Create directory if it doesn't exist
        if !tokio::fs::metadata(working_dir).await.is_ok() {
            tokio::fs::create_dir_all(working_dir).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create directory: {}", e)))?;
        }
        
        // Write configuration file
        fs::write(&config_path, config).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write config file: {}", e)))?;

        Ok(format!("Generated Terraform configuration for {} at {}", resource_type, config_path))
    }

    /// Validate Terraform configuration
    async fn validate_config(&self, working_dir: &str) -> Result<String, ToolError> {
        let result = self.execute_terraform(&["validate"], Some(working_dir)).await?;
        Ok(format!("Terraform validation for {}:\n{}", working_dir, result))
    }

    /// Format Terraform files
    async fn format_files(&self, working_dir: &str) -> Result<String, ToolError> {
        let result = self.execute_terraform(&["fmt"], Some(working_dir)).await?;
        Ok(format!("Terraform format for {}:\n{}", working_dir, result))
    }
}

impl Default for TerraformTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TerraformTool {
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
                ToolParameter::string("command", "Terraform command (init, plan, apply, destroy, show, generate, validate, format)"),
                ToolParameter::string("working_dir", "Working directory path"),
                ToolParameter::optional_string("resource_type", "Type of resource to generate (aws_ec2, gcp_vm, azure_vm, kubernetes)"),
                ToolParameter::boolean("auto_approve", "Auto-approve apply/destroy operations").optional(),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'command' parameter".to_string()))?;
        
        let working_dir = call.get_string("working_dir")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'working_dir' parameter".to_string()))?;
        
        info!("Executing Terraform command: {} in {}", command, working_dir);
        
        let result = match command.as_str() {
            "init" => {
                self.init_terraform(&working_dir).await?
            },
            "plan" => {
                self.plan_terraform(&working_dir).await?
            },
            "apply" => {
                let auto_approve = call.get_bool("auto_approve").unwrap_or(false);
                self.apply_terraform(&working_dir, auto_approve).await?
            },
            "destroy" => {
                let auto_approve = call.get_bool("auto_approve").unwrap_or(false);
                self.destroy_terraform(&working_dir, auto_approve).await?
            },
            "show" => {
                self.show_state(&working_dir).await?
            },
            "generate" => {
                let resource_type = call.get_string("resource_type")
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'resource_type' parameter for generate".to_string()))?;
                self.generate_config(&resource_type, &working_dir).await?
            },
            "validate" => {
                self.validate_config(&working_dir).await?
            },
            "format" => {
                self.format_files(&working_dir).await?
            },
            _ => return Err(ToolError::InvalidArguments(format!("Unknown command: {}", command))),
        };
        
        Ok(ToolResult::success(call.id.clone(), self.name(), result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_terraform_tool_creation() {
        let tool = TerraformTool::new();
        assert_eq!(tool.name(), "terraform");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_terraform_tool_schema() {
        let tool = TerraformTool::new();
        let schema = tool.schema();
        
        assert_eq!(schema.name, "terraform");
        assert!(!schema.description.is_empty());
    }
}