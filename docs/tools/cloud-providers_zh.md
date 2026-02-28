# Cloud Providers Tool

The Cloud Providers tool offers unified management across AWS, Azure, and GCP for resource provisioning, monitoring, and cost management.

## Overview

- **Tool Name**: `cloud`
- **Purpose**: Multi-cloud provider management for AWS, Azure, and GCP resources and services
- **Location**: `crates/sage-tools/src/tools/infrastructure/cloud.rs`

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `provider` | string | Yes | Cloud provider (aws, azure, gcp) |
| `command` | string | Yes | Command type (manage, cost) |
| `service` | string | No | Cloud service (ec2, s3, lambda, vm, storage, compute) |
| `action` | string | No | Action to perform (list, create, delete, etc.) |
| `resource_name` | string | No | Name of the resource |

## Supported Providers

### Amazon Web Services (AWS)

#### Elastic Compute Cloud (EC2)
List EC2 instances:
```json
{
  "provider": "aws",
  "command": "manage",
  "service": "ec2",
  "action": "list"
}
```

Create EC2 instance:
```json
{
  "provider": "aws",
  "command": "manage",
  "service": "ec2",
  "action": "create",
  "resource_name": "my-web-server"
}
```

#### Simple Storage Service (S3)
List S3 buckets:
```json
{
  "provider": "aws",
  "command": "manage",
  "service": "s3",
  "action": "list"
}
```

Create S3 bucket:
```json
{
  "provider": "aws",
  "command": "manage",
  "service": "s3",
  "action": "create",
  "resource_name": "my-app-bucket"
}
```

#### Lambda Functions
List Lambda functions:
```json
{
  "provider": "aws",
  "command": "manage",
  "service": "lambda",
  "action": "list"
}
```

### Microsoft Azure

#### Virtual Machines
List Azure VMs:
```json
{
  "provider": "azure",
  "command": "manage",
  "service": "vm",
  "action": "list"
}
```

Create Azure VM:
```json
{
  "provider": "azure",
  "command": "manage",
  "service": "vm",
  "action": "create",
  "resource_name": "my-azure-vm"
}
```

#### Storage Accounts
List storage accounts:
```json
{
  "provider": "azure",
  "command": "manage",
  "service": "storage",
  "action": "list"
}
```

### Google Cloud Platform (GCP)

#### Compute Engine
List compute instances:
```json
{
  "provider": "gcp",
  "command": "manage",
  "service": "compute",
  "action": "list"
}
```

Create compute instance:
```json
{
  "provider": "gcp",
  "command": "manage",
  "service": "compute",
  "action": "create",
  "resource_name": "my-gcp-instance"
}
```

#### Cloud Storage
List storage buckets:
```json
{
  "provider": "gcp",
  "command": "manage",
  "service": "storage",
  "action": "list"
}
```

Create storage bucket:
```json
{
  "provider": "gcp",
  "command": "manage",
  "service": "storage",
  "action": "create",
  "resource_name": "my-gcp-bucket"
}
```

## Cost Management

### AWS Cost Explorer
Get AWS cost information:
```json
{
  "provider": "aws",
  "command": "cost"
}
```

### Azure Consumption
Get Azure usage and billing:
```json
{
  "provider": "azure",
  "command": "cost"
}
```

### GCP Billing
Get GCP billing information:
```json
{
  "provider": "gcp",
  "command": "cost"
}
```

## Usage Examples

### Multi-Cloud Resource Management
```rust
use sage_tools::CloudTool;

let cloud = CloudTool::new();

// Create resources across multiple providers
let providers = ["aws", "azure", "gcp"];

for provider in providers {
    // List existing resources
    let list_call = ToolCall::new(
        &format!("list-{}", provider),
        "cloud",
        json!({
            "provider": provider,
            "command": "manage",
            "service": match provider {
                "aws" => "ec2",
                "azure" => "vm",
                "gcp" => "compute",
                _ => unreachable!()
            },
            "action": "list"
        })
    );
    
    let resources = cloud.execute(&list_call).await?;
    println!("{} resources: {}", provider, resources.content);
    
    // Create new resource
    let create_call = ToolCall::new(
        &format!("create-{}", provider),
        "cloud",
        json!({
            "provider": provider,
            "command": "manage",
            "service": match provider {
                "aws" => "ec2",
                "azure" => "vm",
                "gcp" => "compute",
                _ => unreachable!()
            },
            "action": "create",
            "resource_name": format!("multi-cloud-instance-{}", provider)
        })
    );
    
    cloud.execute(&create_call).await?;
}
```

