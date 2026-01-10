---
name: sage-config-system
description: Sage 配置系统开发指南，涵盖多源加载、凭证管理、验证、持久化
when_to_use: 当涉及配置加载、凭证管理、配置验证、运行时配置修改时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 93
---

# Sage 配置系统开发指南

## 模块概览

配置模块是 Sage 的核心基础设施，代码量 **11083 行**，包含：

```
crates/sage-core/src/config/
├── mod.rs              # 公开接口
├── config.rs           # 主配置结构 (393行)
├── defaults.rs         # 默认配置加载
├── model_params.rs     # 模型参数 (658行)
├── provider.rs         # Provider 配置 (882行)
├── provider_defaults.rs # Provider 默认值 (173行)
├── provider_registry.rs # Provider 注册表 (501行)
├── persistence.rs      # 运行时持久化 (437行)
├── timeouts.rs         # 超时配置 (157行)
├── tool_config.rs      # 工具配置 (100行)
├── mcp_config.rs       # MCP 配置 (274行)
├── logging_config.rs   # 日志配置 (105行)
├── trajectory_config.rs # 轨迹配置 (48行)
├── lakeview_config.rs  # Lakeview 配置
├── loader/             # 配置加载器
│   ├── mod.rs          # 入口
│   ├── builder.rs      # 构建器模式 (74行)
│   ├── loading.rs      # 加载逻辑
│   ├── types.rs        # ConfigSource 类型
│   └── tests.rs        # 测试
├── credential/         # 凭证管理系统
│   ├── mod.rs          # 入口
│   ├── resolver.rs     # 凭证解析器
│   ├── resolved.rs     # 解析结果
│   ├── source.rs       # 凭证来源
│   ├── status.rs       # 配置状态
│   ├── hint.rs         # 状态提示
│   └── unified_loader.rs # 统一加载器
├── validation/         # 配置验证
│   ├── mod.rs          # ConfigValidator (156行)
│   ├── provider.rs     # Provider 验证
│   ├── model.rs        # 模型验证
│   ├── limits.rs       # 限制验证
│   ├── paths.rs        # 路径验证
│   ├── tools.rs        # 工具验证
│   ├── logging.rs      # 日志验证
│   └── lakeview.rs     # Lakeview 验证
├── onboarding/         # 引导流程
│   ├── mod.rs          # 入口
│   ├── manager.rs      # 引导管理器 (702行)
│   └── state.rs        # 引导状态 (511行)
├── args_loader.rs      # 命令行参数加载
├── env_loader.rs       # 环境变量加载
└── file_loader.rs      # 文件加载
```

---

## 一、核心架构：Config

### 1.1 主配置结构

```rust
// crates/sage-core/src/config/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 默认 LLM Provider
    pub default_provider: String,

    /// 最大执行步数 (None = 无限)
    pub max_steps: Option<u32>,

    /// 总 Token 预算
    pub total_token_budget: Option<u64>,

    /// 每个 Provider 的模型参数
    pub model_providers: HashMap<String, ModelParameters>,

    /// Lakeview 配置
    pub lakeview_config: Option<LakeviewConfig>,
    pub enable_lakeview: bool,

    /// 工作目录
    pub working_directory: Option<PathBuf>,

    /// 子配置
    pub tools: ToolConfig,
    pub logging: LoggingConfig,
    pub trajectory: TrajectoryConfig,
    pub mcp: McpConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "anthropic".to_string(),
            max_steps: None,  // 无限制
            total_token_budget: None,
            model_providers: create_default_providers(),
            // ...
        }
    }
}
```

### 1.2 深度合并机制

**关键设计：字段级覆盖而非全量替换**

```rust
impl Config {
    /// 合并配置（other 优先）
    ///
    /// 使用深度合并 - 对 model_providers 进行字段级覆盖，
    /// 而不是替换整个 provider 配置。
    pub fn merge(&mut self, other: Config) {
        // 非空才覆盖
        if !other.default_provider.is_empty() {
            self.default_provider = other.default_provider;
        }

        // Option 类型：有值才覆盖
        if other.max_steps.is_some() {
            self.max_steps = other.max_steps;
        }

        // 深度合并 model_providers
        for (provider, other_params) in other.model_providers {
            if let Some(existing) = self.model_providers.get_mut(&provider) {
                existing.merge(other_params);  // 字段级合并
            } else {
                self.model_providers.insert(provider, other_params);
            }
        }

        // 子配置也使用 merge
        self.tools.merge(other.tools);
        self.logging.merge(other.logging);
        self.mcp.merge(other.mcp);
    }
}
```

