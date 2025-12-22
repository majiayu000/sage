# Sage 代码库搜索策略改进建议

## 当前问题分析

### 1. Task 工具未实现真正的子代理执行

**问题文件**: `crates/sage-tools/src/tools/process/task.rs`

当前 Task 工具只是注册任务，没有实际执行：
```rust
// 第 254-273 行
// For now, return a placeholder response
// In a full implementation, this would trigger the SubAgentExecutor
```

**修复方案**:
1. 实现同步执行：调用 `SubAgentExecutor::execute()`
2. 或者集成到主 Agent 循环中处理 pending tasks

### 2. 主代理没有自动使用 Explore 子代理

**问题**: 当任务需要代码库探索时，主代理直接使用 Grep/Read，而不是委托给 Explore 子代理。

**修复方案**:
在系统提示中添加更强的引导：
```
当需要探索代码库理解问题时，必须先使用 Task 工具的 Explore 子代理，
而不是直接使用 Grep/Read。直接搜索只适用于已知明确路径的情况。
```

### 3. 搜索策略缺乏智能性

**当前行为**:
- 使用模糊的 Grep 模式（如搜索 "Identity"）
- 没有利用任务描述中的提示（明确说了 `sympy/printing/pycode.py`）
- 串行执行搜索而不是并行

**改进方案**:

#### A. 直接定位策略
如果任务描述中包含文件路径，应该：
```
1. 优先读取明确提到的文件
2. 然后再搜索相关文件
```

#### B. 并行搜索策略
同时执行多个相关搜索：
```
并行执行:
- Grep("_print_Identity")
- Grep("class NumPyPrinter")
- Grep("def _print_.*Matrix")
```

#### C. 分层搜索策略
```
第1层: 精确搜索（类名、函数名）
第2层: 模式搜索（正则匹配）
第3层: 语义搜索（关键词）
```

## Claude Code 参考策略

### 探索代理提示的关键指导

```
NOTE: You are meant to be a fast agent that returns output as
quickly as possible. In order to achieve this you must:
- Make efficient use of the tools: be smart about how you search
- Wherever possible spawn multiple parallel tool calls
```

### 主代理系统提示的关键规则

```
VERY IMPORTANT: When exploring the codebase to gather context,
use the Task tool with subagent_type=Explore instead of running
search commands directly.
```

## 具体代码改进

### 1. 修复 Task 工具执行

```rust
// crates/sage-tools/src/tools/process/task.rs

async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
    // ... 解析参数 ...

    if !run_in_background {
        // 同步执行子代理
        let executor = SubAgentExecutor::new(
            self.registry.clone(),
            self.llm_client.clone(),
            self.tools.clone()
        );

        let config = SubAgentConfig::new(agent_type, prompt);
        let result = executor.execute(config, cancel_token).await?;

        return Ok(ToolResult {
            success: result.success,
            output: Some(result.output),
            ...
        });
    }

    // 后台执行...
}
```

### 2. 增强系统提示

```rust
// 在系统提示中添加
const EXPLORATION_GUIDANCE: &str = r#"
## 代码库探索策略

当需要理解代码库时，遵循以下优先级：

1. **直接定位**: 如果任务描述中提到了具体文件路径，先读取该文件
2. **使用 Explore 子代理**: 对于开放式探索，使用 Task(subagent_type=Explore)
3. **精确搜索**: 使用类名/函数名进行精确 Grep
4. **并行搜索**: 同时启动多个相关搜索

不要:
- 使用模糊关键词进行盲目搜索
- 串行执行可以并行的搜索
- 忽略任务描述中已给出的文件路径提示
"#;
```

### 3. 添加搜索效率检测

在 Agent 执行循环中添加检测：
```rust
// 如果检测到过多的连续 Grep 调用但没有 Edit
if consecutive_grep_calls > 5 && edit_calls == 0 {
    // 提示 Agent 考虑是否应该开始修改
    add_system_reminder("You have been searching for a while.
        If you have found the target file, start making changes.");
}
```

## 预期效果

| 指标 | 当前 | 改进后 |
|-----|------|--------|
| 探索步数 | 10-15 | 3-5 |
| 搜索效率 | 低 (盲目搜索) | 高 (精确定位) |
| Token 消耗 | ~450K | ~100K |
| 复杂问题成功率 | 33% | 预计 70%+ |

## 优先级

1. **高**: 实现 Task 工具的真正子代理执行
2. **高**: 增强系统提示中的搜索策略指导
3. **中**: 添加文件路径提取逻辑（从任务描述中提取明确路径）
4. **中**: 实现并行搜索调用
5. **低**: 添加搜索效率监控和提示
