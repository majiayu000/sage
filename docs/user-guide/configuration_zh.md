# Configuration Guide / 配置指南

## Overview / 概述

Sage Agent uses a JSON configuration file to manage LLM provider settings, API keys, execution parameters, and tool configurations.
Sage Agent 使用 JSON 配置文件来管理 LLM 提供商设置、API 密钥、执行参数和工具配置。

**Default Configuration File / 默认配置文件:** `sage_config.json`

Provider endpoints are user settings. The execution loader ignores `base_url` in project JSON, TOML, and YAML files, including a project file passed with `--config-file`, so project configuration cannot redirect your API credentials. Set custom endpoints in `model_providers.<provider>.base_url` in `~/.sage/config.json`, a provider environment variable such as `ANTHROPIC_BASE_URL`, or a `model_base_url` override when using the configuration API. User config endpoints take precedence over environment endpoints; explicit overrides take precedence over both.

提供商 endpoint 属于用户设置。执行配置加载器会忽略项目 JSON、TOML 和 YAML 文件中的 `base_url`，包括通过 `--config-file` 传入的项目文件，防止项目配置改变 API 凭据的发送地址。自定义 endpoint 请写入 `~/.sage/config.json` 的 `model_providers.<provider>.base_url`，使用 `ANTHROPIC_BASE_URL` 等提供商环境变量，或在配置 API 中传入 `model_base_url` 覆盖项。用户配置中的 endpoint 优先于环境变量，显式覆盖项优先于两者。

---

## Quick Setup / 快速设置

### MCP startup consent / MCP 启动授权

Automatically discovered `sage_config.json`, `sage_config.toml`, `sage_config.yaml`, and `sage_config.yml` files do not supply MCP settings. Ordinary project settings still load. MCP settings in the user config (`~/.sage/config.json`) remain trusted.
自动发现的上述工作区配置不提供 MCP 设置；普通项目设置仍会加载。用户全局配置中的 MCP 设置仍受信任。

After reviewing a project's MCP commands, explicitly select its configuration with `sage --config-file sage_config.json`. This selection grants consent to the file's MCP commands for that invocation, including in print mode. SDK callers can use `SageAgentSdk::with_config_file` or a programmatically constructed trusted `Config`.
检查项目的 MCP 命令后，可显式指定上述配置文件；这会授权本次调用使用该文件的 MCP 命令，也适用于非交互模式。SDK 可显式选择配置文件或传入受信任的配置对象。

### Step 1: Initialize Configuration / 初始化配置

```bash
# Create default configuration file
# 创建默认配置文件
sage config init

# This creates: sage_config.json
# 这将创建: sage_config.json
```

### Step 2: Add Your API Keys / 添加 API 密钥

Edit `sage_config.json` and replace placeholder API keys:
编辑 `sage_config.json` 并替换占位符 API 密钥:

```json
{
  "default_provider": "anthropic",
  "model_providers": {
    "anthropic": {
      "api_key": "sk-ant-xxxxxxxxxxxxx",  // ← Replace with your key
      "model": "claude-sonnet-4-20250514"
    },
    "openai": {
      "api_key": "sk-xxxxxxxxxxxxx",      // ← Replace with your key
      "model": "gpt-4"
    }
  }
}
```

### Step 3: Verify Configuration / 验证配置

```bash
# Validate configuration file
# 验证配置文件
sage config validate

# View current configuration
# 查看当前配置
sage config show
```

---

## Configuration File Structure / 配置文件结构

### Complete Example / 完整示例

