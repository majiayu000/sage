//! Kubernetes Tool
//!
//! This tool provides Kubernetes cluster management including:
//! - Pod and deployment management
//! - Service and ingress configuration
//! - Resource monitoring and scaling
//! - Configuration management

use async_trait::async_trait;
use tokio::process::Command;
use tracing::{debug, info};

use sage_core::tools::base::{Tool, ToolError};
use sage_core::tools::types::{ToolCall, ToolParameter, ToolResult, ToolSchema};

/// Kubernetes management tool
#[derive(Debug, Clone)]
pub struct KubernetesTool {
    name: String,
    description: String,
}

impl KubernetesTool {
    /// Create a new Kubernetes tool
    pub fn new() -> Self {
        Self {
            name: "kubernetes".to_string(),
            description:
                "Kubernetes cluster management for deployments, pods, services, and monitoring"
                    .to_string(),
        }
    }

    /// Execute a kubectl command
    async fn execute_kubectl(
        &self,
        args: &[&str],
        namespace: Option<&str>,
    ) -> Result<String, ToolError> {
        let mut cmd = Command::new("kubectl");
        cmd.args(args);

        if let Some(ns) = namespace {
            cmd.args(&["--namespace", ns]);
        }

        debug!("Executing kubectl command: kubectl {}", args.join(" "));

        let output = cmd
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute kubectl: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!(
                "kubectl command failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }

    /// Get cluster information
    async fn get_cluster_info(&self) -> Result<String, ToolError> {
        let mut result = String::new();

        // Get cluster info
        let cluster_info = self.execute_kubectl(&["cluster-info"], None).await?;
        result.push_str("Cluster Information:\n");
        result.push_str(&cluster_info);
        result.push_str("\n");

        // Get nodes
        let nodes = self
            .execute_kubectl(&["get", "nodes", "-o", "wide"], None)
            .await?;
        result.push_str("Nodes:\n");
        result.push_str(&nodes);
        result.push_str("\n");

        // Get namespaces
        let namespaces = self.execute_kubectl(&["get", "namespaces"], None).await?;
        result.push_str("Namespaces:\n");
        result.push_str(&namespaces);

        Ok(result)
    }

    /// Manage deployments
    async fn manage_deployment(
        &self,
        action: &str,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<String, ToolError> {
        match action {
            "create" => {
                // Create a basic deployment template
                let deployment_yaml = format!(
                    r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {}
  labels:
    app: {}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: {}
  template:
    metadata:
      labels:
        app: {}
    spec:
      containers:
      - name: {}
        image: nginx:latest
        ports:
        - containerPort: 80
"#,
                    name, name, name, name, name
                );

                // Write to temporary file and apply
                let temp_file = format!("/tmp/{}-deployment.yaml", name);
                tokio::fs::write(&temp_file, deployment_yaml)
                    .await
                    .map_err(|e| {
                        ToolError::ExecutionFailed(format!(
                            "Failed to write deployment file: {}",
                            e
                        ))
                    })?;

                let result = self
                    .execute_kubectl(&["apply", "-f", &temp_file], namespace)
                    .await?;

                // Clean up temp file
                let _ = tokio::fs::remove_file(&temp_file).await;

                Ok(format!("Created deployment '{}':\n{}", name, result))
            }
            "delete" => {
                let result = self
                    .execute_kubectl(&["delete", "deployment", name], namespace)
                    .await?;
                Ok(format!("Deleted deployment '{}':\n{}", name, result))
            }
            "scale" => {
                // Default to scaling to 3 replicas
                let result = self
                    .execute_kubectl(&["scale", "deployment", name, "--replicas=3"], namespace)
                    .await?;
                Ok(format!("Scaled deployment '{}':\n{}", name, result))
            }
            "status" => {
                let result = self
                    .execute_kubectl(&["get", "deployment", name, "-o", "wide"], namespace)
                    .await?;
                Ok(format!("Deployment '{}' status:\n{}", name, result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown deployment action: {}",
                action
            ))),
        }
    }

    /// Manage pods
    async fn manage_pods(
        &self,
        action: &str,
        namespace: Option<&str>,
        pod_name: Option<&str>,
    ) -> Result<String, ToolError> {
        match action {
            "list" => {
                let result = self
                    .execute_kubectl(&["get", "pods", "-o", "wide"], namespace)
                    .await?;
                Ok(format!("Pods:\n{}", result))
            }
            "logs" => {
                let name = pod_name.ok_or_else(|| {
                    ToolError::InvalidArguments("Pod name required for logs".to_string())
                })?;
                let result = self
                    .execute_kubectl(&["logs", name, "--tail=100"], namespace)
                    .await?;
                Ok(format!("Logs for pod '{}':\n{}", name, result))
            }
            "describe" => {
                let name = pod_name.ok_or_else(|| {
                    ToolError::InvalidArguments("Pod name required for describe".to_string())
                })?;
                let result = self
                    .execute_kubectl(&["describe", "pod", name], namespace)
                    .await?;
                Ok(format!("Pod '{}' details:\n{}", name, result))
            }
            "delete" => {
                let name = pod_name.ok_or_else(|| {
                    ToolError::InvalidArguments("Pod name required for delete".to_string())
                })?;
                let result = self
                    .execute_kubectl(&["delete", "pod", name], namespace)
                    .await?;
                Ok(format!("Deleted pod '{}':\n{}", name, result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown pod action: {}",
                action
            ))),
        }
    }

    /// Manage services
    async fn manage_services(
        &self,
        action: &str,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<String, ToolError> {
        match action {
            "create" => {
                let service_yaml = format!(
                    r#"
apiVersion: v1
kind: Service
metadata:
  name: {}
spec:
  selector:
    app: {}
  ports:
    - protocol: TCP
      port: 80
      targetPort: 80
  type: ClusterIP
"#,
                    name, name
                );

                let temp_file = format!("/tmp/{}-service.yaml", name);
                tokio::fs::write(&temp_file, service_yaml)
                    .await
                    .map_err(|e| {
                        ToolError::ExecutionFailed(format!("Failed to write service file: {}", e))
                    })?;

                let result = self
                    .execute_kubectl(&["apply", "-f", &temp_file], namespace)
                    .await?;

                let _ = tokio::fs::remove_file(&temp_file).await;

                Ok(format!("Created service '{}':\n{}", name, result))
            }
            "list" => {
                let result = self
                    .execute_kubectl(&["get", "services", "-o", "wide"], namespace)
                    .await?;
                Ok(format!("Services:\n{}", result))
            }
            "delete" => {
                let result = self
                    .execute_kubectl(&["delete", "service", name], namespace)
                    .await?;
                Ok(format!("Deleted service '{}':\n{}", name, result))
            }
            _ => Err(ToolError::InvalidArguments(format!(
                "Unknown service action: {}",
                action
            ))),
        }
    }
}

impl Default for KubernetesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for KubernetesTool {
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
                ToolParameter::string(
                    "command",
                    "Kubernetes command (cluster-info, deployment, pod, service)",
                ),
                ToolParameter::optional_string(
                    "action",
                    "Action to perform (create, delete, list, scale, status, logs, describe)",
                ),
                ToolParameter::optional_string("name", "Resource name"),
                ToolParameter::optional_string("namespace", "Kubernetes namespace"),
                ToolParameter::optional_string("pod_name", "Pod name for pod-specific operations"),
            ],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let command = call.get_string("command").ok_or_else(|| {
            ToolError::InvalidArguments("Missing 'command' parameter".to_string())
        })?;

        let namespace = call.get_string("namespace");

        info!("Executing Kubernetes command: {}", command);

        let result = match command.as_str() {
            "cluster-info" => self.get_cluster_info().await?,
            "deployment" => {
                let action = call.get_string("action").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'action' parameter for deployment".to_string(),
                    )
                })?;
                let name = call.get_string("name").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'name' parameter for deployment".to_string(),
                    )
                })?;
                self.manage_deployment(&action, &name, namespace.as_deref())
                    .await?
            }
            "pod" => {
                let action = call.get_string("action").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'action' parameter for pod".to_string())
                })?;
                let pod_name = call.get_string("pod_name");
                self.manage_pods(&action, namespace.as_deref(), pod_name.as_deref())
                    .await?
            }
            "service" => {
                let action = call.get_string("action").ok_or_else(|| {
                    ToolError::InvalidArguments(
                        "Missing 'action' parameter for service".to_string(),
                    )
                })?;
                let name = call.get_string("name").ok_or_else(|| {
                    ToolError::InvalidArguments("Missing 'name' parameter for service".to_string())
                })?;
                self.manage_services(&action, &name, namespace.as_deref())
                    .await?
            }
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
    async fn test_kubernetes_tool_creation() {
        let tool = KubernetesTool::new();
        assert_eq!(tool.name(), "kubernetes");
        assert!(!tool.description().is_empty());
    }

    #[tokio::test]
    async fn test_kubernetes_tool_schema() {
        let tool = KubernetesTool::new();
        let schema = tool.schema();

        assert_eq!(schema.name, "kubernetes");
        assert!(!schema.description.is_empty());
    }
}