### Cost Analysis Across Providers
```rust
async fn analyze_multi_cloud_costs() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    let providers = ["aws", "azure", "gcp"];
    
    for provider in providers {
        let cost_call = ToolCall::new(
            &format!("cost-{}", provider),
            "cloud",
            json!({
                "provider": provider,
                "command": "cost"
            })
        );
        
        let cost_info = cloud.execute(&cost_call).await?;
        println!("{} costs: {}", provider, cost_info.content);
    }
    
    Ok(())
}
```

### Disaster Recovery Setup
```rust
async fn setup_dr_infrastructure() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    
    // Primary region in AWS
    cloud.execute(&ToolCall::new("1", "cloud", json!({
        "provider": "aws",
        "command": "manage",
        "service": "ec2",
        "action": "create",
        "resource_name": "primary-app-server"
    }))).await?;
    
    // Backup region in Azure
    cloud.execute(&ToolCall::new("2", "cloud", json!({
        "provider": "azure",
        "command": "manage",
        "service": "vm",
        "action": "create",
        "resource_name": "backup-app-server"
    }))).await?;
    
    // Data backup in GCP
    cloud.execute(&ToolCall::new("3", "cloud", json!({
        "provider": "gcp",
        "command": "manage",
        "service": "storage",
        "action": "create",
        "resource_name": "dr-backup-bucket"
    }))).await?;
    
    Ok(())
}
```

## Prerequisites

### AWS CLI
```bash
# Install AWS CLI
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install

# Configure credentials
aws configure
```

### Azure CLI
```bash
# Install Azure CLI
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash

# Login to Azure
az login
```

### GCP CLI
```bash
# Install gcloud CLI
curl https://sdk.cloud.google.com | bash
exec -l $SHELL

# Initialize and authenticate
gcloud init
gcloud auth login
```

## Authentication Setup

### AWS Credentials
```bash
# Using AWS credentials file
cat ~/.aws/credentials
[default]
aws_access_key_id = YOUR_ACCESS_KEY
aws_secret_access_key = YOUR_SECRET_KEY

# Using environment variables
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_DEFAULT_REGION=us-west-2
```

### Azure Authentication
```bash
# Service Principal authentication
export ARM_CLIENT_ID="your-client-id"
export ARM_CLIENT_SECRET="your-client-secret"
export ARM_SUBSCRIPTION_ID="your-subscription-id"
export ARM_TENANT_ID="your-tenant-id"
```

### GCP Authentication
```bash
# Service Account authentication
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"

# User authentication
gcloud auth application-default login
```

## Resource Templates

### AWS EC2 Configuration
```json
{
  "ImageId": "ami-0abcdef1234567890",
  "InstanceType": "t2.micro",
  "KeyName": "my-key-pair",
  "SecurityGroups": ["my-security-group"],
  "UserData": "#!/bin/bash\nyum update -y\nyum install -y httpd\nsystemctl start httpd"
}
```

### Azure VM Configuration
```json
{
  "location": "West Europe",
  "properties": {
    "hardwareProfile": {
      "vmSize": "Standard_B1s"
    },
    "osProfile": {
      "computerName": "myVM",
      "adminUsername": "azureuser"
    },
    "storageProfile": {
      "imageReference": {
        "publisher": "Canonical",
        "offer": "UbuntuServer",
        "sku": "18.04-LTS",
        "version": "latest"
      }
    }
  }
}
```

### GCP Instance Configuration
```json
{
  "name": "my-instance",
  "machineType": "zones/us-central1-a/machineTypes/e2-micro",
  "disks": [{
    "boot": true,
    "autoDelete": true,
    "initializeParams": {
      "sourceImage": "projects/debian-cloud/global/images/family/debian-11"
    }
  }],
  "networkInterfaces": [{
    "network": "global/networks/default",
    "accessConfigs": [{"type": "ONE_TO_ONE_NAT", "name": "External NAT"}]
  }]
}
```