```json
{
  "default_provider": "google",
  "max_steps": 20,
  "total_token_budget": 100000,
  "enable_lakeview": false,
  "lakeview_config": null,

  "logging": {
    "format": "pretty",
    "level": "info",
    "log_file": null,
    "log_to_console": true,
    "log_to_file": false
  },

  "model_providers": {
    "anthropic": {
      "api_key": "your-anthropic-api-key",
      "model": "claude-sonnet-4-20250514",
      "max_tokens": 4096,
      "temperature": 0.7,
      "max_retries": 3,
      "api_version": "2023-06-01",
      "base_url": null,
      "parallel_tool_calls": false,
      "stop_sequences": null,
      "top_k": null,
      "top_p": 1.0
    },
    "google": {
      "api_key": "your-google-api-key",
      "model": "gemini-2.5-pro",
      "max_tokens": 120000,
      "temperature": 0.7,
      "max_retries": 3,
      "parallel_tool_calls": true
    },
    "openai": {
      "api_key": "your-openai-api-key",
      "model": "gpt-4",
      "max_tokens": 4096,
      "temperature": 0.7,
      "max_retries": 3,
      "parallel_tool_calls": true
    },
    "ollama": {
      "base_url": "http://localhost:11434",
      "model": "llama3",
      "max_tokens": 4096,
      "temperature": 0.7
    }
  },

  "tools": {
    "enabled_tools": [
      "str_replace_based_edit_tool",
      "sequentialthinking",
      "json_edit_tool",
      "task_done",
      "bash"
    ],
    "allow_parallel_execution": true,
    "max_execution_time": 300,
    "tool_settings": {}
  },

  "working_directory": null,

  "trajectory": {
    "enabled": false,
    "directory": "trajectories",
    "auto_save": true,
    "save_interval_steps": 5
  }
}
```

---

## API Key Configuration / API 密钥配置

### Method 1: Direct Configuration / 方法一:直接配置

Add API keys directly in the configuration file:
在配置文件中直接添加 API 密钥:

```json
{
  "model_providers": {
    "anthropic": {
      "api_key": "sk-ant-api03-xxxxx"
    }
  }
}
```

### Method 2: Environment Variables (Recommended) / 方法二:环境变量(推荐)

Use environment variable substitution for better security:
使用环境变量替换以提高安全性:

**Configuration File / 配置文件:**
```json
{
  "model_providers": {
    "anthropic": {
      "api_key": "${ANTHROPIC_API_KEY}"
    },
    "openai": {
      "api_key": "${OPENAI_API_KEY}"
    },
    "google": {
      "api_key": "${GOOGLE_API_KEY}"
    }
  }
}
```

**Shell Environment / Shell 环境:**
```bash
# Add to ~/.bashrc or ~/.zshrc
# 添加到 ~/.bashrc 或 ~/.zshrc

export ANTHROPIC_API_KEY="sk-ant-api03-xxxxx"
export OPENAI_API_KEY="sk-xxxxx"
export GOOGLE_API_KEY="AIzaSyxxxxx"

# Reload shell configuration
# 重新加载 shell 配置
source ~/.bashrc  # or source ~/.zshrc
```

---

## Provider-Specific Configuration / 提供商特定配置

### Anthropic (Claude)

```json
{
  "anthropic": {
    "api_key": "${ANTHROPIC_API_KEY}",
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 4096,
    "temperature": 0.7,
    "api_version": "2023-06-01",
    "max_retries": 3,
    "enable_prompt_caching": true  // Enable for cost savings
  }
}
```

**Recommended Models / 推荐模型:**
- `claude-sonnet-4-20250514` - Best for complex tasks
- `claude-3-sonnet-20240229` - Balanced performance
- `claude-3-haiku-20240307` - Fast and economical

**Get API Key / 获取 API 密钥:**
https://console.anthropic.com/settings/keys

---

### OpenAI (GPT)

```json
{
  "openai": {
    "api_key": "${OPENAI_API_KEY}",
    "model": "gpt-4",
    "max_tokens": 4096,
    "temperature": 0.7,
    "max_retries": 3,
    "parallel_tool_calls": true
  }
}
```

**Recommended Models / 推荐模型:**
- `gpt-4-turbo` - Latest GPT-4 with lower cost
- `gpt-4` - Most capable model
- `gpt-3.5-turbo` - Fast and cost-effective

