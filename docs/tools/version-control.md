# Git Version Control Tool

The Git tool provides essential version control operations for managing code repositories.

## Overview

- **Tool Name**: `git`
- **Purpose**: Git version control operations including status, add, commit, push, pull, and branch management
- **Location**: `crates/sage-tools/src/tools/vcs/git_simple.rs`

## Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | Git command to execute (status, add, commit, push, pull, log, branch, etc.) |
| `path` | string | No | Working directory path |
| `message` | string | No | Commit message (required for commit command) |
| `branch` | string | No | Branch name |
| `remote` | string | No | Remote name (default: origin) |
| `files` | string | No | Files to add (space-separated, default: .) |

## Supported Commands

### Status
Check repository status and see uncommitted changes:
```json
{
  "command": "status",
  "path": "/path/to/repo"
}
```

### Add Files
Stage files for commit:
```json
{
  "command": "add",
  "files": "src/main.rs Cargo.toml",
  "path": "/path/to/repo"
}
```

### Commit Changes
Create a commit with staged changes:
```json
{
  "command": "commit",
  "message": "feat: add new feature",
  "path": "/path/to/repo"
}
```

### Push Changes
Push commits to remote repository:
```json
{
  "command": "push",
  "remote": "origin",
  "branch": "main",
  "path": "/path/to/repo"
}
```

### Pull Changes
Pull latest changes from remote:
```json
{
  "command": "pull",
  "path": "/path/to/repo"
}
```

### View Log
Display commit history:
```json
{
  "command": "log",
  "path": "/path/to/repo"
}
```

### Branch Operations
List branches or create new branch:
```json
{
  "command": "branch",
  "path": "/path/to/repo"
}
```

Create new branch:
```json
{
  "command": "branch",
  "branch": "feature/new-feature",
  "path": "/path/to/repo"
}
```

### Checkout
Switch to existing branch:
```json
{
  "command": "checkout",
  "branch": "develop",
  "path": "/path/to/repo"
}
```

### Diff
View differences:
```json
{
  "command": "diff",
  "path": "/path/to/repo"
}
```

### Remote Info
View remote repository information:
```json
{
  "command": "remote",
  "path": "/path/to/repo"
}
```

## Usage Examples

### Basic Workflow
```rust
use sage_tools::GitTool;

let git = GitTool::new();

// Check status
let status_call = ToolCall::new("1", "git", json!({
    "command": "status",
    "path": "/path/to/repo"
}));
let status = git.execute(&status_call).await?;

// Add files
let add_call = ToolCall::new("2", "git", json!({
    "command": "add",
    "files": ".",
    "path": "/path/to/repo"
}));
git.execute(&add_call).await?;

// Commit
let commit_call = ToolCall::new("3", "git", json!({
    "command": "commit",
    "message": "Initial commit",
    "path": "/path/to/repo"
}));
git.execute(&commit_call).await?;

// Push
let push_call = ToolCall::new("4", "git", json!({
    "command": "push",
    "remote": "origin",
    "branch": "main",
    "path": "/path/to/repo"
}));
git.execute(&push_call).await?;
```

## Error Handling

The tool returns `ToolError` for various failure conditions:
- **ExecutionFailed**: Git command execution failed
- **InvalidArguments**: Missing required parameters or invalid command

## Dependencies

- Git must be installed and available in PATH
- Appropriate repository permissions for operations
- Valid Git repository in the specified path

## Best Practices

1. Always check status before making commits
2. Use descriptive commit messages
3. Specify working directory for multi-repository projects
4. Handle authentication for remote operations
5. Use branch operations for feature development