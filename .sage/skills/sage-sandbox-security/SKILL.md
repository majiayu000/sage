---
name: sage-sandbox-security
description: Sage 沙箱安全开发指南，涵盖命令验证、路径策略、OS 级隔离、违规追踪
when_to_use: 当涉及命令安全检查、路径访问控制、沙箱配置、安全策略时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 92
---

# Sage 沙箱安全开发指南

## 模块概览

沙箱模块是 Sage 的安全核心，代码量 **5792 行**，提供多层安全防护：

```
crates/sage-core/src/sandbox/
├── mod.rs              # 公开接口 + DefaultSandbox (415行)
├── config.rs           # 沙箱配置
├── limits.rs           # 资源限制
├── executor/           # 命令执行器
│   ├── mod.rs          # 入口
│   ├── executor.rs     # SandboxExecutor
│   ├── builder.rs      # ExecutionBuilder
│   ├── limits.rs       # 资源限制执行
│   ├── types.rs        # SandboxedExecution
│   └── tests.rs        # 测试
├── policy/             # 安全策略
│   ├── mod.rs          # SandboxPolicy
│   ├── path_policy.rs  # 路径访问控制 (460行)
│   ├── command_policy.rs # 命令过滤 (137行)
│   └── network_policy.rs # 网络访问控制 (139行)
├── validation/         # 命令验证
│   ├── mod.rs          # 入口 (98行)
│   ├── heredoc_check.rs # Heredoc 注入检测 (222行)
│   ├── metacharacter_check.rs # Shell 元字符检测 (292行)
│   ├── pattern_check.rs # 危险模式检测 (219行)
│   ├── removal_check.rs # 危险删除检测 (276行)
│   ├── variable_check.rs # 变量注入检测 (173行)
│   └── types.rs        # 类型定义 (256行)
├── os_sandbox/         # OS 级沙箱
│   ├── mod.rs          # 入口 (77行)
│   ├── macos.rs        # macOS sandbox-exec (272行)
│   ├── linux.rs        # Linux seccomp (229行)
│   └── types.rs        # 配置类型 (206行)
└── violations/         # 违规追踪
    ├── mod.rs          # 入口
    ├── types.rs        # ViolationType (232行)
    ├── store.rs        # ViolationStore (328行)
    └── annotator.rs    # 违规注解 (216行)
```

---

## 一、安全架构

### 1.1 三层安全模型

```
┌─────────────────────────────────────────────────────────────┐
│                     Layer 3: OS Sandbox                      │
│          macOS sandbox-exec / Linux seccomp                  │
│                    (可选, 最强隔离)                           │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│                  Layer 2: Resource Limits                    │
│          CPU, Memory, Output, File Size (rlimit)             │
│                    (Unix 资源限制)                            │
└─────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────┐
│                   Layer 1: Policy-based                      │
│         Path/Command/Network Restrictions (默认)             │
│                    (策略检查层)                               │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Sandbox Trait

```rust
// crates/sage-core/src/sandbox/mod.rs
#[async_trait]
pub trait Sandbox: Send + Sync {
    /// 沙箱名称
    fn name(&self) -> &str;

    /// 检查路径访问
    fn check_path(&self, path: &PathBuf, write: bool) -> SandboxResult<()>;

    /// 检查命令是否允许
    fn check_command(&self, command: &str) -> SandboxResult<()>;

    /// 检查网络访问
    fn check_network(&self, host: &str, port: u16) -> SandboxResult<()>;

    /// 获取资源限制
    fn resource_limits(&self) -> &ResourceLimits;

    /// 在沙箱中执行命令
    async fn execute_command(
        &self,
        command: &str,
        args: &[String],
        working_dir: Option<&PathBuf>,
        env: Option<&HashMap<String, String>>,
    ) -> SandboxResult<SandboxedExecution>;

    /// 读取文件
    async fn read_file(&self, path: &PathBuf) -> SandboxResult<String>;

    /// 写入文件
    async fn write_file(&self, path: &PathBuf, content: &str) -> SandboxResult<()>;

    /// 沙箱是否激活
    fn is_active(&self) -> bool;