**Get API Key / 获取 API 密钥:**
https://platform.openai.com/api-keys

---

### Google (Gemini)

```json
{
  "google": {
    "api_key": "${GOOGLE_API_KEY}",
    "model": "gemini-2.5-pro",
    "max_tokens": 120000,
    "temperature": 0.7,
    "max_retries": 3,
    "parallel_tool_calls": true
  }
}
```

**Recommended Models / 推荐模型:**
- `gemini-2.5-pro` - Latest and most capable
- `gemini-1.5-pro` - Excellent performance
- `gemini-1.5-flash` - Fast with large context

**Get API Key / 获取 API 密钥:**
https://makersuite.google.com/app/apikey

---

### Ollama (Local Models) / Ollama(本地模型)

```json
{
  "ollama": {
    "base_url": "http://localhost:11434",
    "model": "llama3",
    "max_tokens": 4096,
    "temperature": 0.7,
    "api_key": null  // No API key needed for local
  }
}
```

**Prerequisites / 前置条件:**
1. Install Ollama: https://ollama.ai/
2. Pull a model: `ollama pull llama3`
3. Start Ollama service: `ollama serve`

**Recommended Models / 推荐模型:**
- `llama3` - Meta's latest model
- `codellama` - Code-specialized model
- `mistral` - High-quality open model

---

## Global Settings / 全局设置

### Execution Parameters / 执行参数

```json
{
  "default_provider": "anthropic",     // Which provider to use by default
  "max_steps": 20,                    // Maximum execution steps per task
  "total_token_budget": 100000,       // Total token limit
  "working_directory": null           // Default working directory
}
```

### Logging Configuration / 日志配置

```json
{
  "logging": {
    "format": "pretty",              // "pretty" or "json"
    "level": "info",                 // "trace", "debug", "info", "warn", "error"
    "log_to_console": true,          // Print logs to console
    "log_to_file": false,            // Save logs to file
    "log_file": null                 // Path to log file
  }
}
```

**Log Levels / 日志级别:**
- `trace` - Most verbose, all details
- `debug` - Development debugging
- `info` - Standard operational messages
- `warn` - Warning messages only
- `error` - Error messages only

---

## Tool Configuration / 工具配置

### Enable/Disable Tools / 启用/禁用工具

```json
{
  "tools": {
    "enabled_tools": [
      "str_replace_based_edit_tool",  // File editing
      "bash",                         // Shell commands
      "json_edit_tool",               // JSON editing
      "task_done",                    // Task completion
      "sequentialthinking"            // Planning
    ],
    "allow_parallel_execution": true,
    "max_execution_time": 300        // Timeout in seconds
  }
}
```

**Available Tools / 可用工具:**
- `bash` - Execute shell commands
- `str_replace_based_edit_tool` - Edit files via string replacement
- `json_edit_tool` - Edit JSON files
- `task_done` - Mark tasks as complete
- `sequentialthinking` - Planning and reasoning
- And many more (see `sage tools`)

---

## Trajectory Recording / 轨迹记录

Enable execution recording for debugging and analysis:
启用执行记录用于调试和分析:

```json
{
  "trajectory": {
    "enabled": true,                  // Enable trajectory recording
    "directory": "trajectories",      // Output directory
    "auto_save": true,                // Auto-save during execution
    "save_interval_steps": 5          // Save every N steps
  }
}
```

**Use Cases / 使用场景:**
- Debug complex task execution
- Analyze agent performance
- Replay and review past executions
- Training and fine-tuning

---

## Configuration Validation / 配置验证

### Validate Configuration / 验证配置

```bash
# Validate default config file
# 验证默认配置文件
sage config validate

# Validate specific file
# 验证特定文件
sage config validate --config-file custom_config.json
```

### Common Validation Errors / 常见验证错误

**1. Invalid JSON syntax / JSON 语法错误**
```
Error: Expected ',' or '}' at line 10
```
Solution: Check for missing commas, brackets, or quotes