---

## 二、配置加载系统

### 2.1 多源加载器

```rust
// crates/sage-core/src/config/loader/builder.rs
pub struct ConfigLoader {
    sources: Vec<ConfigSource>,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self { sources: Vec::new() }
    }

    /// 链式添加配置源
    pub fn with_file<P: AsRef<Path>>(self, path: P) -> Self {
        self.add_source(ConfigSource::File(path.as_ref().to_path_buf()))
    }

    pub fn with_env(self) -> Self {
        self.add_source(ConfigSource::Environment)
    }

    pub fn with_args(self, args: HashMap<String, String>) -> Self {
        self.add_source(ConfigSource::CommandLine(args))
    }

    pub fn with_defaults(self) -> Self {
        self.add_source(ConfigSource::Default)
    }

    /// 加载并合并所有配置源
    pub fn load(self) -> SageResult<Config> {
        let mut config = Config::default();

        for source in &self.sources {
            let source_config = load_from_source(source)?;
            config.merge(source_config);
        }

        config.validate()?;
        Ok(config)
    }
}
```

### 2.2 配置源类型

```rust
// crates/sage-core/src/config/loader/types.rs
pub enum ConfigSource {
    /// 文件源（JSON/TOML/YAML）
    File(PathBuf),
    /// 环境变量
    Environment,
    /// 命令行参数
    CommandLine(HashMap<String, String>),
    /// 默认配置
    Default,
}
```

### 2.3 典型加载流程

```rust
// 使用示例
let config = ConfigLoader::new()
    .with_defaults()                    // 1. 默认值
    .with_file("~/.sage/config.json")   // 2. 全局配置
    .with_file("./sage_config.json")    // 3. 项目配置
    .with_env()                         // 4. 环境变量
    .with_args(cli_args)                // 5. CLI 参数
    .load()?;
```

**加载优先级（后者覆盖前者）：**
```
Default → Global File → Project File → Env → CLI Args
```

---

## 三、凭证管理系统

### 3.1 多源凭证解析

```rust
// crates/sage-core/src/config/credential/resolver.rs
pub struct CredentialResolver {
    config: ResolverConfig,
}

impl CredentialResolver {
    /// 解析所有凭证
    pub fn resolve_all(&self) -> ResolvedCredentials {
        let mut credentials = ResolvedCredentials::new();

        // 按优先级顺序解析
        for source in self.config.sources_in_order() {
            let found = self.resolve_from_source(&source);
            credentials.merge(found);
        }

        credentials
    }
}
```

### 3.2 凭证来源与优先级

```rust
// crates/sage-core/src/config/credential/source.rs
pub enum CredentialSource {
    /// CLI 参数 --api-key
    CliArg,
    /// 环境变量 ANTHROPIC_API_KEY
    Environment,
    /// 项目配置 ./sage_config.json
    ProjectConfig,
    /// 全局配置 ~/.sage/credentials.json
    GlobalConfig,
    /// 自动导入（如 ~/.anthropic/credentials）
    AutoImport,
}

pub enum CredentialPriority {
    Highest = 0,  // CLI
    High = 1,     // Env
    Medium = 2,   // Project
    Low = 3,      // Global
    Lowest = 4,   // AutoImport
}
```

### 3.3 解析结果

```rust
// crates/sage-core/src/config/credential/resolved.rs
pub struct ResolvedCredential {
    pub value: String,
    pub source: CredentialSource,
    pub priority: CredentialPriority,
}

pub struct ResolvedCredentials {
    credentials: HashMap<String, ResolvedCredential>,
}

impl ResolvedCredentials {
    /// 获取特定 Provider 的 API Key
    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.credentials.get(provider).map(|c| c.value.as_str())
    }
}
```

### 3.4 统一配置加载器

```rust
// crates/sage-core/src/config/credential/unified_loader.rs
pub struct UnifiedConfigLoader {
    config_file: Option<PathBuf>,
    cli_overrides: CliOverrides,
}

impl UnifiedConfigLoader {
    /// 加载配置（永不失败）
    pub fn load(self) -> LoadedConfig {
        // 1. 加载配置文件
        let mut config = self.load_config_file();

        // 2. 应用 CLI 覆盖
        self.apply_cli_overrides(&mut config);

        // 3. 解析凭证
        let resolver = CredentialResolver::with_defaults();
        let credentials = resolver.resolve_all();

        // 4. 注入凭证到配置
        self.inject_credentials(&mut config, &credentials);

        // 5. 获取配置状态
        let status = resolver.get_status();

        LoadedConfig {
            config,
            credentials,
            status,
        }
    }
}
```

