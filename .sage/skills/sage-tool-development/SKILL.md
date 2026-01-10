---
name: sage-tool-development
description: Sage 工具开发规范，包含 TOOL.md 描述文件标准、权限系统、沙箱集成
when_to_use: 当需要开发新工具、修改现有工具、或设计工具系统时使用
allowed_tools:
  - Read
  - Grep
  - Glob
  - Edit
  - Write
  - Bash
user_invocable: true
priority: 90
---

# Sage 工具开发规范

## 工具结构标准（学习 Crush）

每个工具必须包含代码实现和描述文件：

```
crates/sage-tools/src/
├── bash/
│   ├── mod.rs           # 工具实现
│   ├── TOOL.md          # 工具描述（注入 system prompt）
│   ├── safety.rs        # 安全检查（可选）
│   └── tests.rs         # 测试
├── edit/
│   ├── mod.rs
│   ├── TOOL.md
│   ├── diff.rs          # 辅助功能
│   └── tests.rs
├── read/
│   ├── mod.rs
│   ├── TOOL.md
│   └── tests.rs
...
```

## TOOL.md 格式规范

```markdown
---
name: ToolName
description: 简短描述（一行）
dangerous: true|false
requires_permission: true|false
category: file|shell|web|mcp
---

## Description

详细描述工具的功能和用途。

## Parameters

| 参数名 | 类型 | 必需 | 默认值 | 描述 |
|-------|------|------|-------|------|
| `param1` | string | 是 | - | 参数说明 |
| `param2` | number | 否 | 100 | 参数说明 |

## Usage Notes

- 使用注意事项
- 最佳实践
- 常见陷阱

## Examples

\`\`\`
示例调用
\`\`\`

## Security Considerations

安全相关说明（如有）
```

## 工具实现模板

```rust
//! {工具名} - {简短描述}

use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolResult, ToolSchema, ToolError};
use serde::{Deserialize, Serialize};

/// 工具参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {ToolName}Params {
    /// 参数描述
    pub required_param: String,

    /// 可选参数描述
    #[serde(default)]
    pub optional_param: Option<i32>,
}

/// {工具名}工具
pub struct {ToolName}Tool {
    // 工具状态（如需要）
}

impl {ToolName}Tool {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Tool for {ToolName}Tool {
    fn name(&self) -> &str {
        "{tool_name}"  // 小写，用于调用
    }

    fn description(&self) -> &str {
        include_str!("TOOL.md")  // 从描述文件加载
            .lines()
            .skip_while(|l| l.starts_with("---") || !l.starts_with("## Description"))
            .skip(1)
            .take_while(|l| !l.starts_with("##"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), vec![
            // 定义参数 schema
        ])
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        // 1. 解析参数
        let params: {ToolName}Params = call.parse_params()?;

        // 2. 验证参数
        self.validate(&params)?;

        // 3. 执行逻辑
        let result = self.do_execute(&params).await?;

        // 4. 返回结果
        Ok(ToolResult::success(
            &result,
            self.name(),
            "output_type"
        ))
    }

    fn is_dangerous(&self) -> bool {
        false  // 根据工具性质设置
    }

    fn requires_permission(&self) -> bool {
        false  // 根据工具性质设置
    }
}

impl {ToolName}Tool {
    fn validate(&self, params: &{ToolName}Params) -> Result<(), ToolError> {
        // 参数验证逻辑
        Ok(())
    }

    async fn do_execute(&self, params: &{ToolName}Params) -> Result<String, ToolError> {
        // 核心执行逻辑
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_execution() {
        // 测试基本功能
    }

    #[tokio::test]
    async fn test_error_handling() {
        // 测试错误处理
    }
}
```

## 权限系统集成

### 权限级别

```rust
pub enum PermissionLevel {
    /// 无需权限（Read, Glob, Grep）
    None,

    /// 需要用户确认（Edit, Write）
    Confirm,

    /// 需要明确授权（Bash, 危险操作）
    Authorize,

    /// 禁止执行（根据规则）
    Deny,
}
```

### 在工具中声明权限

```rust
impl Tool for MyTool {
    fn permission_level(&self, call: &ToolCall) -> PermissionLevel {
        // 根据调用参数动态决定权限级别
        if self.is_destructive(call) {
            PermissionLevel::Authorize
        } else {
            PermissionLevel::Confirm
        }
    }
}
```

## 沙箱集成

危险工具必须在沙箱中执行：

```rust
impl Tool for BashTool {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let sandbox = SandboxBuilder::new()
            .with_timeout(Duration::from_secs(120))
            .with_memory_limit(512 * 1024 * 1024)  // 512MB
            .with_network(false)  // 禁用网络（除非必要）
            .build()?;

        sandbox.execute(|| {
            // 在沙箱中执行命令
        }).await
    }
}
```

## 工具分类

| 类别 | 工具 | 权限 | 沙箱 |
|-----|------|------|------|
| **只读** | Read, Glob, Grep, WebFetch | None | 否 |
| **写入** | Edit, Write | Confirm | 否 |
| **执行** | Bash | Authorize | 是 |
| **系统** | TaskOutput, KillShell | Authorize | 否 |
| **MCP** | 动态工具 | 按配置 | 按配置 |

## 工具注册

```rust
// sage-tools/src/lib.rs
pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    // 只读工具
    registry.register(ReadTool::new());
    registry.register(GlobTool::new());
    registry.register(GrepTool::new());

    // 写入工具
    registry.register(EditTool::new());
    registry.register(WriteTool::new());

    // 执行工具（需要沙箱）
    registry.register(BashTool::new().with_sandbox());

    // 加载 TOOL.md 描述
    registry.load_descriptions("src/");
}
```

## 检查清单

开发新工具前确认：

- [ ] 创建工具目录 `{tool_name}/`
- [ ] 编写 `TOOL.md` 描述文件
- [ ] 实现 `Tool` trait
- [ ] 确定权限级别
- [ ] 是否需要沙箱？
- [ ] 编写测试用例
- [ ] 注册到 ToolRegistry
- [ ] 更新文档