**2. Missing required fields / 缺少必填字段**
```
Error: Missing required field 'api_key'
```
Solution: Add the required field to your configuration

**3. Invalid provider / 无效的提供商**
```
Error: Unknown provider 'invalid_provider'
```
Solution: Use valid providers: anthropic, openai, google, ollama, etc.

---

## Advanced Configuration / 高级配置

### Custom Base URL / 自定义基础 URL

For using proxies or custom endpoints:
用于使用代理或自定义端点:

```json
{
  "openai": {
    "api_key": "sk-xxxxx",
    "model": "gpt-4",
    "base_url": "https://your-proxy.com/v1"
  }
}
```

### Custom Stop Sequences / 自定义停止序列

```json
{
  "anthropic": {
    "stop_sequences": ["END", "STOP", "###"]
  }
}
```

### Temperature and Sampling / 温度和采样

```json
{
  "anthropic": {
    "temperature": 0.7,   // 0.0 = deterministic, 1.0 = creative
    "top_p": 1.0,         // Nucleus sampling
    "top_k": null         // Top-k sampling
  }
}
```

---

## Multiple Configuration Files / 多个配置文件

You can maintain different configurations for different scenarios:
您可以为不同场景维护不同的配置:

```bash
# Production configuration
# 生产环境配置
sage "Task" --config-file prod_config.json

# Development configuration
# 开发环境配置
sage "Task" --config-file dev_config.json

# Testing with local models
# 使用本地模型测试
sage "Task" --config-file ollama_config.json
```

**Example File Structure / 示例文件结构:**
```
project/
├── sage_config.json          # Default
├── prod_config.json          # Production
├── dev_config.json           # Development
└── test_config.json          # Testing
```

---

## Security Best Practices / 安全最佳实践

### 1. Never Commit API Keys / 永远不要提交 API 密钥

```bash
# Add to .gitignore
# 添加到 .gitignore
echo "sage_config.json" >> .gitignore
echo "*_config.json" >> .gitignore
```

### 2. Use Environment Variables / 使用环境变量

Store sensitive data in environment variables, not config files.
将敏感数据存储在环境变量中,而不是配置文件中。

### 3. Restrict File Permissions / 限制文件权限

```bash
# Make config file readable only by owner
# 使配置文件仅所有者可读
chmod 600 sage_config.json
```

### 4. Use Separate Keys / 使用独立密钥

Create separate API keys for development and production.
为开发和生产创建独立的 API 密钥。

---

## Troubleshooting / 故障排除

### Issue: API Key Not Found / 问题:未找到 API 密钥

**Error / 错误:**
```
Error: API key not configured for provider 'anthropic'
```

**Solution / 解决方案:**
1. Verify API key is set in config file or environment
2. Check environment variable name matches exactly
3. Reload shell after setting environment variables

### Issue: Rate Limit Exceeded / 问题:超出速率限制

**Error / 错误:**
```
Error: Rate limit exceeded (429)
```

**Solution / 解决方案:**
1. Wait and retry (automatic with `max_retries`)
2. Reduce `max_steps` to limit requests
3. Switch to a different provider
4. Upgrade your API plan

### Issue: Invalid Model Name / 问题:无效的模型名称

**Error / 错误:**
```
Error: Model 'invalid-model' not found
```

**Solution / 解决方案:**
Check model name spelling and availability for your API tier.
检查模型名称拼写和您的 API 层级可用性。

---

## Next Steps / 下一步

After configuration is complete:
配置完成后:

1. **Test Configuration** / **测试配置**
   ```bash
   sage "echo Hello, Sage"
   ```

2. **Read Quick Start Guide** / **阅读快速入门指南**
   → [Quick Start Guide](../../README_zh.md#-quick-start)

3. **Explore Examples** / **探索示例**
   ```bash
   make examples
   ```

---

## Additional Resources / 其他资源

- **Configuration Template**: `configs/sage_config.example.json`
- **Environment Variables**: See `.env.example`
- **API Documentation**: `/docs/api/`
