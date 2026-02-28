# Tool Usage Examples

This directory contains practical examples of using Sage Agent tools in real-world scenarios.

## Example Categories

### Basic Tool Usage
- [Simple Git Operations](git-basics.md)
- [Log Analysis](log-analysis.md)
- [Test Generation](test-generation.md)

### Infrastructure Management
- [AWS Deployment](aws-deployment.md)
- [Kubernetes Application](k8s-application.md)
- [Terraform Infrastructure](terraform-infrastructure.md)

### Development Workflows
- [CI/CD Pipeline](cicd-pipeline.md)
- [Multi-Cloud Deployment](multi-cloud.md)
- [Monitoring Setup](monitoring-setup.md)

### Advanced Integration
- [Microservices Architecture](microservices.md)
- [Disaster Recovery](disaster-recovery.md)
- [Security Scanning](security-scanning.md)

## Quick Start Examples

### Basic Tool Chain
```rust
use sage_tools::*;

async fn basic_development_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Check repository status
    let git = GitTool::new();
    let status = git.execute(&ToolCall::new("1", "git", json!({
        "command": "status",
        "path": "/project"
    }))).await?;
    
    // 2. Analyze logs for errors
    let analyzer = LogAnalyzerTool::new();
    let analysis = analyzer.execute(&ToolCall::new("2", "log_analyzer", json!({
        "command": "errors",
        "file_path": "/var/log/app.log"
    }))).await?;
    
    // 3. Generate tests for new code
    let generator = TestGeneratorTool::new();
    let tests = generator.execute(&ToolCall::new("3", "test_generator", json!({
        "command": "unit",
        "language": "rust",
        "function_name": "process_data"
    }))).await?;
    
    Ok(())
}
```

### Infrastructure Deployment
```rust
async fn deploy_infrastructure() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create Terraform configuration
    let terraform = TerraformTool::new();
    terraform.execute(&ToolCall::new("1", "terraform", json!({
        "command": "generate",
        "working_dir": "/infrastructure",
        "resource_type": "aws_ec2"
    }))).await?;
    
    // 2. Deploy infrastructure
    terraform.execute(&ToolCall::new("2", "terraform", json!({
        "command": "apply",
        "working_dir": "/infrastructure",
        "auto_approve": true
    }))).await?;
    
    // 3. Deploy application to Kubernetes
    let k8s = KubernetesTool::new();
    k8s.execute(&ToolCall::new("3", "kubernetes", json!({
        "command": "deploy",
        "name": "my-app",
        "image": "nginx:latest",
        "replicas": 3
    }))).await?;
    
    Ok(())
}
```

## Integration Patterns

### Event-Driven Workflow
```rust
use tokio::sync::mpsc;

async fn event_driven_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(100);
    
    // Log monitoring task
    tokio::spawn(async move {
        let analyzer = LogAnalyzerTool::new();
        loop {
            // Monitor logs every minute
            tokio::time::sleep(Duration::from_secs(60)).await;
            
            let result = analyzer.execute(&ToolCall::new("monitor", "log_analyzer", json!({
                "command": "errors",
                "file_path": "/var/log/app.log",
                "lines": 100
            }))).await;
            
            if let Ok(analysis) = result {
                if analysis.content.contains("CRITICAL") {
                    tx.send("critical_error").await.unwrap();
                }
            }
        }
    });
    
    // Event handler
    while let Some(event) = rx.recv().await {
        match event {
            "critical_error" => {
                // Scale up infrastructure
                let k8s = KubernetesTool::new();
                k8s.execute(&ToolCall::new("scale", "kubernetes", json!({
                    "command": "scale",
                    "name": "my-app",
                    "replicas": 10
                }))).await?;
            },
            _ => {}
        }
    }
    
    Ok(())
}
```

