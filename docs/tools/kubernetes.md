# Kubernetes Management Tool

The Kubernetes tool provides comprehensive cluster management, deployment operations, and monitoring capabilities.

## Overview

- **Tool Name**: `kubernetes`
- **Purpose**: Kubernetes cluster management including deployments, pods, services, and monitoring
- **Location**: `crates/sage-tools/src/tools/infrastructure/kubernetes.rs`

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | Kubernetes command (deploy, get, delete, logs, scale, port-forward, apply) |
| `resource_type` | string | No | Kubernetes resource type (pod, service, deployment, configmap, secret) |
| `name` | string | No | Resource name |
| `namespace` | string | No | Kubernetes namespace (default: default) |
| `replicas` | number | No | Number of replicas for scaling |
| `image` | string | No | Container image for deployments |
| `port` | number | No | Port number for port forwarding |
| `yaml_path` | string | No | Path to YAML manifest file |

## Supported Commands

### Deploy Applications
Create new deployments:
```json
{
  "command": "deploy",
  "name": "my-app",
  "image": "nginx:latest",
  "namespace": "production",
  "replicas": 3
}
```

### Get Resources
List or describe Kubernetes resources:
```json
{
  "command": "get",
  "resource_type": "pods",
  "namespace": "default"
}
```

Get specific resource:
```json
{
  "command": "get",
  "resource_type": "deployment",
  "name": "my-app",
  "namespace": "production"
}
```

### Delete Resources
Remove Kubernetes resources:
```json
{
  "command": "delete",
  "resource_type": "deployment",
  "name": "my-app",
  "namespace": "production"
}
```

### View Logs
Access pod logs:
```json
{
  "command": "logs",
  "name": "my-app-pod",
  "namespace": "production"
}
```

### Scale Deployments
Adjust replica count:
```json
{
  "command": "scale",
  "name": "my-app",
  "replicas": 5,
  "namespace": "production"
}
```

### Port Forwarding
Forward local ports to pods:
```json
{
  "command": "port-forward",
  "name": "my-app-pod",
  "port": 8080,
  "namespace": "production"
}
```

### Apply Manifests
Deploy from YAML files:
```json
{
  "command": "apply",
  "yaml_path": "/path/to/deployment.yaml",
  "namespace": "production"
}
```

## Resource Types

### Supported Resource Types
- `pod` - Individual container instances
- `deployment` - Managed pod replicas
- `service` - Network access to pods
- `configmap` - Configuration data
- `secret` - Sensitive information
- `namespace` - Resource isolation
- `ingress` - External access rules
- `persistentvolume` - Storage resources
- `job` - Batch workloads
- `cronjob` - Scheduled tasks

## Usage Examples

### Basic Deployment Workflow
```rust
use sage_tools::KubernetesTool;

let k8s = KubernetesTool::new();

// Create deployment
let deploy_call = ToolCall::new("1", "kubernetes", json!({
    "command": "deploy",
    "name": "web-app",
    "image": "nginx:1.21",
    "replicas": 3,
    "namespace": "production"
}));
k8s.execute(&deploy_call).await?;

// Check deployment status
let status_call = ToolCall::new("2", "kubernetes", json!({
    "command": "get",
    "resource_type": "deployment",
    "name": "web-app",
    "namespace": "production"
}));
let status = k8s.execute(&status_call).await?;

// Scale if needed
let scale_call = ToolCall::new("3", "kubernetes", json!({
    "command": "scale",
    "name": "web-app",
    "replicas": 5,
    "namespace": "production"
}));
k8s.execute(&scale_call).await?;
```

### Monitoring and Debugging
```rust
// Check pod status
let pods_call = ToolCall::new("1", "kubernetes", json!({
    "command": "get",
    "resource_type": "pods",
    "namespace": "production"
}));
let pods = k8s.execute(&pods_call).await?;

// View logs for troubleshooting
let logs_call = ToolCall::new("2", "kubernetes", json!({
    "command": "logs",
    "name": "web-app-12345",
    "namespace": "production"
}));
let logs = k8s.execute(&logs_call).await?;

// Port forward for local testing
let forward_call = ToolCall::new("3", "kubernetes", json!({
    "command": "port-forward",
    "name": "web-app-12345",
    "port": 8080,
    "namespace": "production"
}));
k8s.execute(&forward_call).await?;
```

