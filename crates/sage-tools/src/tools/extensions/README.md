# Extension Tools

This module provides extension tools for the Sage Agent system, enabling specialized skill execution and custom slash command support.

## Tools

### SkillTool

Execute specialized skills within conversation context. Skills provide domain-specific capabilities and expertise.

**Parameters:**
- `skill` (string, required): The name of the skill to execute

**Available Skills:**
- `artifacts-builder`: Build elaborate multi-component HTML/React artifacts
- `brainstorming`: Collaborative design refinement through Socratic dialogue
- `comprehensive-testing`: Complete testing strategy and best practices
- `elegant-architecture`: Clean architecture design with strict file limits
- `frontend-design`: Create distinctive, production-grade frontend interfaces
- `git-commit-smart`: Generate meaningful conventional commit messages
- `playwright-automation`: Browser automation and testing
- `product-manager`: Comprehensive PM framework
- `project-health-auditor`: Comprehensive codebase health analysis
- `rust-best-practices`: High-quality, idiomatic Rust code guidance
- `systematic-debugging`: Four-phase debugging framework
- `test-driven-development`: TDD discipline enforcement
- `ui-designer`: Professional UI/UX design assistant

**Example Usage:**
```rust
use sage_tools::tools::extensions::SkillTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use std::collections::HashMap;

let skill_tool = SkillTool::new();

let mut args = HashMap::new();
args.insert("skill".to_string(), serde_json::Value::String("brainstorming".to_string()));

let call = ToolCall {
    id: "call-1".to_string(),
    name: "skill".to_string(),
    arguments: args,
    call_id: None,
};

let result = skill_tool.execute(&call).await?;
println!("Result: {:?}", result);
```

### SlashCommandTool

Execute custom slash commands defined in `.claude/commands/` directory.

**Parameters:**
- `command` (string, required): The slash command with arguments (e.g., "/review-pr 123")

**Command Format:**
- Must start with `/`
- Can include arguments separated by spaces
- Arguments are passed to the command template

**Example Commands:**
- `/test` - Run test suite
- `/review-pr 123` - Review pull request #123
- `/deploy production` - Deploy to production environment
- `/help` - Show available commands

**Example Usage:**
```rust
use sage_tools::tools::extensions::SlashCommandTool;
use sage_core::tools::base::Tool;
use sage_core::tools::types::ToolCall;
use std::collections::HashMap;

let slash_tool = SlashCommandTool::new();

let mut args = HashMap::new();
args.insert("command".to_string(), serde_json::Value::String("/review-pr 123".to_string()));

let call = ToolCall {
    id: "call-1".to_string(),
    name: "SlashCommand".to_string(),
    arguments: args,
    call_id: None,
};

let result = slash_tool.execute(&call).await?;
println!("Result: {:?}", result);
```

**Custom Command Directory:**
You can specify a custom command directory when creating the tool:

```rust
use std::path::PathBuf;

let custom_dir = PathBuf::from("/path/to/commands");
let slash_tool = SlashCommandTool::with_command_dir(custom_dir);
```

## Integration

Both tools are automatically included in the default tool set when using `get_default_tools()`:

```rust
use sage_tools::tools::get_default_tools;

let tools = get_default_tools();
// Includes SkillTool and SlashCommandTool
```

Or you can get extension tools specifically:

```rust
use sage_tools::tools::get_extension_tools;

let extension_tools = get_extension_tools();
// Returns only SkillTool and SlashCommandTool
```

## Testing

Both tools include comprehensive test suites. Run tests with:

```bash
cargo test --package sage-tools extensions
```

## Architecture

Both tools follow the standard Sage tool pattern:
- Implement the `Tool` trait from `sage_core::tools::base`
- Provide schema for parameter validation
- Return structured `ToolResult` with success/error information
- Support async execution
- Include comprehensive test coverage