    /// 当前资源使用
    fn current_usage(&self) -> ResourceUsage;
}
```

---

## 二、命令验证系统

### 2.1 验证流程

```rust
// crates/sage-core/src/sandbox/validation/mod.rs
pub fn validate_command(command: &str, context: &ValidationContext) -> ValidationResult {
    let checks = [
        check_heredoc_safety(command),        // Heredoc 注入
        check_shell_metacharacters(command, context), // Shell 元字符
        check_dangerous_variables(command),   // 变量注入
        check_dangerous_patterns(command),    // 危险模式
        check_dangerous_removal(command),     // 危险删除
    ];

    let mut all_warnings = Vec::new();

    for result in &checks {
        if !result.allowed {
            return result.clone_with_warnings(all_warnings);
        }
        all_warnings.extend(result.warnings.clone());
    }

    ValidationResult::pass_with_warnings(CheckType::Composite, all_warnings)
}
```

### 2.2 Heredoc 注入检测

```rust
// crates/sage-core/src/sandbox/validation/heredoc_check.rs
pub fn check_heredoc_safety(command: &str) -> ValidationResult {
    // 检测 heredoc 分隔符
    let heredoc_pattern = Regex::new(r"<<\s*(\$?\w+)").unwrap();

    for cap in heredoc_pattern.captures_iter(command) {
        let delimiter = cap.get(1).unwrap().as_str();

        // 变量分隔符是高风险
        if delimiter.starts_with('$') {
            return ValidationResult::block(
                CheckType::Heredoc,
                format!("Variable heredoc delimiter: {}", delimiter),
            );
        }
    }

    ValidationResult::pass(CheckType::Heredoc)
}
```

### 2.3 危险删除检测

```rust
// crates/sage-core/src/sandbox/validation/removal_check.rs
const CRITICAL_PATHS: &[&str] = &[
    "/",
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/home",
    "/lib",
    "/lib64",
    "/opt",
    "/proc",
    "/root",
    "/sbin",
    "/sys",
    "/tmp",
    "/usr",
    "/var",
    "~",
    "$HOME",
];

pub fn check_dangerous_removal(command: &str) -> ValidationResult {
    // 检测 rm 命令
    let rm_pattern = Regex::new(r"\brm\s+(-[rf]+\s+)*(.+)").unwrap();

    if let Some(cap) = rm_pattern.captures(command) {
        let target = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        for critical in CRITICAL_PATHS {
            if target == *critical || target.starts_with(&format!("{}/", critical)) {
                return ValidationResult::block(
                    CheckType::Removal,
                    format!("Critical path removal: {}", target),
                );
            }
        }
    }

    ValidationResult::pass(CheckType::Removal)
}
```

### 2.4 Shell 元字符检测

```rust
// crates/sage-core/src/sandbox/validation/metacharacter_check.rs
const DANGEROUS_METACHARACTERS: &[(&str, &str)] = &[
    (";", "Command chaining"),
    ("&&", "Conditional execution"),
    ("||", "Conditional execution"),
    ("|", "Pipe"),
    ("`", "Command substitution"),
    ("$(", "Command substitution"),
    ("${", "Variable expansion"),
    (">(", "Process substitution"),
    ("<(", "Process substitution"),
];

pub fn check_shell_metacharacters(
    command: &str,
    context: &ValidationContext
) -> ValidationResult {
    if context.strictness == ValidationStrictness::Permissive {
        return ValidationResult::pass(CheckType::Metacharacter);
    }

    let mut warnings = Vec::new();

    for (char, desc) in DANGEROUS_METACHARACTERS {
        if command.contains(char) {
            if context.strictness == ValidationStrictness::Strict {
                return ValidationResult::block(
                    CheckType::Metacharacter,
                    format!("{}: {}", desc, char),
                );
            }
            warnings.push(ValidationWarning::new(
                WarningSeverity::Medium,
                format!("{}: {}", desc, char),
            ));
        }
    }

    ValidationResult::pass_with_warnings(CheckType::Metacharacter, warnings)
}
```

---

## 三、路径策略

### 3.1 敏感文件保护

```rust
// crates/sage-core/src/sandbox/policy/path_policy.rs