### YAML Manifest Deployment
```rust
// Apply complete application stack
let apply_call = ToolCall::new("1", "kubernetes", json!({
    "command": "apply",
    "yaml_path": "k8s/production/app.yaml",
    "namespace": "production"
}));
k8s.execute(&apply_call).await?;
```

## Configuration

### Prerequisites
- `kubectl` must be installed and configured
- Valid kubeconfig file (`~/.kube/config`)
- Appropriate cluster permissions

### Environment Setup
```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"

# Configure cluster access
kubectl config set-cluster my-cluster --server=https://k8s.example.com
kubectl config set-credentials my-user --token=<token>
kubectl config set-context my-context --cluster=my-cluster --user=my-user
kubectl config use-context my-context
```

## Best Practices

### Deployment Management
1. **Namespace Isolation**: Use namespaces to separate environments
2. **Resource Limits**: Always set CPU and memory limits
3. **Health Checks**: Configure liveness and readiness probes
4. **Rolling Updates**: Use deployment strategies for zero-downtime updates

### Security
1. **RBAC**: Implement role-based access control
2. **Secrets Management**: Use Kubernetes secrets for sensitive data
3. **Network Policies**: Restrict pod-to-pod communication
4. **Image Security**: Use trusted container registries

### Monitoring
1. **Resource Usage**: Monitor CPU and memory consumption
2. **Log Aggregation**: Centralize application logs
3. **Metrics Collection**: Use Prometheus and Grafana
4. **Alerting**: Set up alerts for critical conditions

## Advanced Examples

### Blue-Green Deployment
```rust
// Deploy new version (green)
let green_deploy = ToolCall::new("1", "kubernetes", json!({
    "command": "deploy",
    "name": "app-green",
    "image": "myapp:v2.0",
    "replicas": 3,
    "namespace": "production"
}));
k8s.execute(&green_deploy).await?;

// Test green deployment
// ... testing logic ...

// Switch traffic to green (update service selector)
let service_update = ToolCall::new("2", "kubernetes", json!({
    "command": "apply",
    "yaml_path": "k8s/service-green.yaml",
    "namespace": "production"
}));
k8s.execute(&service_update).await?;

// Remove blue deployment
let blue_delete = ToolCall::new("3", "kubernetes", json!({
    "command": "delete",
    "resource_type": "deployment",
    "name": "app-blue",
    "namespace": "production"
}));
k8s.execute(&blue_delete).await?;
```

### Canary Deployment
```rust
// Deploy canary version with 1 replica
let canary_deploy = ToolCall::new("1", "kubernetes", json!({
    "command": "deploy",
    "name": "app-canary",
    "image": "myapp:v2.0",
    "replicas": 1,
    "namespace": "production"
}));
k8s.execute(&canary_deploy).await?;

// Monitor canary metrics
// ... monitoring logic ...

// Gradually scale canary and reduce main deployment
let canary_scale = ToolCall::new("2", "kubernetes", json!({
    "command": "scale",
    "name": "app-canary",
    "replicas": 3,
    "namespace": "production"
}));
k8s.execute(&canary_scale).await?;

let main_scale = ToolCall::new("3", "kubernetes", json!({
    "command": "scale",
    "name": "app-main",
    "replicas": 2,
    "namespace": "production"
}));
k8s.execute(&main_scale).await?;
```

## Error Handling

Common error scenarios and solutions:

- **ExecutionFailed**: kubectl command failed - check cluster connectivity
- **InvalidArguments**: Missing required parameters - verify command syntax
- **ResourceNotFound**: Resource doesn't exist - check name and namespace
- **PermissionDenied**: Insufficient RBAC permissions - update cluster roles

## Integration

### CI/CD Pipeline
```yaml
# .github/workflows/deploy.yml
- name: Deploy to Kubernetes
  run: |
    sage-cli tool kubernetes \
      --command deploy \
      --name ${{ github.event.repository.name }} \
      --image ${{ env.IMAGE_TAG }} \
      --namespace production \
      --replicas 3
```

### Monitoring Integration
```rust
// Automated deployment monitoring
async fn monitor_deployment(deployment_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let k8s = KubernetesTool::new();
    
    loop {
        let status = k8s.execute(&ToolCall::new(
            "monitor",
            "kubernetes",
            json!({
                "command": "get",
                "resource_type": "deployment",
                "name": deployment_name,
                "namespace": "production"
            })
        )).await?;
        
        if status.content.contains("Ready") {
            break;
        }
        
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    
    Ok(())
}
```