# Terraform Infrastructure as Code Tool

The Terraform tool provides comprehensive infrastructure management capabilities for provisioning and managing cloud resources across multiple providers.

## Overview

- **Tool Name**: `terraform`
- **Purpose**: Terraform infrastructure as code management for provisioning and managing cloud resources
- **Location**: `crates/sage-tools/src/tools/infrastructure/terraform.rs`

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | Terraform command (init, plan, apply, destroy, show, generate, validate, format) |
| `working_dir` | string | Yes | Working directory path |
| `resource_type` | string | No | Type of resource to generate (aws_ec2, gcp_vm, azure_vm, kubernetes) |
| `auto_approve` | boolean | No | Auto-approve apply/destroy operations |

## Supported Commands

### Initialize
Initialize Terraform in a directory:
```json
{
  "command": "init",
  "working_dir": "/path/to/terraform/project"
}
```

### Plan
Preview infrastructure changes:
```json
{
  "command": "plan",
  "working_dir": "/path/to/terraform/project"
}
```

### Apply
Create or update infrastructure:
```json
{
  "command": "apply",
  "working_dir": "/path/to/terraform/project",
  "auto_approve": true
}
```

### Destroy
Remove infrastructure:
```json
{
  "command": "destroy",
  "working_dir": "/path/to/terraform/project",
  "auto_approve": false
}
```

### Show State
Display current infrastructure state:
```json
{
  "command": "show",
  "working_dir": "/path/to/terraform/project"
}
```

### Generate Configuration
Create infrastructure configuration templates:
```json
{
  "command": "generate",
  "working_dir": "/path/to/terraform/project",
  "resource_type": "aws_ec2"
}
```

### Validate
Validate Terraform configuration:
```json
{
  "command": "validate",
  "working_dir": "/path/to/terraform/project"
}
```

### Format
Format Terraform files:
```json
{
  "command": "format",
  "working_dir": "/path/to/terraform/project"
}
```

## Resource Templates

### AWS EC2 Instance
```hcl
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
  ami           = "ami-0c55b159cbfafe1d0"
  instance_type = "t2.micro"

  tags = {
    Name = "terraform-example"
  }
}

output "instance_ip" {
  value = aws_instance.example.public_ip
}
```

### GCP Compute Instance
```hcl
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
```

### Azure Virtual Machine
```hcl
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
```

### Kubernetes Resources
```hcl
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
```

## Usage Examples

### Basic Infrastructure Workflow
```rust
use sage_tools::TerraformTool;

let terraform = TerraformTool::new();

// Generate AWS EC2 configuration
let generate_call = ToolCall::new("1", "terraform", json!({
    "command": "generate",
    "working_dir": "/tmp/terraform-aws",
    "resource_type": "aws_ec2"
}));
terraform.execute(&generate_call).await?;

// Initialize Terraform
let init_call = ToolCall::new("2", "terraform", json!({
    "command": "init",
    "working_dir": "/tmp/terraform-aws"
}));
terraform.execute(&init_call).await?;

// Plan changes
let plan_call = ToolCall::new("3", "terraform", json!({
    "command": "plan",
    "working_dir": "/tmp/terraform-aws"
}));
let plan_result = terraform.execute(&plan_call).await?;

// Apply infrastructure
let apply_call = ToolCall::new("4", "terraform", json!({
    "command": "apply",
    "working_dir": "/tmp/terraform-aws",
    "auto_approve": true
}));
terraform.execute(&apply_call).await?;
```

### Multi-Environment Management
```rust
// Development environment
let dev_init = ToolCall::new("1", "terraform", json!({
    "command": "init",
    "working_dir": "/infrastructure/environments/dev"
}));
terraform.execute(&dev_init).await?;

let dev_apply = ToolCall::new("2", "terraform", json!({
    "command": "apply",
    "working_dir": "/infrastructure/environments/dev",
    "auto_approve": true
}));
terraform.execute(&dev_apply).await?;

// Production environment
let prod_init = ToolCall::new("3", "terraform", json!({
    "command": "init",
    "working_dir": "/infrastructure/environments/prod"
}));
terraform.execute(&prod_init).await?;

let prod_plan = ToolCall::new("4", "terraform", json!({
    "command": "plan",
    "working_dir": "/infrastructure/environments/prod"
}));
let prod_plan_result = terraform.execute(&prod_plan).await?;
```

### Infrastructure Validation
```rust
// Validate configuration
let validate_call = ToolCall::new("1", "terraform", json!({
    "command": "validate",
    "working_dir": "/infrastructure/modules/vpc"
}));
let validation = terraform.execute(&validate_call).await?;

// Format code
let format_call = ToolCall::new("2", "terraform", json!({
    "command": "format",
    "working_dir": "/infrastructure/modules/vpc"
}));
terraform.execute(&format_call).await?;

// Show current state
let show_call = ToolCall::new("3", "terraform", json!({
    "command": "show",
    "working_dir": "/infrastructure/modules/vpc"
}));
let state = terraform.execute(&show_call).await?;
```