/// 敏感文件列表（学习自 Claude Code）
const SENSITIVE_FILES: &[&str] = &[
    // Git 配置
    ".gitconfig", ".git/config", ".git/hooks/",
    // Shell 配置
    ".bashrc", ".bash_profile", ".bash_history",
    ".zshrc", ".zsh_history", ".profile", ".zprofile",
    // SSH 和凭证
    ".ssh/", ".aws/", ".docker/", ".kube/", ".gnupg/",
    // 包管理器凭证
    ".npmrc", ".pypirc", ".netrc",
    ".cargo/credentials", ".cargo/credentials.toml",
    // 环境和密钥
    ".env", ".env.local", ".env.production",
    "secrets.yaml", "secrets.json", ".secrets",
    // IDE 配置（可能包含 token）
    ".vscode/settings.json", ".idea/",
];

/// 允许的 tmp 路径
const ALLOWED_TMP_PREFIXES: &[&str] = &[
    "/tmp/sage/",
    "/tmp/sage-agent/",
    "/private/tmp/sage/",      // macOS
    "/private/tmp/sage-agent/", // macOS
];
```

### 3.2 路径检查逻辑

```rust
impl PathPolicy {
    /// 检查路径访问权限
    pub fn check_path(&self, path: &Path, write: bool) -> SandboxResult<()> {
        // 1. 敏感文件检查
        if self.is_sensitive_file(path) && self.strict_sensitive_files {
            return Err(SandboxError::PathAccessDenied {
                path: path.to_string_lossy().into(),
            });
        }

        // 2. 写入检查
        if write {
            // 检查 /tmp 写入限制
            if self.is_temp_path(path) && !self.is_allowed_temp_path(path) {
                return Err(SandboxError::PathAccessDenied {
                    path: path.to_string_lossy().into(),
                });
            }

            // 检查允许写入路径
            if !self.is_write_allowed(path) {
                return Err(SandboxError::PathAccessDenied {
                    path: path.to_string_lossy().into(),
                });
            }
        } else {
            // 读取检查
            if !self.allow_all_reads && !self.is_read_allowed(path) {
                return Err(SandboxError::PathAccessDenied {
                    path: path.to_string_lossy().into(),
                });
            }
        }

        Ok(())
    }
}
```

---

## 四、OS 级沙箱

### 4.1 macOS sandbox-exec

```rust
// crates/sage-core/src/sandbox/os_sandbox/macos.rs
pub fn apply_sandbox_exec(
    cmd: &mut Command,
    config: &OsSandboxConfig
) -> Result<(), SandboxError> {
    // 生成 sandbox profile
    let profile = generate_sandbox_profile(config);

    // 保存到临时文件
    let profile_path = save_profile(&profile)?;

    // 修改命令
    let original_program = cmd.as_std().get_program().to_owned();
    let original_args: Vec<_> = cmd.as_std().get_args().map(|s| s.to_owned()).collect();

    cmd.args([
        "-f".as_ref(),
        profile_path.as_os_str(),
        original_program.as_os_str(),
    ]);
    cmd.args(&original_args);

    // 设置程序为 sandbox-exec
    *cmd = Command::new("/usr/bin/sandbox-exec");
    // ... 重新添加参数

    Ok(())
}

/// 生成 sandbox profile
fn generate_sandbox_profile(config: &OsSandboxConfig) -> String {
    let mut profile = String::new();
    profile.push_str("(version 1)\n");

    match config.mode {
        OsSandboxMode::Strict => {
            profile.push_str("(deny default)\n");
            profile.push_str("(allow process-exec)\n");
            // 添加允许的操作
        }
        OsSandboxMode::Normal => {
            profile.push_str("(deny default)\n");
            profile.push_str("(allow process*)\n");
            profile.push_str("(allow file-read*)\n");
            // ...
        }
        _ => {}
    }

    profile
}
```

### 4.2 Linux seccomp (未来)

```rust
// crates/sage-core/src/sandbox/os_sandbox/linux.rs
pub fn apply_seccomp(
    cmd: &mut Command,
    config: &OsSandboxConfig
) -> Result<(), SandboxError> {
    // TODO: 实现 seccomp 过滤
    // 使用 libseccomp-rs 或直接 syscall

    tracing::warn!("Linux seccomp sandbox not yet implemented");
    Ok(())
}
```

### 4.3 OS 沙箱可用性检查

```rust
// crates/sage-core/src/sandbox/os_sandbox/mod.rs
pub fn is_os_sandbox_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::path::Path::new("/usr/bin/sandbox-exec").exists()
    }
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/proc/sys/kernel/seccomp").exists()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

