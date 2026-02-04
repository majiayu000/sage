# Slash Command 系统设计分析

基于对 open-claude-code 和 sage 项目的深入分析。

## 一、架构对比

### Open-Claude-Code (JavaScript/React)

| 特性 | 实现方式 |
|------|---------|
| 命令定义 | 对象字面量 `{ type, name, description, call() }` |
| 命令类型 | `local-jsx` (React组件), `local` (文本), `prompt` (AI提示) |
| 权限管理 | 细粒度规则系统 (Policy > User > Project > Local) |
| 沙箱执行 | 命令级沙箱，危险命令检测 |
| 输出处理 | 敏感数据编辑、长输出总结 |

### Sage (Rust)

| 特性 | 实现方式 |
|------|---------|
| 命令定义 | `SlashCommand` 结构体 + 构建器模式 |
| 命令类型 | `InteractiveCommand` 枚举 + `SlashCommandAction` |
| 权限管理 | 工具级权限 |
| 命令发现 | 文件系统扫描 (.sage/commands/, ~/.config/sage/commands/) |
| 参数替换 | `$ARG1`, `$ARGUMENTS`, `$ARGUMENTS_JSON` |

## 二、Sage Model 切换流程分析

### 当前实现

```
用户输入 /model
    ↓
process_slash_command() [slash_commands.rs:32]
    ↓
CommandExecutor.process() → InteractiveCommand::ModelSelect
    ↓
handle_interactive_command_v2() [slash_commands.rs:163]
    ↓
获取 Provider 配置和凭证
    ↓
ModelsApiClient.fetch_*_models() 从 API 获取模型列表
    ↓
返回 SlashCommandAction::ModelSelect { models }
    ↓
executor_loop() [executor.rs:198] 进入模型选择模式
    ↓
用户选择模型 (↑↓ 导航, Enter 确认)
    ↓
发送 UiCommand::Submit("/model <selected>")
    ↓
SlashCommandAction::SwitchModel { model }
    ↓
executor.switch_model(&model) [executor.rs:174]
    ↓
更新 UI 状态 s.session.model = model
```

### 关键代码位置

| 文件 | 行号 | 功能 |
|------|------|------|
| `slash_commands.rs` | 163-243 | ModelSelect 处理，获取模型列表 |
| `executor.rs` | 172-196 | SwitchModel 处理，调用 executor.switch_model() |
| `executor.rs` | 198-207 | ModelSelect 处理，进入选择模式 |
| `mod.rs` (rnk_app) | 136-173 | 模型选择键盘事件处理 |
| `unified/mod.rs` | 294-304 | switch_model() 实现 |

## 三、问题诊断

### 问题：Model 切换后不生效

**根本原因**：`switch_model()` 方法正确更新了 `LlmOrchestrator`，但 UI 状态更新和实际 LLM 调用之间存在断层。

**代码分析**：

```rust
// executor.rs:172-196
Ok(SlashCommandAction::SwitchModel { model }) => {
    match executor.switch_model(&model) {
        Ok(_) => {
            // ✅ 更新 UI 状态
            {
                let mut s = state.write();
                s.session.model = model.clone();
            }
            // ✅ 显示成功消息
            rnk::println(
                Text::new(format!("✓ Switched to model: {}", model))
                    .color(Color::Green)
                    .into_element(),
            );
        }
        Err(e) => { ... }
    }
}
```

```rust
// unified/mod.rs:294-304
pub fn switch_model(&mut self, model: &str) -> SageResult<String> {
    // ✅ 更新配置
    self.config.set_default_model(model.to_string());

    // ✅ 重建 LLM orchestrator
    self.llm_orchestrator = LlmOrchestrator::from_config(&self.config)
        .map_err(|e| SageError::config(format!("Failed to switch model: {}", e)))?;

    tracing::info!("Switched to model: {}", model);
    Ok(model.to_string())
}
```

**潜在问题点**：

1. **Config.set_default_model() 实现** - 需要检查是否正确更新了 model_providers 中的 model 字段
2. **LlmOrchestrator.from_config()** - 需要确认是否正确读取了更新后的 model
3. **Provider 关联** - 切换模型时可能需要同时考虑 provider 的变化

## 四、修复方案

### 方案 1：检查 Config.set_default_model() 实现

需要确保该方法正确更新了当前 provider 的 model 字段：

```rust
// 期望的实现
pub fn set_default_model(&mut self, model: String) {
    if let Some(provider_params) = self.model_providers.get_mut(&self.default_provider) {
        provider_params.model = model;
    }
}
```

### 方案 2：增强 switch_model() 方法

```rust
pub fn switch_model(&mut self, model: &str) -> SageResult<String> {
    // 1. 更新配置
    self.config.set_default_model(model.to_string());

    // 2. 验证配置已更新
    let current_model = self.config.get_default_model();
    if current_model != model {
        return Err(SageError::config("Failed to update model in config"));
    }

    // 3. 重建 LLM orchestrator
    self.llm_orchestrator = LlmOrchestrator::from_config(&self.config)?;

    // 4. 验证 orchestrator 使用了新模型
    // (需要添加 get_current_model() 方法)

    Ok(model.to_string())
}
```