---

## 四、配置验证

### 4.1 验证器设计

```rust
// crates/sage-core/src/config/validation/mod.rs
pub struct ConfigValidator;

impl ConfigValidator {
    /// 完整验证
    pub fn validate(config: &Config) -> SageResult<()> {
        validate_providers(config)?;
        validate_models(config)?;
        validate_limits(config)?;
        validate_paths(config)?;
        validate_tools(config)?;
        Ok(())
    }

    /// 单项验证
    pub fn validate_providers(config: &Config) -> SageResult<()>;
    pub fn validate_models(config: &Config) -> SageResult<()>;
    pub fn validate_limits(config: &Config) -> SageResult<()>;
    pub fn validate_paths(config: &Config) -> SageResult<()>;
    pub fn validate_tools(config: &Config) -> SageResult<()>;
    pub fn validate_logging(config: &Config) -> SageResult<()>;
}
```

### 4.2 验证规则示例

```rust
// crates/sage-core/src/config/validation/provider.rs
pub fn validate_providers(config: &Config) -> SageResult<()> {
    // 1. 默认 Provider 必须存在
    if !config.model_providers.contains_key(&config.default_provider) {
        return Err(SageError::config(format!(
            "Default provider '{}' not found",
            config.default_provider
        )));
    }

    // 2. 验证每个 Provider 的配置
    for (name, params) in &config.model_providers {
        params.validate().map_err(|e| {
            SageError::config(format!("Invalid '{}': {}", name, e))
        })?;
    }

    Ok(())
}
```

---

## 五、运行时持久化

### 5.1 持久化管理器

```rust
// crates/sage-core/src/config/persistence.rs
pub struct ConfigPersistence {
    config_path: PathBuf,        // ~/.sage/config.json
    credentials_path: PathBuf,   // ~/.sage/credentials.json
}

impl ConfigPersistence {
    /// 使用点号路径设置字段
    pub fn set_field(&self, path: &str, value: Value) -> SageResult<()> {
        let mut config = self.load_config_json()?;
        set_nested_value(&mut config, path, value);
        self.save_config_json(&config)
    }

    /// 获取字段值
    pub fn get_field(&self, path: &str) -> Option<Value> {
        let config = self.load_config_json().ok()?;
        get_nested_value(&config, path)
    }

    /// 设置 API Key
    pub fn set_api_key(&self, provider: &str, api_key: &str) -> SageResult<()> {
        let mut creds = self.load_credentials_json()?;

        if !creds.get("api_keys").map(|v| v.is_object()).unwrap_or(false) {
            creds["api_keys"] = Value::Object(serde_json::Map::new());
        }

        creds["api_keys"][provider] = Value::String(api_key.to_string());
        self.save_credentials_json(&creds)
    }

    /// 设置默认 Provider
    pub fn set_default_provider(&self, provider: &str) -> SageResult<()> {
        self.set_field("default_provider", Value::String(provider.to_string()))
    }
}
```

### 5.2 ConfigUpdate 类型

```rust
pub struct ConfigUpdate {
    pub path: String,
    pub value: Value,
    pub description: String,
}

impl ConfigPersistence {
    /// 批量更新
    pub fn apply_updates(&self, updates: &[ConfigUpdate]) -> SageResult<()> {
        let mut config = self.load_config_json()?;

        for update in updates {
            set_nested_value(&mut config, &update.path, update.value.clone());
        }

        self.save_config_json(&config)
    }
}
```

---

## 六、Provider 注册表

### 6.1 Provider 信息

```rust
// crates/sage-core/src/config/provider_registry.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,           // "anthropic"
    pub name: String,         // "Anthropic"
    pub description: String,
    pub api_base_url: String,
    pub env_var: String,      // "ANTHROPIC_API_KEY"
    pub help_url: Option<String>,
    pub models: Vec<ModelInfo>,
    pub requires_api_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,               // "claude-sonnet-4-20250514"
    pub name: String,             // "Claude 4 Sonnet"
    pub default: bool,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
}
```