pub fn os_sandbox_name() -> &'static str {
    #[cfg(target_os = "macos")]
    { "sandbox-exec (macOS)" }
    #[cfg(target_os = "linux")]
    { "seccomp (Linux)" }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    { "none" }
}
```

---

## 五、违规追踪

### 5.1 违规类型

```rust
// crates/sage-core/src/sandbox/violations/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViolationType {
    HeredocInjection,      // Heredoc 注入
    ShellMetacharacterAbuse, // Shell 元字符滥用
    VariableInjection,     // 变量注入
    DangerousPattern,      // 危险模式
    CriticalPathRemoval,   // 关键路径删除
    SensitiveFileAccess,   // 敏感文件访问
    PathAccessDenied,      // 路径访问拒绝
    CommandBlocked,        // 命令被阻止
    DisallowedTempWrite,   // 禁止的 tmp 写入
    NetworkViolation,      // 网络违规
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationSeverity {
    Low,      // 信息性
    Medium,   // 潜在问题
    High,     // 安全问题
    Critical, // 必须阻止
}
```

### 5.2 违规存储

```rust
// crates/sage-core/src/sandbox/violations/store.rs
pub struct ViolationStore {
    violations: RwLock<Vec<Violation>>,
    max_violations: usize,
}

impl ViolationStore {
    /// 记录违规
    pub fn record(&self, violation: Violation) {
        let mut violations = self.violations.write();

        // 限制最大数量
        if violations.len() >= self.max_violations {
            violations.remove(0);
        }

        violations.push(violation);
    }

    /// 按类型统计
    pub fn count_by_type(&self, violation_type: ViolationType) -> usize {
        self.violations.read()
            .iter()
            .filter(|v| v.violation_type == violation_type)
            .count()
    }

    /// 获取高严重性违规
    pub fn get_high_severity(&self) -> Vec<Violation> {
        self.violations.read()
            .iter()
            .filter(|v| v.severity >= ViolationSeverity::High)
            .cloned()
            .collect()
    }
}
```

---

## 六、沙箱配置

### 6.1 配置结构

```rust
// crates/sage-core/src/sandbox/config.rs
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// 是否启用
    pub enabled: bool,

    /// 沙箱模式
    pub mode: SandboxMode,

    /// 工作目录
    pub working_dir: Option<PathBuf>,

    /// 允许读取的路径
    pub allowed_read_paths: Vec<PathBuf>,

    /// 允许写入的路径
    pub allowed_write_paths: Vec<PathBuf>,

    /// 允许的命令
    pub allowed_commands: Vec<String>,

    /// 阻止的命令
    pub blocked_commands: Vec<String>,

    /// 是否允许网络
    pub allow_network: bool,

    /// 超时时间
    pub timeout: Duration,

    /// 资源限制
    pub limits: ResourceLimits,
}

#[derive(Debug, Clone, Copy)]
pub enum SandboxMode {
    Permissive,  // 最少限制
    Normal,      // 默认
    Restricted,  // 受限
    Strict,      // 最严格
}
```

### 6.2 预设配置

```rust
impl SandboxConfig {
    /// 宽松模式
    pub fn permissive() -> Self {
        Self {
            enabled: true,
            mode: SandboxMode::Permissive,
            allowed_read_paths: vec![PathBuf::from("/")],
            allowed_write_paths: vec![],
            allow_network: true,
            ..Default::default()
        }
    }

    /// 严格模式
    pub fn strict(working_dir: PathBuf) -> Self {
        Self {
            enabled: true,
            mode: SandboxMode::Strict,
            working_dir: Some(working_dir.clone()),
            allowed_read_paths: vec![working_dir.clone()],
            allowed_write_paths: vec![working_dir],
            allow_network: false,
            ..Default::default()
        }
    }
}
```

### 6.3 资源限制

```rust
// crates/sage-core/src/sandbox/limits.rs
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// 最大内存 (字节)
    pub max_memory_bytes: Option<u64>,

    /// 最大 CPU 时间 (秒)
    pub max_cpu_seconds: Option<u64>,

    /// 最大输出大小 (字节)
    pub max_output_bytes: Option<u64>,

    /// 最大文件大小 (字节)
    pub max_file_size_bytes: Option<u64>,

    /// 最大进程数
    pub max_processes: Option<u32>,

    /// 最大打开文件数
    pub max_open_files: Option<u32>,
}
```

---

## 七、构建器模式

```rust
// crates/sage-core/src/sandbox/mod.rs
pub struct SandboxBuilder {
    config: SandboxConfig,
}

