# Tool Usage Policy

This document defines the correct usage boundaries for Sage Agent tools, inspired by Claude Code's design principles.

## Core Principles

1. **Tool Specialization**: Each tool has a specific purpose. Use the right tool for the job.
2. **No Overlap**: Don't use general-purpose tools (like Bash) for tasks that have dedicated tools.
3. **Fail Fast**: Validation should catch incorrect tool usage early.

## Tool Categories and Boundaries

### File Operations

**Use dedicated file tools:**
- `Read`: Reading file contents
- `Write`: Creating new files
- `Edit`: Modifying existing files
- `Glob`: Finding files by pattern
- `Grep`: Searching file contents
- `NotebookEdit`: Editing Jupyter notebooks

**Never use Bash for:**
- ❌ `cat`, `head`, `tail` → Use `Read` instead
- ❌ `echo > file` or `cat <<EOF` → Use `Write` or `Edit` instead
- ❌ `sed`, `awk` → Use `Edit` instead
- ❌ `find` → Use `Glob` instead
- ❌ `grep`, `rg` → Use `Grep` instead

**Bash is only for:**
- ✅ System commands: `git`, `npm`, `cargo`, `docker`
- ✅ Process management: `ps`, `kill`
- ✅ Environment operations: `export`, `cd`
- ✅ Package managers: `apt`, `brew`, `pip`

### Process Management

**Use dedicated process tools:**
- `Bash`: Execute system commands
- `KillShell`: Terminate background shells
- `TaskTool`: Spawn sub-agents
- `TaskOutput`: Get output from background tasks

**Bash usage rules:**
- ✅ Use for terminal operations
- ✅ Chain commands with `&&` for sequential execution
- ✅ Use `;` only when you don't care about failures
- ❌ Never use for file content manipulation
- ❌ Never use `echo` to communicate with user (output text directly)

### Task Management

**Use dedicated task tools:**
- `TodoWrite`: Create and update task lists
- `ViewTasklist`: View current tasks
- `AddTasks`: Add new tasks
- `UpdateTasks`: Update task status
- `TaskDone`: Mark tasks complete

**Task management rules:**
- ✅ Use `TodoWrite` for complex multi-step tasks
- ✅ Mark tasks as `in_progress` before starting
- ✅ Mark tasks as `completed` immediately after finishing
- ❌ Don't batch completions
- ❌ Only ONE task should be `in_progress` at a time

### Network Operations

**Use dedicated network tools:**
- `WebSearch`: Search the web
- `WebFetch`: Fetch and analyze web content
- `Browser`: Automated browser interactions

**Network rules:**
- ✅ Use `WebFetch` for reading web pages
- ✅ Use `WebSearch` for finding information
- ❌ Never use `curl` or `wget` via Bash for content fetching

### Code Exploration

**Use dedicated exploration tools:**
- `Glob`: Find files by pattern
- `Grep`: Search code content
- `Read`: Read specific files
- `TaskTool` with `Explore` agent: Complex codebase exploration

**Exploration rules:**
- ✅ Use `Glob` for pattern-based file finding
- ✅ Use `Grep` for keyword searches
- ✅ Use `TaskTool` with `Explore` agent for open-ended exploration
- ❌ Never use `find` or `grep` via Bash
- ❌ Don't use `ls` to explore directory structure

## Validation Rules

### Pre-execution Validation

Before executing any tool, validate:

1. **Tool Selection**: Is this the right tool for the task?
2. **Parameter Completeness**: Are all required parameters provided?
3. **Security**: Does the operation pose security risks?

### Bash Command Validation

Bash commands should be rejected if they contain:

- File reading: `cat`, `head`, `tail`, `less`, `more`
- File writing: `echo >`, `cat <<EOF`, `tee`
- File editing: `sed`, `awk`, `perl -i`
- File finding: `find`, `locate`
- Content search: `grep`, `rg`, `ag`, `ack`
- Communication: `echo` (for user messages)

### Exceptions

Bash commands are allowed for:

- Git operations: `git status`, `git diff`, `git commit`
- Build tools: `cargo build`, `npm install`, `make`
- Package managers: `apt install`, `brew install`
- Process inspection: `ps`, `top`, `htop`
- System info: `uname`, `df`, `du`

## Error Messages

When incorrect tool usage is detected, provide clear guidance:

```
❌ Incorrect: Using Bash to read files
   Command: cat file.txt

✅ Correct: Use the Read tool instead
   Tool: Read
   Parameters: { file_path: "file.txt" }
```

## Implementation

### Validation Layer

```rust
pub struct ToolUsageValidator {
    policies: Vec<ToolPolicy>,
}

impl ToolUsageValidator {
    pub fn validate_bash_command(&self, command: &str) -> Result<(), ValidationError> {
        // Check for file operation commands
        if command.contains("cat ") || command.contains("head ") {
            return Err(ValidationError::WrongTool {
                attempted: "Bash",
                correct: "Read",
                reason: "Use Read tool for file operations",
            });
        }

        // More validation rules...
        Ok(())
    }
}
```

### Agent Integration

The agent should:

1. Validate tool selection before execution
2. Suggest correct tool when wrong tool is used
3. Log tool usage patterns for analysis
4. Provide feedback to improve tool selection

## Benefits

1. **Consistency**: All agents use tools the same way
2. **Performance**: Specialized tools are optimized for their purpose
3. **Security**: Reduced risk of command injection
4. **Maintainability**: Clear boundaries make code easier to understand
5. **Debugging**: Easier to trace issues when tools are used correctly

## References

- Claude Code tool architecture
- Open Claude Code decompiled analysis
- Sage Agent tool implementation
