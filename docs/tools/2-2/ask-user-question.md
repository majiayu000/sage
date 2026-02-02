# AskUserQuestion Tool

## 基本信息

| 属性 | 值 |
|------|-----|
| 工具名称 | AskUserQuestion |
| Claude Code 版本 | 2.0.77 |
| 类别 | 交互工具 |
| Sage 实现 | `sage-tools/src/tools/interaction/ask_user/` |

## 功能描述

在执行过程中向用户提问，收集偏好、澄清指令或获取决策。

## 完整 Prompt

```markdown
Use this tool when you need to ask the user questions during execution. This allows you to:
1. Gather user preferences or requirements
2. Clarify ambiguous instructions
3. Get decisions on implementation choices as you work
4. Offer choices to the user about what direction to take.

Usage notes:
- Users will always be able to select "Other" to provide custom text input
- Use multiSelect: true to allow multiple answers to be selected for a question
- If you recommend a specific option, make that the first option in the list and add "(Recommended)" at the end of the label

Plan mode note: In plan mode, use this tool to clarify requirements or choose between approaches BEFORE finalizing your plan. Do NOT use this tool to ask "Is my plan ready?" or "Should I proceed?" - use ExitPlanMode for plan approval.
```

## 参数

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| questions | array | ✅ | 问题列表 (1-4 个) |
| questions[].question | string | ✅ | 问题内容 |
| questions[].header | string | ✅ | 简短标签 (最多 12 字符) |
| questions[].options | array | ✅ | 选项列表 (2-4 个) |
| questions[].options[].label | string | ✅ | 选项显示文本 |
| questions[].options[].description | string | ❌ | 选项说明 |
| questions[].multiSelect | boolean | ❌ | 是否允许多选 |

## 设计原理

### 1. 结构化问题
**为什么**:
- 比自由文本更容易回答
- 减少歧义
- 提高响应速度

### 2. 自动添加 "Other" 选项
**为什么**:
- 用户可能有预设选项之外的想法
- 保持灵活性
- 不限制用户选择

### 3. 推荐选项标记
**为什么**:
- 帮助用户快速决策
- 展示 Agent 的专业判断
- 减少用户认知负担

### 4. 多选支持
**为什么**:
- 某些问题需要多个答案
- 如"启用哪些功能？"
- 提高交互效率

### 5. 规划模式限制
**为什么**:
- 避免用 AskUserQuestion 确认计划
- ExitPlanMode 专门用于计划批准
- 保持工具职责清晰

## 使用场景

### ✅ 应该使用
- 澄清模糊的需求
- 选择实现方案
- 确认用户偏好
- 获取配置选项

### ❌ 不应该使用
- 确认计划是否可以 (使用 ExitPlanMode)
- 简单的是/否确认
- 可以自主决定的事项

## 示例

### 单选问题
```json
{
  "questions": [{
    "question": "Which authentication method should we use?",
    "header": "Auth method",
    "options": [
      {"label": "JWT (Recommended)", "description": "Stateless, scalable"},
      {"label": "Session", "description": "Traditional, server-side"},
      {"label": "OAuth 2.0", "description": "Third-party integration"}
    ],
    "multiSelect": false
  }]
}
```

### 多选问题
```json
{
  "questions": [{
    "question": "Which features do you want to enable?",
    "header": "Features",
    "options": [
      {"label": "Dark mode", "description": "Theme switching"},
      {"label": "Notifications", "description": "Push notifications"},
      {"label": "Analytics", "description": "Usage tracking"}
    ],
    "multiSelect": true
  }]
}
```

## Sage 实现差异

Sage 的 AskUserQuestionTool 支持：
- 更多问题类型 (文本输入、数字等)
- 问题验证
- 历史答案记录
- 默认值设置