impl SandboxBuilder {
    pub fn new() -> Self {
        Self { config: SandboxConfig::default() }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn mode(mut self, mode: SandboxMode) -> Self {
        self.config.mode = mode;
        self
    }

    pub fn working_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.working_dir = Some(path.into());
        self
    }

    pub fn allow_read(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.allowed_read_paths.push(path.into());
        self
    }

    pub fn allow_write(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.allowed_write_paths.push(path.into());
        self
    }

    pub fn allow_command(mut self, cmd: impl Into<String>) -> Self {
        self.config.allowed_commands.push(cmd.into());
        self
    }

    pub fn block_command(mut self, cmd: impl Into<String>) -> Self {
        self.config.blocked_commands.push(cmd.into());
        self
    }

    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.config.limits.max_memory_bytes = Some(bytes);
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.config.timeout = duration;
        self
    }

    pub fn build(self) -> SandboxResult<DefaultSandbox> {
        DefaultSandbox::new(self.config)
    }
}

// 使用示例
let sandbox = SandboxBuilder::new()
    .enabled(true)
    .mode(SandboxMode::Restricted)
    .working_dir("/tmp/sage/work")
    .allow_read("/tmp")
    .allow_write("/tmp/sage")
    .allow_command("ls")
    .block_command("rm -rf")
    .timeout(Duration::from_secs(30))
    .memory_limit(100 * 1024 * 1024)
    .build()?;
```

---

## 八、错误类型

```rust
// crates/sage-core/src/sandbox/mod.rs
#[derive(Debug, Clone, thiserror::Error)]
pub enum SandboxError {
    #[error("Resource limit exceeded: {resource} ({current}/{limit})")]
    ResourceLimitExceeded { resource: String, current: u64, limit: u64 },

    #[error("Path access denied: {path}")]
    PathAccessDenied { path: String },

    #[error("Command not allowed: {command}")]
    CommandNotAllowed { command: String },

    #[error("Network access denied: {host}")]
    NetworkAccessDenied { host: String },

    #[error("Sandbox execution timeout after {0:?}")]
    Timeout(Duration),

    #[error("Sandbox initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Failed to spawn sandboxed process: {0}")]
    SpawnFailed(String),

    #[error("Invalid sandbox configuration: {0}")]
    InvalidConfig(String),

    #[error("Sandbox permission denied: {0}")]
    PermissionDenied(String),

    #[error("Sandbox internal error: {0}")]
    Internal(String),
}
```

---

## 九、开发指南

### 9.1 添加新验证检查

1. 在 `validation/` 创建新模块：
```rust
// validation/new_check.rs
pub fn check_new_pattern(command: &str) -> ValidationResult {
    // 实现检查逻辑
    ValidationResult::pass(CheckType::NewCheck)
}
```

2. 在 `validation/mod.rs` 添加：
```rust
mod new_check;
pub use new_check::check_new_pattern;

pub fn validate_command(command: &str, context: &ValidationContext) -> ValidationResult {
    let checks = [
        // ... 现有检查
        check_new_pattern(command),  // 新增
    ];
    // ...
}
```

3. 在 `CheckType` 枚举添加变体

### 9.2 添加敏感文件

在 `path_policy.rs` 的 `SENSITIVE_FILES` 添加：
```rust
const SENSITIVE_FILES: &[&str] = &[
    // ... 现有
    ".new_sensitive_file",  // 新增
];
```

### 9.3 安全最佳实践

1. **默认拒绝**: 使用白名单而非黑名单
2. **最小权限**: 只授予必要的权限
3. **深度防御**: 多层安全检查
4. **审计日志**: 记录所有违规
5. **失败安全**: 出错时选择安全选项

---

## 十、相关模块

- `sage-tool-development` - 工具开发（使用沙箱执行）
- `sage-agent-execution` - Agent 执行（沙箱集成）
- `sage-recovery-patterns` - 恢复模式（违规处理）

---

*最后更新: 2026-01-10*
