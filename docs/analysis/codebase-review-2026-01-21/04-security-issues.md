# Sage Agent 安全分析报告

**分析日期**: 2026-01-21
**分析范围**: `/Users/apple/Desktop/code/AI/code-agent/sage` 代码库
**分析版本**: feat/tink-ui-refactor 分支

---

## 目录

1. [严重级别问题](#严重级别问题)
2. [高风险问题](#高风险问题)
3. [中等风险问题](#中等风险问题)
4. [低风险问题](#低风险问题)
5. [安全设计评估](#安全设计评估)
6. [修复建议总结](#修复建议总结)

---

## 严重级别问题

### 1. SQL 注入风险 - 数据库工具接受原始 SQL

**文件路径**: `/crates/sage-tools/src/tools/database/sql/types.rs`
**行号**: 30-71

**漏洞描述**:
`DatabaseOperation` 枚举允许直接传入原始 SQL 语句，没有参数化查询的强制要求。`Query` 和 `Select` 操作接受 `sql: String` 字段，可能导致 SQL 注入攻击。

```rust
pub enum DatabaseOperation {
    /// Execute a query
    Query {
        sql: String,  // 直接接受原始 SQL
        params: Option<Vec<serde_json::Value>>,
    },
    /// Execute a query and return results
    Select {
        sql: String,  // 直接接受原始 SQL
        params: Option<Vec<serde_json::Value>>,
        limit: Option<usize>,
    },
    // ...
}
```

**风险等级**: 严重

**修复建议**:
1. 强制使用参数化查询，移除 `sql` 字段中允许的原始 SQL
2. 实现 SQL 语句解析和验证
3. 添加白名单机制，仅允许特定类型的查询
4. 在执行前对 SQL 语句进行 sanitization

---

### 2. 邮件凭证明文处理

**文件路径**: `/crates/sage-tools/src/tools/data/email/types.rs`
**行号**: 39-58

**漏洞描述**:
`SmtpConfig` 和 `ImapConfig` 结构体直接存储明文密码，这些密码可能在日志、调试输出或序列化时被泄露。结构体使用 `#[derive(Debug)]`，会在调试时输出完整密码。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,  // 明文密码
    pub use_tls: bool,
    pub use_starttls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImapConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,  // 明文密码
    pub use_tls: bool,
}
```

**风险等级**: 严重

**修复建议**:
1. 使用 `secrecy` crate 包装敏感字段
2. 自定义 `Debug` 实现，屏蔽密码输出
3. 考虑使用环境变量或密钥管理服务
4. 添加 `#[serde(skip_serializing)]` 防止序列化密码

---

### 3. MongoDB 连接字符串泄露风险

**文件路径**: `/crates/sage-tools/src/tools/database/mongodb.rs`
**行号**: 87-95, 115-116

**漏洞描述**:
`MongoDbParams` 结构体包含 `connection_string`（可能包含认证凭证），并且使用 `Debug` trait 和 tracing 日志记录操作参数。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoDbParams {
    pub connection_string: String,  // 可能包含用户名/密码
    pub database: String,
    pub operation: MongoOperation,
}

// 在执行时记录日志
debug!("Executing MongoDB operation: {:?}", params.operation);
```

**风险等级**: 严重

**修复建议**:
1. 解析连接字符串，分离凭证信息
2. 自定义 `Debug` 实现，隐藏敏感信息
3. 使用专门的凭证管理而非 URL 嵌入

---

## 高风险问题

### 4. Permissive 模式允许所有命令执行

**文件路径**: `/crates/sage-core/src/sandbox/config/mod.rs`
**行号**: 132-158

**漏洞描述**:
`SandboxConfig::permissive()` 创建一个几乎没有限制的沙箱配置。`allowed_commands` 设置为空（意味着允许所有命令），`allowed_read_paths` 包含根目录 `/`。

```rust
pub fn permissive() -> Self {
    Self {
        enabled: true,
        mode: SandboxMode::Permissive,
        allowed_read_paths: vec![PathBuf::from("/")],  // 允许读取整个文件系统
        allowed_write_paths: vec![],
        allowed_commands: vec![], // Empty means all allowed
        blocked_commands: always_blocked_commands(),
        // ...
        validation_strictness: ValidationStrictness::Minimal,
        strict_sensitive_files: false,  // 禁用敏感文件保护
    }
}
```

**风险等级**: 高

**修复建议**:
1. 在 permissive 模式下也保持基本安全限制
2. 记录所有 permissive 模式下的操作
3. 默认不应使用 permissive 模式
4. 添加警告提示用户 permissive 模式的风险

---

### 5. 命令执行使用 bash -c 模式

**文件路径**: `/crates/sage-tools/src/tools/process/bash/execution.rs`
**行号**: 103-108（推断位置）

**漏洞描述**:
命令执行通过 `bash -c` 传递完整命令字符串，这种模式容易受到 shell 注入攻击。虽然有安全检查，但某些边界情况可能被绕过。

```rust
let mut cmd = Command::new("bash");
cmd.arg("-c")
    .arg(command)  // 完整命令作为单个字符串
    .current_dir(&self.working_directory)
```

**风险等级**: 高

**修复建议**:
1. 尽可能使用 `Command::new(program).args([...])` 直接执行
2. 对于必须使用 shell 的情况，强化输入验证
3. 实现命令解析器，验证命令结构
4. 考虑使用更安全的 shell 如 `nushell`

---

### 6. 空白 allowed_commands 允许所有命令

**文件路径**: `/crates/sage-core/src/sandbox/policy/command_policy.rs`
**行号**: 30, 79-83

**漏洞描述**:
当 `allowed_commands` 为空时，策略默认允许所有命令执行（仅检查 blocked 列表）。这种"默认允许"的设计不符合最小权限原则。

```rust
let allow_all = allowed_commands.is_empty();  // 空 = 全部允许

// ...

// Check allowed commands
if !self.allow_all && !self.allowed_commands.contains(&base_command) {
    return Err(SandboxError::CommandNotAllowed {
        command: command.to_string(),
    });
}
```

**风险等级**: 高

**修复建议**:
1. 改为"默认拒绝"模式
2. 要求显式配置允许的命令列表
3. 提供预定义的安全命令白名单

---

### 7. TOCTOU 竞争条件 - 路径安全检查

**文件路径**: `/crates/sage-core/src/tools/base/filesystem_tool.rs`
**行号**: 122-192（推断位置）

**漏洞描述**:
`is_safe_path()` 函数先检查路径是否存在，然后进行规范化。在检查和实际操作之间存在时间窗口，攻击者可能利用符号链接替换文件。

```rust
// 先检查存在性
if !path.exists() {
    // 对于不存在的路径...
}

// 然后规范化
let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
```

**风险等级**: 高

**修复建议**:
1. 使用原子操作或锁定机制
2. 在规范化后重新验证
3. 使用 `openat()` 系列系统调用防止 TOCTOU
4. 检查文件描述符而非路径

---

### 8. HTTP 认证信息可能被日志记录

**文件路径**: `/crates/sage-tools/src/tools/network/http_client/request.rs`
**行号**: 16-17, 99

**漏洞描述**:
HTTP 请求中的 Bearer token 和 Basic Auth 凭证在设置后，可能通过日志记录暴露。日志行 `debug!("Making HTTP request: {} {}", method, params.url)` 可能记录包含凭证的 URL。

```rust
AuthType::Bearer { token } => request.header("Authorization", format!("Bearer {}", token)),
AuthType::Basic { username, password } => request.basic_auth(username, Some(password)),

// 日志可能暴露敏感信息
debug!("Making HTTP request: {} {}", method, params.url);
```

**风险等级**: 高

**修复建议**:
1. 过滤日志中的敏感信息
2. 不在 URL 中包含认证信息
3. 使用专门的敏感数据包装类型

---

## 中等风险问题

### 9. 正则表达式绕过风险 - 命令阻止模式

**文件路径**: `/crates/sage-core/src/sandbox/policy/command_policy.rs`
**行号**: 33-47

**漏洞描述**:
使用正则表达式阻止危险命令模式，但这些模式可能被绕过。例如，`;\s*(rm|sudo|dd|mkfs)` 可以通过换行符或 tab 绕过。

```rust
let blocked_patterns = vec![
    Regex::new(r";\s*(rm|sudo|dd|mkfs)").ok(),
    Regex::new(r"\$\(").ok(),
    Regex::new(r"`").ok(),
    Regex::new(r"\|\s*(sh|bash|zsh|rm|sudo)").ok(),
    Regex::new(r">\s*/etc/").ok(),
    Regex::new(r">\s*/dev/").ok(),
]
```

**风险等级**: 中

**修复建议**:
1. 使用更严格的命令解析器而非正则
2. 添加更多绕过测试用例
3. 考虑使用命令白名单而非黑名单
4. 规范化命令字符串后再匹配

---

### 10. API 密钥自动导入可能导致意外凭证使用

**文件路径**: `/crates/sage-core/src/config/credential/providers.rs`
**行号**: 38-57

**漏洞描述**:
系统自动从其他工具（claude-code, cursor, aider）导入凭证。用户可能不知道正在使用哪个凭证，可能导致意外的 API 调用计费。

```rust
pub fn auto_import_paths() -> Vec<(String, PathBuf)> {
    let home = dirs::home_dir().unwrap_or_default();
    vec![
        ("claude-code".to_string(), home.join(".claude").join("credentials.json")),
        ("cursor".to_string(), home.join(".cursor").join("credentials.json")),
        ("aider".to_string(), home.join(".aider").join("credentials.json")),
    ]
}
```

**风险等级**: 中

**修复建议**:
1. 在使用导入的凭证前提示用户确认
2. 显示凭证来源信息
3. 提供禁用自动导入的配置选项

---

### 11. 测试代码中使用 unsafe 块操作环境变量

**文件路径**: `/crates/sage-core/src/config/credential/resolver/tests.rs`
**行号**: 18-19, 31, 94, 108
**文件路径**: `/crates/sage-core/src/config/credential/unified_loader/tests.rs`
**行号**: 9, 136, 224
**文件路径**: `/crates/sage-core/src/config/loader/tests.rs`
**行号**: 199, 222, 282, 303, 314, 327, 338, 357, 396, 517, 530, 591, 614

**漏洞描述**:
测试代码中大量使用 `unsafe` 块来操作环境变量，虽然在 Rust 2024 中这是必需的，但增加了代码复杂性和潜在的内存安全问题。

```rust
unsafe {
    std::env::set_var("ANTHROPIC_API_KEY", "test-key");
}
```

**风险等级**: 中

**修复建议**:
1. 使用专门的测试环境隔离库
2. 考虑使用依赖注入而非直接环境变量
3. 封装 unsafe 操作到专门的测试辅助函数

---

### 12. 网络验证代码重复

**文件路径**: `/crates/sage-tools/src/tools/network/http_client/validation.rs`
**文件路径**: `/crates/sage-tools/src/tools/network/validation.rs`

**漏洞描述**:
两个文件包含几乎相同的 SSRF 防护逻辑，代码重复可能导致一处修复时另一处遗漏，造成安全漏洞。

**风险等级**: 中

**修复建议**:
1. 合并两个验证模块到单一位置
2. 使用 trait 或公共函数共享验证逻辑
3. 添加集成测试确保两处行为一致

---

### 13. rm 命令通配符检测可能过于严格

**文件路径**: `/crates/sage-core/src/sandbox/validation/removal_check.rs`
**行号**: 96-104

**漏洞描述**:
当前实现阻止了 `rm -rf /tmp/*.log` 这样的合法操作，因为它被标记为"带通配符的递归删除"。这可能导致正常开发工作流被中断。

```rust
// Check for rm -rf with wildcards at root level
if RM_RF.is_match(command) && WILDCARD_PATH.is_match(target) {
    return ValidationResult::block(
        CheckType::DangerousRemoval,
        format!(
            "Recursive removal with wildcard '{}' is too dangerous",
            target
        ),
    );
}
```

**风险等级**: 中

**修复建议**:
1. 区分危险通配符（`/*`）和安全通配符（`/tmp/mydir/*.log`）
2. 允许在工作目录内使用通配符
3. 对特定模式使用确认而非阻止

---

## 低风险问题

### 14. 使用 Pid::from_raw 可能导致无效 PID

**文件路径**: `/crates/sage-tools/src/tools/process/kill_shell.rs`
**行号**: 84
**文件路径**: `/crates/sage-core/src/tools/background_task.rs`
**行号**: 251

**漏洞描述**:
使用 `Pid::from_raw()` 创建进程 ID 时没有验证 PID 的有效性，可能导致向错误的进程发送信号。

```rust
let pid_val = Pid::from_raw(pid as i32);
```

**风险等级**: 低

**修复建议**:
1. 在使用前验证 PID 是否有效
2. 检查进程是否存在
3. 验证进程所有权

---

### 15. 默认日志级别可能泄露敏感信息

**文件路径**: `/crates/sage-core/src/config/credential/resolver.rs`
**行号**: 53, 64, 73, 86, 100, 117

**漏洞描述**:
凭证解析器使用 `debug!` 和 `info!` 记录凭证来源信息。虽然不记录实际密钥值，但记录了"找到密钥"的信息，可能泄露凭证存在性。

```rust
debug!("Found {} key from CLI argument", provider);
debug!("Found {} key from environment variable {}", provider, env_var);
info!("Auto-imported {} key from {} ({})", name, source_name, path.display());
```

**风险等级**: 低

**修复建议**:
1. 将凭证相关日志降级为 trace 级别
2. 在生产环境禁用详细凭证日志
3. 对日志内容进行审计

---

### 16. 沙箱 enabled 标志可被禁用

**文件路径**: `/crates/sage-core/src/sandbox/mod.rs`
**行号**: 246-248
**文件路径**: `/crates/sage-core/src/sandbox/config/mod.rs`
**行号**: 22

**漏洞描述**:
`SandboxConfig` 的 `enabled` 字段允许完全禁用沙箱。`is_active()` 方法直接返回此配置值，不做额外检查。

```rust
fn is_active(&self) -> bool {
    self.config.enabled
}
```

**风险等级**: 低

**修复建议**:
1. 在禁用沙箱时记录警告
2. 要求特殊权限才能禁用沙箱
3. 添加环境变量覆盖以防止配置错误

---

### 17. Fork Bomb 检测可能被编码绕过

**文件路径**: `/crates/sage-tools/src/tools/process/bash/security.rs`
**行号**: 39-56

**漏洞描述**:
Fork bomb 检测使用字符串匹配，可能被 Unicode 变体、base64 编码或其他混淆技术绕过。

```rust
let fork_bombs = [
    ":(){ :|:& };:",
    ":(){:|:&};:",
];
for pattern in &fork_bombs {
    if command_lower.contains(pattern) {
        // 阻止
    }
}
```

**风险等级**: 低

**修复建议**:
1. 添加更多 fork bomb 变体检测
2. 使用行为分析而非模式匹配
3. 设置进程数限制作为后备保护

---

## 安全设计评估

### 优点

1. **多层防御**: 沙箱系统包含策略层、资源限制层和 OS 级隔离层
2. **Heredoc 注入防护**: 专门的 heredoc 安全检查模块
3. **变量注入检测**: 检测重定向和删除命令中的变量
4. **关键路径保护**: 阻止删除系统关键目录
5. **SSRF 防护**: URL 验证阻止私有 IP 和元数据端点访问
6. **用户确认机制**: 破坏性命令需要用户确认
7. **违规追踪**: 记录和分析安全违规事件

### 需要改进的方面

1. **默认安全**: 应从"默认允许"改为"默认拒绝"
2. **凭证管理**: 需要更安全的凭证存储和处理机制
3. **日志审计**: 需要确保敏感信息不被记录
4. **代码重复**: 安全相关代码不应重复
5. **输入验证**: 需要更强的数据库查询验证

---

## 修复建议总结

### 立即需要修复（严重级别）

| 问题 | 优先级 | 预估工作量 |
|------|--------|------------|
| SQL 注入风险 | P0 | 3 天 |
| 邮件凭证明文处理 | P0 | 1 天 |
| MongoDB 连接字符串泄露 | P0 | 1 天 |

### 短期修复（高风险）

| 问题 | 优先级 | 预估工作量 |
|------|--------|------------|
| Permissive 模式安全强化 | P1 | 2 天 |
| 命令执行安全增强 | P1 | 3 天 |
| TOCTOU 竞争条件 | P1 | 2 天 |
| HTTP 认证日志过滤 | P1 | 1 天 |

### 中期修复（中等风险）

| 问题 | 优先级 | 预估工作量 |
|------|--------|------------|
| 正则表达式绕过 | P2 | 2 天 |
| API 密钥导入确认 | P2 | 1 天 |
| 代码重复消除 | P2 | 1 天 |

### 长期改进（低风险 + 架构改进）

| 问题 | 优先级 | 预估工作量 |
|------|--------|------------|
| 默认安全策略重构 | P3 | 5 天 |
| 统一凭证管理 | P3 | 3 天 |
| 安全审计日志系统 | P3 | 3 天 |

---

## 附录：分析方法

本分析使用以下方法：

1. **静态代码审查**: 审查所有安全相关模块
2. **模式搜索**: 搜索已知的不安全模式（unsafe、unwrap、password、secret 等）
3. **数据流分析**: 追踪敏感数据的流向
4. **配置审查**: 检查默认配置的安全性
5. **依赖分析**: 检查外部依赖的使用方式

---

*报告生成时间: 2026-01-21*
*分析工具: Claude Code Security Analyzer*