## Configuration Management

### Directory Structure
```
infrastructure/
├── environments/
│   ├── dev/
│   │   ├── main.tf
│   │   ├── variables.tf
│   │   └── terraform.tfvars
│   ├── staging/
│   └── prod/
├── modules/
│   ├── vpc/
│   ├── security-groups/
│   └── databases/
└── shared/
    ├── providers.tf
    └── variables.tf
```

### Environment Variables
```bash
# AWS Configuration
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_DEFAULT_REGION="us-west-2"

# Azure Configuration
export ARM_CLIENT_ID="your-client-id"
export ARM_CLIENT_SECRET="your-client-secret"
export ARM_SUBSCRIPTION_ID="your-subscription-id"
export ARM_TENANT_ID="your-tenant-id"

# GCP Configuration
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
export GOOGLE_PROJECT="your-project-id"
```

## Best Practices

### Project Organization
1. **Module Structure**: Use modules for reusable components
2. **Environment Separation**: Separate state files for each environment
3. **Version Control**: Store Terraform code in version control
4. **State Management**: Use remote state backends (S3, GCS, Azure Storage)

### Security
1. **Credential Management**: Use environment variables or credential files
2. **State Encryption**: Enable encryption for state files
3. **Access Control**: Implement proper IAM policies
4. **Secret Management**: Use cloud provider secret services

### Workflow
1. **Plan Before Apply**: Always run plan before apply
2. **Code Review**: Review Terraform changes in pull requests
3. **Automated Testing**: Test infrastructure code with tools like Terratest
4. **Gradual Rollouts**: Apply changes incrementally

## Advanced Examples

### Multi-Cloud Deployment
```rust
async fn deploy_multi_cloud() -> Result<(), Box<dyn std::error::Error>> {
    let terraform = TerraformTool::new();
    
    // Deploy to AWS
    terraform.execute(&ToolCall::new("1", "terraform", json!({
        "command": "apply",
        "working_dir": "/infrastructure/aws",
        "auto_approve": true
    }))).await?;
    
    // Deploy to GCP
    terraform.execute(&ToolCall::new("2", "terraform", json!({
        "command": "apply",
        "working_dir": "/infrastructure/gcp",
        "auto_approve": true
    }))).await?;
    
    // Deploy to Azure
    terraform.execute(&ToolCall::new("3", "terraform", json!({
        "command": "apply",
        "working_dir": "/infrastructure/azure",
        "auto_approve": true
    }))).await?;
    
    Ok(())
}
```

### Infrastructure Testing
```rust
async fn test_infrastructure() -> Result<(), Box<dyn std::error::Error>> {
    let terraform = TerraformTool::new();
    
    // Validate all modules
    let modules = ["vpc", "security-groups", "databases"];
    
    for module in modules {
        let validate_result = terraform.execute(&ToolCall::new(
            &format!("validate-{}", module),
            "terraform",
            json!({
                "command": "validate",
                "working_dir": format!("/infrastructure/modules/{}", module)
            })
        )).await?;
        
        if validate_result.content.contains("Error") {
            return Err(format!("Validation failed for module: {}", module).into());
        }
    }
    
    Ok(())
}
```

### Disaster Recovery
```rust
async fn backup_and_restore() -> Result<(), Box<dyn std::error::Error>> {
    let terraform = TerraformTool::new();
    
    // Backup current state
    let state = terraform.execute(&ToolCall::new("1", "terraform", json!({
        "command": "show",
        "working_dir": "/infrastructure/prod"
    }))).await?;
    
    // Store backup
    std::fs::write("/backups/terraform-state.json", state.content)?;
    
    // In case of disaster, restore infrastructure
    terraform.execute(&ToolCall::new("2", "terraform", json!({
        "command": "apply",
        "working_dir": "/infrastructure/prod",
        "auto_approve": true
    }))).await?;
    
    Ok(())
}
```

## Integration

### CI/CD Pipeline
```yaml
# .github/workflows/terraform.yml
name: Terraform

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  terraform:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v2
    
    - name: Terraform Validate
      run: |
        sage-cli tool terraform \
          --command validate \
          --working_dir infrastructure/
    
    - name: Terraform Plan
      run: |
        sage-cli tool terraform \
          --command plan \
          --working_dir infrastructure/
    
    - name: Terraform Apply
      if: github.ref == 'refs/heads/main'
      run: |
        sage-cli tool terraform \
          --command apply \
          --working_dir infrastructure/ \
          --auto_approve true
```

### Monitoring Integration
```rust
// Monitor infrastructure changes
async fn monitor_terraform_state() -> Result<(), Box<dyn std::error::Error>> {
    let terraform = TerraformTool::new();
    
    let current_state = terraform.execute(&ToolCall::new(
        "monitor",
        "terraform",
        json!({
            "command": "show",
            "working_dir": "/infrastructure/prod"
        })
    )).await?;
    
    // Compare with previous state and send alerts if needed
    if state_changed(&current_state.content)? {
        send_alert("Terraform state changed").await?;
    }
    
    Ok(())
}
```