### Parallel Tool Execution
```rust
use futures::future::join_all;

async fn parallel_operations() -> Result<(), Box<dyn std::error::Error>> {
    let operations = vec![
        // Git operations
        async {
            let git = GitTool::new();
            git.execute(&ToolCall::new("1", "git", json!({
                "command": "status",
                "path": "/project"
            }))).await
        },
        
        // Log analysis
        async {
            let analyzer = LogAnalyzerTool::new();
            analyzer.execute(&ToolCall::new("2", "log_analyzer", json!({
                "command": "analyze",
                "file_path": "/var/log/app.log"
            }))).await
        },
        
        // Infrastructure check
        async {
            let cloud = CloudTool::new();
            cloud.execute(&ToolCall::new("3", "cloud", json!({
                "provider": "aws",
                "command": "manage",
                "service": "ec2",
                "action": "list"
            }))).await
        },
    ];
    
    let results = join_all(operations).await;
    
    for result in results {
        match result {
            Ok(output) => println!("Success: {}", output.content),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

## Tool Composition

### High-Level Abstractions
```rust
// Custom deployment orchestrator using multiple tools
pub struct DeploymentOrchestrator {
    git: GitTool,
    terraform: TerraformTool,
    kubernetes: KubernetesTool,
    cloud: CloudTool,
}

impl DeploymentOrchestrator {
    pub fn new() -> Self {
        Self {
            git: GitTool::new(),
            terraform: TerraformTool::new(),
            kubernetes: KubernetesTool::new(),
            cloud: CloudTool::new(),
        }
    }
    
    pub async fn full_deployment(&self, config: &DeploymentConfig) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Validate repository
        self.git.execute(&ToolCall::new("1", "git", json!({
            "command": "status",
            "path": &config.repo_path
        }))).await?;
        
        // 2. Deploy infrastructure
        self.terraform.execute(&ToolCall::new("2", "terraform", json!({
            "command": "apply",
            "working_dir": &config.infrastructure_path,
            "auto_approve": true
        }))).await?;
        
        // 3. Deploy application
        self.kubernetes.execute(&ToolCall::new("3", "kubernetes", json!({
            "command": "deploy",
            "name": &config.app_name,
            "image": &config.image,
            "replicas": config.replicas
        }))).await?;
        
        // 4. Verify deployment
        self.cloud.execute(&ToolCall::new("4", "cloud", json!({
            "provider": &config.provider,
            "command": "manage",
            "service": "ec2",
            "action": "list"
        }))).await?;
        
        Ok(())
    }
}
```

## Testing Examples

### Unit Testing Tools
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_git_tool() {
        let git = GitTool::new();
        let schema = git.schema();
        
        assert_eq!(schema.name, "git");
        assert!(!schema.parameters.is_empty());
    }
    
    #[tokio::test]
    async fn test_log_analyzer() {
        let analyzer = LogAnalyzerTool::new();
        
        // Create test log file
        let test_log = "/tmp/test.log";
        std::fs::write(test_log, "ERROR: Test error\nINFO: Test info").unwrap();
        
        let result = analyzer.execute(&ToolCall::new("test", "log_analyzer", json!({
            "command": "errors",
            "file_path": test_log
        }))).await;
        
        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.content.contains("ERROR"));
        
        // Cleanup
        std::fs::remove_file(test_log).unwrap();
    }
}
```

### Integration Testing
```rust
#[tokio::test]
async fn test_deployment_workflow() {
    let orchestrator = DeploymentOrchestrator::new();
    let config = DeploymentConfig {
        repo_path: "/tmp/test-repo".to_string(),
        infrastructure_path: "/tmp/terraform".to_string(),
        app_name: "test-app".to_string(),
        image: "nginx:latest".to_string(),
        replicas: 1,
        provider: "aws".to_string(),
    };
    
    // Setup test environment
    setup_test_repo(&config.repo_path).await;
    setup_test_infrastructure(&config.infrastructure_path).await;
    
    // Run deployment
    let result = orchestrator.full_deployment(&config).await;
    assert!(result.is_ok());
    
    // Cleanup
    cleanup_test_environment(&config).await;
}
```

## Best Practices

1. **Error Handling**: Always handle tool errors gracefully
2. **Resource Cleanup**: Clean up resources after operations
3. **Logging**: Log tool operations for debugging
4. **Testing**: Test tool integrations thoroughly
5. **Documentation**: Document complex tool workflows
6. **Security**: Validate inputs and handle credentials securely

## Contributing Examples

To add new examples:

1. Create a new markdown file in the appropriate category
2. Include practical, runnable code examples
3. Explain the use case and expected outcomes
4. Add error handling and best practices
5. Update this README with links to your example

See [Contributing Guide](../../development/contributing.md) for more details.