### 6.2 注册表与缓存

```rust
pub struct ProviderRegistry {
    cache_dir: PathBuf,
    cache_ttl: Duration,  // 默认 24 小时
    providers: Option<Vec<ProviderInfo>>,
}

impl ProviderRegistry {
    /// 获取所有可用 Provider
    pub fn get_providers(&mut self) -> &[ProviderInfo] {
        if self.providers.is_none() {
            self.providers = Some(self.load_providers());
        }
        self.providers.as_ref().unwrap()
    }

    /// 强制刷新
    pub fn refresh(&mut self) {
        self.providers = None;
        let providers = self.embedded_providers();
        self.save_cache(&providers).ok();
        self.providers = Some(providers);
    }
}
```

---

## 七、模型参数配置

### 7.1 ModelParameters

```rust
// crates/sage-core/src/config/model_params.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelParameters {
    pub model: String,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub parallel_tool_calls: Option<bool>,
    pub max_retries: Option<u32>,
    pub base_url: Option<String>,
    pub api_version: Option<String>,
    pub stop_sequences: Option<Vec<String>>,
}

impl ModelParameters {
    /// 深度合并
    pub fn merge(&mut self, other: ModelParameters) {
        // 非空字符串才覆盖
        if !other.model.is_empty() {
            self.model = other.model;
        }

        // Option 类型：Some 才覆盖
        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        // ... 其他字段
    }

    /// 验证
    pub fn validate(&self) -> SageResult<()> {
        if self.model.is_empty() {
            return Err(SageError::config("Model name cannot be empty"));
        }

        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(SageError::config("Temperature must be 0.0-2.0"));
            }
        }

        Ok(())
    }
}
```

---

## 八、开发指南

### 8.1 添加新配置字段

1. 在 `Config` 结构添加字段（带 `#[serde(default)]`）：
```rust
pub struct Config {
    // ...
    #[serde(default)]
    pub new_feature: bool,
}
```

2. 在 `Config::merge` 中处理合并逻辑

3. 如需要，在 `validation/` 添加验证规则

4. 更新 `sage_config.json.example`

### 8.2 添加新 Provider

1. 在 `provider_registry.rs` 的 `embedded_providers()` 添加：
```rust
ProviderInfo {
    id: "new_provider".to_string(),
    name: "New Provider".to_string(),
    env_var: "NEW_PROVIDER_API_KEY".to_string(),
    models: vec![
        ModelInfo {
            id: "new-model".to_string(),
            name: "New Model".to_string(),
            default: true,
            ..Default::default()
        }
    ],
    ..Default::default()
}
```

2. 在 `provider_defaults.rs` 添加默认参数

3. 在 `credential/resolver.rs` 添加环境变量映射

### 8.3 配置文件格式

**sage_config.json 示例：**
```json
{
  "default_provider": "anthropic",
  "max_steps": 100,
  "total_token_budget": 1000000,
  "model_providers": {
    "anthropic": {
      "model": "claude-sonnet-4-20250514",
      "max_tokens": 8192,
      "temperature": 0.7
    },
    "openai": {
      "model": "gpt-4o",
      "max_tokens": 4096
    }
  },
  "tools": {
    "max_execution_time": 300,
    "allow_parallel_execution": true
  },
  "mcp": {
    "servers": [
      {
        "name": "filesystem",
        "command": "npx",
        "args": ["-y", "@anthropic/mcp-server-filesystem"]
      }
    ]
  }
}
```

**credentials.json 示例：**
```json
{
  "api_keys": {
    "anthropic": "sk-ant-xxx",
    "openai": "sk-xxx"
  }
}
```

---

## 九、配置层次结构

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Args                             │
│                    (最高优先级)                               │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Environment Vars                          │
│                SAGE_*, ANTHROPIC_API_KEY, ...               │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                  Project Config                              │
│              ./sage_config.json                              │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                  Global Config                               │
│              ~/.sage/config.json                             │
│              ~/.sage/credentials.json                        │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                     Defaults                                 │
│              (代码内置默认值)                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 十、相关模块

- `sage-llm-integration` - LLM 客户端（使用 ProviderConfig）
- `sage-session-management` - 会话管理（使用配置）
- `sage-mcp-protocol` - MCP 协议（使用 McpConfig）
- `sage-agent-execution` - Agent 执行（使用 ExecutionOptions）

---

*最后更新: 2026-01-10*