### 方案 3：支持跨 Provider 切换

当用户选择的模型属于不同 provider 时，需要同时切换 provider：

```rust
pub fn switch_model(&mut self, model: &str) -> SageResult<String> {
    // 检测模型属于哪个 provider
    let target_provider = self.detect_provider_for_model(model)?;

    // 如果需要切换 provider
    if target_provider != self.config.default_provider {
        self.config.default_provider = target_provider.clone();
    }

    // 更新模型
    self.config.set_default_model(model.to_string());

    // 重建 orchestrator
    self.llm_orchestrator = LlmOrchestrator::from_config(&self.config)?;

    Ok(model.to_string())
}
```

## 五、Open-Claude-Code 的 Model 切换参考

Open-Claude-Code 的模型切换设计：

1. **模型列表来源**：
   - API 动态获取（优先）
   - 内置模型列表（降级）

2. **切换方式**：
   - 运行时切换（需要重启）
   - 命令行参数 `--model`
   - 配置文件 `default_model`

3. **持久化**：
   - 会话级别（不持久化）
   - 配置文件级别（持久化）

## 六、建议的改进

### 6.1 增加模型验证

在切换前验证模型是否可用：

```rust
async fn validate_model(&self, model: &str) -> SageResult<bool> {
    // 检查模型是否在可用列表中
    // 或者发送一个简单的 API 请求验证
}
```

### 6.2 增加 Provider 自动检测

```rust
fn detect_provider_for_model(&self, model: &str) -> Option<String> {
    // claude-* -> anthropic
    // gpt-*, o1*, o3* -> openai
    // gemini-* -> google
    // ...
}
```

### 6.3 增加切换确认

在 UI 中显示切换前后的对比：

```
Current: claude-sonnet-4-20250514 (anthropic)
Switch to: claude-opus-4-5-20251101 (anthropic)
Confirm? [Y/n]
```

### 6.4 支持模型别名

```rust
// 用户可以使用简短名称
/model opus -> claude-opus-4-5-20251101
/model sonnet -> claude-sonnet-4-20250514
/model gpt4 -> gpt-4-turbo
```

## 七、代码验证结果

经过完整的代码审查，**模型切换的核心逻辑是正确的**：

### 验证的代码路径

1. **Config.set_default_model()** ✅
   - 正确更新 `model_providers[default_provider].model`
   - 位置: `config/config.rs:103-107`

2. **LlmOrchestrator.from_config()** ✅
   - 正确调用 `config.default_model_parameters()`
   - 正确调用 `to_llm_parameters()` 获取 model 名称
   - 位置: `agent/unified/llm_orchestrator.rs:48-94`

3. **LlmClient.new()** ✅
   - 正确将 `model_params` 传递给 provider instance
   - 位置: `llm/client/constructor.rs:59-182`

4. **AnthropicProvider.chat()** ✅
   - 正确使用 `self.model_params.model` 构建 API 请求
   - 位置: `llm/providers/anthropic.rs:51`

### 可能的问题点

如果模型切换仍然不生效，可能的原因：

1. **UI 状态未同步** - `background_loop` 中的 session 信息更新逻辑
2. **日志级别** - 需要启用 `RUST_LOG=sage_core=debug` 查看详细日志
3. **缓存问题** - 某些地方可能缓存了旧的 client 实例

### 调试建议

```bash
# 启用详细日志
RUST_LOG=sage_core::agent::unified=debug,sage_core::llm=debug cargo run -p sage-cli
```

## 八、已实施的修复

### 8.1 添加 ModelSwitched 事件

**文件**: `crates/sage-core/src/ui/bridge/events.rs`

```rust
/// Model switched during session
ModelSwitched {
    old_model: String,
    new_model: String,
},
```

### 8.2 更新 EventAdapter 处理 ModelSwitched

**文件**: `crates/sage-core/src/ui/bridge/adapter.rs`

```rust
AgentEvent::ModelSwitched { new_model, .. } => {
    state.session.model = new_model;
}
```

### 8.3 增强 switch_model 方法

**文件**: `crates/sage-core/src/agent/unified/mod.rs`

- 添加切换前后的日志
- 添加配置更新验证
- 添加 orchestrator 模型验证

### 8.4 更新 executor 发送 ModelSwitched 事件

**文件**: `crates/sage-cli/src/ui/rnk_app/executor.rs`

```rust
// Emit model switched event to update adapter state
event_ctx.emit(AgentEvent::ModelSwitched {
    old_model,
    new_model: model.clone(),
});
```

## 九、下一步行动

1. ✅ 代码逻辑已验证正确
2. ✅ 添加 ModelSwitched 事件
3. ✅ 更新 adapter 处理事件
4. ✅ 增强 switch_model 日志
5. 测试实际切换流程
6. 更新版本号并提交