## Best Practices

### Security
1. **IAM Policies**: Use least-privilege access principles
2. **Credential Rotation**: Regularly rotate access keys and secrets
3. **MFA**: Enable multi-factor authentication
4. **Encryption**: Enable encryption at rest and in transit

### Cost Optimization
1. **Right-Sizing**: Monitor and adjust resource sizes
2. **Reserved Instances**: Use reserved instances for predictable workloads
3. **Auto-Scaling**: Implement auto-scaling for variable workloads
4. **Resource Tagging**: Tag resources for cost allocation

### Monitoring
1. **CloudWatch/Monitor/Stackdriver**: Use native monitoring services
2. **Alerts**: Set up billing and resource alerts
3. **Logging**: Centralize logs across providers
4. **Performance**: Monitor application performance metrics

## Advanced Examples

### Auto-Scaling Across Providers
```rust
async fn implement_auto_scaling() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    
    // Monitor AWS instances
    let aws_instances = cloud.execute(&ToolCall::new("1", "cloud", json!({
        "provider": "aws",
        "command": "manage",
        "service": "ec2",
        "action": "list"
    }))).await?;
    
    // If AWS capacity is full, create Azure instance
    if aws_instances.content.contains("maximum capacity") {
        cloud.execute(&ToolCall::new("2", "cloud", json!({
            "provider": "azure",
            "command": "manage",
            "service": "vm",
            "action": "create",
            "resource_name": "overflow-instance"
        }))).await?;
    }
    
    Ok(())
}
```

### Cost Monitoring and Alerts
```rust
async fn monitor_costs() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    let providers = ["aws", "azure", "gcp"];
    let mut total_cost = 0.0;
    
    for provider in providers {
        let cost_info = cloud.execute(&ToolCall::new(
            &format!("cost-{}", provider),
            "cloud",
            json!({
                "provider": provider,
                "command": "cost"
            })
        )).await?;
        
        // Parse cost information and add to total
        // Implementation depends on cost API response format
        
        if total_cost > 1000.0 {
            send_cost_alert(total_cost).await?;
        }
    }
    
    Ok(())
}
```

### Resource Migration
```rust
async fn migrate_workload() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    
    // Create snapshot/backup in source provider
    // Export data from AWS
    
    // Create resources in target provider
    cloud.execute(&ToolCall::new("1", "cloud", json!({
        "provider": "gcp",
        "command": "manage",
        "service": "compute",
        "action": "create",
        "resource_name": "migrated-instance"
    }))).await?;
    
    // Import data to GCP
    // Verify migration success
    // Cleanup old resources
    
    Ok(())
}
```

## Integration

### CI/CD Pipeline
```yaml
# .github/workflows/deploy-multi-cloud.yml
name: Multi-Cloud Deployment

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        provider: [aws, azure, gcp]
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Deploy to ${{ matrix.provider }}
      run: |
        sage-cli tool cloud \
          --provider ${{ matrix.provider }} \
          --command manage \
          --service compute \
          --action create \
          --resource_name "ci-instance-${{ github.sha }}"
```

### Monitoring Dashboard
```rust
// Multi-cloud monitoring service
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cloud = CloudTool::new();
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
    
    loop {
        interval.tick().await;
        
        // Check resource health across all providers
        for provider in ["aws", "azure", "gcp"] {
            match check_provider_health(&cloud, provider).await {
                Ok(status) => println!("{} status: {}", provider, status),
                Err(e) => eprintln!("{} error: {}", provider, e),
            }
        }
    }
}

async fn check_provider_health(cloud: &CloudTool, provider: &str) -> Result<String, Box<dyn std::error::Error>> {
    let result = cloud.execute(&ToolCall::new(
        "health-check",
        "cloud",
        json!({
            "provider": provider,
            "command": "manage",
            "service": match provider {
                "aws" => "ec2",
                "azure" => "vm",
                "gcp" => "compute",
                _ => return Err("Unknown provider".into())
            },
            "action": "list"
        })
    )).await?;
    
    Ok(if result.content.contains("running") { "healthy" } else { "degraded" }.to_string())
}
```