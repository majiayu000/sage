# Sage Agent Tools Documentation

This directory contains comprehensive documentation for all tools available in the Sage Agent framework.

## Tool Categories

### Core Tools
- [File Operations](file-operations.md) - File editing, JSON manipulation, codebase retrieval
- [Process Management](process-management.md) - Bash execution and terminal operations
- [Task Management](task-management.md) - Task tracking and workflow management

### Development Tools
- [Version Control](version-control.md) - Git operations and repository management
- [Testing](testing.md) - Test generation and execution
- [Monitoring](monitoring.md) - Log analysis and system monitoring

### Infrastructure & Cloud
- [Container Management](container-management.md) - Docker operations
- [Kubernetes](kubernetes.md) - Cluster management and deployments
- [Terraform](terraform.md) - Infrastructure as Code management
- [Cloud Providers](cloud-providers.md) - AWS, Azure, GCP resource management

### Data & Security
- [Database Operations](database-operations.md) - SQL and NoSQL database management
- [Data Processing](data-processing.md) - CSV, Excel, and data transformation
- [Security Tools](security-tools.md) - Vulnerability scanning and security analysis
- [Network Tools](network-tools.md) - HTTP clients and web operations

### Communication
- [Email Tools](email-tools.md) - SMTP and IMAP operations

## Quick Start

All tools follow the same interface pattern:

```rust
use sage_core::tools::{Tool, ToolCall, ToolResult};

// Create tool instance
let tool = SomeTool::new();

// Get tool schema
let schema = tool.schema();

// Execute tool with parameters
let call = ToolCall::new("tool_id", tool.name(), parameters);
let result = tool.execute(&call).await?;
```

## Tool Development Guidelines

1. **Interface Compliance**: All tools must implement the `Tool` trait with `schema()` and `execute()` methods
2. **Error Handling**: Use `ToolError` types for consistent error reporting
3. **Parameter Validation**: Validate all required parameters in the `execute()` method
4. **Async Operations**: All tools support async execution using tokio
5. **Testing**: Include unit tests for all tool functionality
6. **Documentation**: Provide clear parameter descriptions and usage examples

## Configuration

Tools can be configured through:
- Environment variables
- Configuration files (`sage_config.json`)
- Runtime parameters

See [Configuration Guide](../user-guide/configuration.md) for details.