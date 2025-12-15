# Sandbox Execution Environment

## Overview

The sandbox module provides isolated execution environments with resource limits, path restrictions, and command filtering for secure tool execution.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Sandbox System                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌───────────────┐    ┌────────────────┐                   │
│  │ SandboxConfig │───▶│  SandboxPolicy │                   │
│  └───────────────┘    └────────┬───────┘                   │
│         │                      │                            │
│         │              ┌───────┴───────┐                   │
│         │              │               │                    │
│         ▼              ▼               ▼                    │
│  ┌─────────────┐ ┌──────────┐ ┌────────────┐              │
│  │  PathPolicy │ │ Command  │ │  Network   │              │
│  │             │ │  Policy  │ │   Policy   │              │
│  └─────────────┘ └──────────┘ └────────────┘              │
│                                                              │
│  ┌───────────────────────────────────────────────────┐     │
│  │              DefaultSandbox                        │     │
│  │  ┌─────────────┐  ┌───────────────┐              │     │
│  │  │   config    │  │    policy     │              │     │
│  │  └─────────────┘  └───────────────┘              │     │
│  │  ┌─────────────────────────────────┐             │     │
│  │  │        ResourceUsage            │             │     │
│  │  └─────────────────────────────────┘             │     │
│  └───────────────────────────────────────────────────┘     │
│                          │                                  │
│                          ▼                                  │
│  ┌───────────────────────────────────────────────────┐     │
│  │            SandboxExecutor                         │     │
│  │  - Process spawning with resource limits           │     │
│  │  - Output capture with size limits                 │     │
│  │  - Timeout enforcement                             │     │
│  └───────────────────────────────────────────────────┘     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Components

### SandboxConfig

Configuration for sandbox behavior:

```rust
pub struct SandboxConfig {
    pub enabled: bool,
    pub mode: SandboxMode,
    pub working_dir: Option<PathBuf>,
    pub allowed_read_paths: Vec<PathBuf>,
    pub allowed_write_paths: Vec<PathBuf>,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
    pub limits: ResourceLimits,
    pub timeout: Duration,
    pub allow_network: bool,
    pub allowed_hosts: Vec<String>,
    pub blocked_hosts: Vec<String>,
}
```

### SandboxMode

```rust
pub enum SandboxMode {
    Permissive,  // Minimal restrictions
    Restricted,  // Default, moderate restrictions
    Strict,      // Maximum restrictions
    Custom,      // User-defined
}
```

### ResourceLimits

```rust
pub struct ResourceLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_seconds: Option<u64>,
    pub max_output_bytes: Option<u64>,
    pub max_file_size_bytes: Option<u64>,
    pub max_processes: Option<u32>,
    pub max_open_files: Option<u32>,
    pub max_stack_bytes: Option<u64>,
    pub max_file_writes: Option<u32>,
    pub max_total_write_bytes: Option<u64>,
}
```

## Policies

### PathPolicy

Controls file system access:

```rust
// Read access check
policy.check_path(&PathBuf::from("/tmp/test.txt"), false)?;

// Write access check
policy.check_path(&PathBuf::from("/tmp/output.txt"), true)?;
```

**Default Denied Paths:**
- `/etc/passwd`, `/etc/shadow`, `/etc/sudoers`
- `/root`
- `/var/log`
- `/proc`, `/sys`, `/dev`

### CommandPolicy

Controls command execution:

```rust
// Command check
policy.check_command("ls -la")?;

// Blocked patterns
// - Shell metacharacter abuse: `; rm`, `$(...)`, backticks
// - Pipes to dangerous commands: `| sh`, `| bash`
// - Redirects to sensitive paths: `> /etc/`, `> /dev/`
```

**Default Blocked Commands:**
- `rm`, `rmdir`, `mv`, `cp`
- `chmod`, `chown`, `chgrp`
- `kill`, `killall`, `pkill`
- `sudo`, `su`, `doas`
- `sh`, `bash`, `zsh` (shell escape prevention)

### NetworkPolicy

Controls network access:

```rust
// Network check
policy.check_access("api.example.com", 443)?;
```

**Default Blocked Ports:**
- 22 (SSH), 23 (Telnet)
- 25 (SMTP), 110 (POP3), 143 (IMAP)
- 445 (SMB)
- 3306 (MySQL), 5432 (PostgreSQL)
- 6379 (Redis), 27017 (MongoDB)

## Sandbox Trait

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    fn name(&self) -> &str;
    fn check_path(&self, path: &PathBuf, write: bool) -> SandboxResult<()>;
    fn check_command(&self, command: &str) -> SandboxResult<()>;
    fn check_network(&self, host: &str, port: u16) -> SandboxResult<()>;
    fn resource_limits(&self) -> &ResourceLimits;

    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&PathBuf>,
        env: Option<&HashMap<String, String>>,
    ) -> SandboxResult<SandboxedExecution>;

    async fn read_file(&self, path: &PathBuf) -> SandboxResult<String>;
    async fn write_file(&self, path: &PathBuf, content: &str) -> SandboxResult<()>;

    fn is_active(&self) -> bool;
    fn current_usage(&self) -> ResourceUsage;
}
```

## SandboxExecutor

Process execution with resource limits:

```rust
let result = SandboxExecutor::execute(
    "ls",
    &["-la".to_string()],
    Some(&PathBuf::from("/tmp")),
    None,
    &ResourceLimits::default(),
    Duration::from_secs(30),
).await?;

