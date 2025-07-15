# API Reference Documentation

This section contains detailed API documentation for all Sage Agent crates and modules.

## 📚 Contents

### Core API (`sage-core`)
- **[Agent API](core-api.md#agent-api)** - Agent execution and management
- **[LLM Client API](core-api.md#llm-client-api)** - Language model integration
- **[Tool System API](core-api.md#tool-system-api)** - Tool execution and management
- **[Configuration API](core-api.md#configuration-api)** - Configuration management
- **[UI Components API](core-api.md#ui-components-api)** - Terminal UI components
- **[Trajectory API](core-api.md#trajectory-api)** - Execution recording and replay

### SDK API (`sage-sdk`)
- **[SDK Client](sdk-api.md#sdk-client)** - High-level SDK interface
- **[Execution Results](sdk-api.md#execution-results)** - Result handling and analysis
- **[Configuration Builder](sdk-api.md#configuration-builder)** - SDK configuration
- **[Error Handling](sdk-api.md#error-handling)** - SDK error management

### Tools API (`sage-tools`)
- **[Built-in Tools](tools-api.md#built-in-tools)** - Available built-in tools
- **[Tool Interface](tools-api.md#tool-interface)** - Tool development interface
- **[Tool Registry](tools-api.md#tool-registry)** - Tool registration and discovery
- **[Custom Tools](tools-api.md#custom-tools)** - Creating custom tools

### CLI API (`sage-cli`)
- **[Command Interface](cli-api.md#command-interface)** - CLI command structure
- **[Interactive Mode](cli-api.md#interactive-mode)** - Interactive mode API
- **[Configuration Commands](cli-api.md#configuration-commands)** - Configuration management
- **[Output Formatting](cli-api.md#output-formatting)** - Output formatting options

## 🔧 API Design Principles

### Consistency
- Consistent naming conventions across all APIs
- Uniform error handling patterns
- Standardized parameter and return types
- Common async/await patterns

### Usability
- Clear and intuitive method names
- Comprehensive documentation with examples
- Sensible default values
- Builder patterns for complex configuration

### Extensibility
- Trait-based design for extensibility
- Plugin-friendly architecture
- Backward compatibility guarantees
- Versioned API contracts

### Performance
- Zero-cost abstractions where possible
- Efficient memory usage patterns
- Async-first design
- Minimal allocations in hot paths

## 📖 Usage Examples

### Basic Agent Execution
```rust
use sage_sdk::SageAgentSDK;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = SageAgentSDK::new()?
        .with_provider_and_model("openai", "gpt-4", None)?
        .with_max_steps(10);
    
    let result = sdk.run("Create a hello world script").await?;
    println!("Result: {}", result.final_result().unwrap_or_default());
    
    Ok(())
}
```

### Custom Tool Development
```rust
use sage_core::tools::{Tool, ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;

pub struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> &str {
        "my_custom_tool"
    }
    
    fn description(&self) -> &str {
        "A custom tool example"
    }
    
    async fn execute(&self, call: &ToolCall) -> ToolResult {
        // Tool implementation
        ToolResult::success(&call.id, &call.name, "Tool executed successfully")
    }
    
    fn schema(&self) -> ToolSchema {
        // Tool schema definition
        ToolSchema::new(self.name(), self.description())
    }
}
```

### Configuration Management
```rust
use sage_core::config::{Config, ModelParameters};
use std::collections::HashMap;

let mut model_params = HashMap::new();
model_params.insert("openai".to_string(), ModelParameters {
    model: "gpt-4".to_string(),
    temperature: Some(0.7),
    max_tokens: Some(4000),
    ..Default::default()
});

let config = Config {
    default_provider: "openai".to_string(),
    max_steps: 20,
    model_providers: model_params,
    ..Default::default()
};
```

## 🔍 API Reference Format

### Method Documentation
Each API method includes:
- **Purpose** - What the method does
- **Parameters** - Input parameters with types and descriptions
- **Returns** - Return type and description
- **Errors** - Possible error conditions
- **Examples** - Usage examples
- **Since** - Version when method was added

### Type Documentation
Each type includes:
- **Description** - Purpose and usage
- **Fields** - Field descriptions for structs
- **Variants** - Variant descriptions for enums
- **Implementations** - Available trait implementations
- **Examples** - Usage examples

### Trait Documentation
Each trait includes:
- **Purpose** - What the trait represents
- **Required Methods** - Methods that must be implemented
- **Provided Methods** - Default implementations
- **Implementors** - Types that implement the trait
- **Examples** - Implementation examples

## 🚀 API Stability

### Stability Guarantees
- **Public APIs** - Backward compatibility within major versions
- **Experimental APIs** - May change without notice (marked as such)
- **Internal APIs** - No stability guarantees
- **Deprecated APIs** - Marked for removal in future versions

### Versioning
- **Semantic Versioning** - Major.Minor.Patch format
- **Breaking Changes** - Only in major version updates
- **New Features** - Added in minor version updates
- **Bug Fixes** - Included in patch version updates

### Migration Guides
- Migration guides for major version updates
- Deprecation notices with migration paths
- Compatibility layers when possible
- Clear upgrade instructions

## 📝 Contributing to API Documentation

### Documentation Standards
- Use rustdoc format for all public APIs
- Include comprehensive examples
- Document error conditions
- Provide usage guidelines

### Review Process
- API changes require documentation updates
- Documentation reviewed with code changes
- Examples tested for correctness
- Consistency checked across modules

---

For implementation guides, see the [Development Documentation](../development/).
For usage examples, see the [User Guide](../user-guide/).