println!("Exit code: {:?}", result.exit_code);
println!("Output: {}", result.stdout);
println!("Duration: {:?}", result.duration);
```

### Unix Resource Limits

On Unix systems, the executor applies:
- `RLIMIT_AS` - Address space limit (memory)
- `RLIMIT_CPU` - CPU time limit
- `RLIMIT_NOFILE` - Open file descriptor limit
- `RLIMIT_STACK` - Stack size limit

## SandboxBuilder

Fluent API for sandbox construction:

```rust
let sandbox = SandboxBuilder::new()
    .enabled(true)
    .mode(SandboxMode::Restricted)
    .working_dir("/project")
    .allow_read("/project")
    .allow_read("/tmp")
    .allow_write("/project/output")
    .allow_command("cargo")
    .allow_command("rustc")
    .block_command("rm")
    .timeout(Duration::from_secs(60))
    .memory_limit(512 * 1024 * 1024)  // 512 MB
    .cpu_limit(30)                      // 30 seconds
    .output_limit(10 * 1024 * 1024)     // 10 MB
    .allow_network(true)
    .build()?;
```

## Usage Examples

### Basic Usage

```rust
use sage_core::{DefaultSandbox, SandboxConfig, Sandbox};

// Create default sandbox
let sandbox = DefaultSandbox::default_sandbox()?;

// Check if command is allowed
sandbox.check_command("ls -la")?;

// Execute command
let result = sandbox.execute_command(
    "echo",
    &["Hello".to_string()],
    None,
    None,
).await?;
```

### Strict Mode

```rust
use sage_core::{DefaultSandbox, SandboxConfig};

// Create strict sandbox
let sandbox = DefaultSandbox::strict(PathBuf::from("/project"))?;

// Only basic commands allowed
// No network access
// Short timeouts
```

### Custom Configuration

```rust
let config = SandboxConfig {
    enabled: true,
    mode: SandboxMode::Custom,
    working_dir: Some(PathBuf::from("/workspace")),
    allowed_read_paths: vec![
        PathBuf::from("/workspace"),
        PathBuf::from("/usr/share"),
    ],
    allowed_write_paths: vec![
        PathBuf::from("/workspace/output"),
    ],
    allowed_commands: vec!["cargo".into(), "git".into()],
    blocked_commands: vec!["rm".into()],
    limits: ResourceLimits {
        max_memory_bytes: Some(1024 * 1024 * 1024), // 1 GB
        max_cpu_seconds: Some(120),
        ..Default::default()
    },
    timeout: Duration::from_secs(300),
    allow_network: true,
    allowed_hosts: vec!["crates.io".into(), "github.com".into()],
    blocked_hosts: vec![],
    env_passthrough: vec!["PATH".into(), "HOME".into()],
    env_override: vec![],
};

let sandbox = DefaultSandbox::new(config)?;
```

## Error Handling

```rust
pub enum SandboxError {
    ResourceLimitExceeded { resource: String, current: u64, limit: u64 },
    PathAccessDenied { path: String },
    CommandNotAllowed { command: String },
    NetworkAccessDenied { host: String },
    Timeout(Duration),
    InitializationFailed(String),
    SpawnFailed(String),
    InvalidConfig(String),
    PermissionDenied(String),
    Internal(String),
}
```

## Integration with Tools

The sandbox integrates with the tool system:

```rust
impl From<SandboxError> for ToolError {
    fn from(err: SandboxError) -> Self {
        match err {
            SandboxError::Timeout(_) => ToolError::Timeout,
            SandboxError::PathAccessDenied { path } =>
                ToolError::PermissionDenied(format!("Path access denied: {}", path)),
            SandboxError::CommandNotAllowed { command } =>
                ToolError::PermissionDenied(format!("Command not allowed: {}", command)),
            // ...
        }
    }
}
```

## Security Considerations

1. **Defense in Depth**: Multiple layers of checks (path, command, network)
2. **Least Privilege**: Default to restrictive settings
3. **Resource Limits**: Prevent resource exhaustion attacks
4. **Input Validation**: Validate commands before execution
5. **Output Limits**: Prevent memory exhaustion from large outputs
6. **Timeout Enforcement**: Kill long-running processes
7. **Path Normalization**: Resolve symlinks to prevent bypasses

## Test Coverage

- 34 unit tests covering:
  - Configuration modes (permissive, restricted, strict)
  - Command filtering and pattern blocking
  - Path access control
  - Network policy enforcement
  - Process execution and timeouts
  - Resource limit checking
