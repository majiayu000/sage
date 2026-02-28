# UI Interaction Improvement Plan

Based on analysis of open-claude-code's UI design.

## Current Issues

1. AI 可以不经用户确认直接执行危险操作（如 rm）
2. UI 反馈不够直观，缺乏边框样式
3. 没有权限确认对话框
4. 工具执行进度显示不够清晰

## Reference: Open-Claude-Code UI Design

### 边框样式
```
╭────────────────────────────────────────────────╮
│             Step 15 - AI Thinking              │
╰────────────────────────────────────────────────╯
```

使用 Ink 框架的 FlexBox 组件：
```javascript
FlexBox({
    borderStyle: "round",      // 圆角边框
    borderColor: "permission", // 边框颜色
    paddingLeft: 1,
    paddingRight: 1
})
```

### 权限确认对话框
```
╭────────────────────────────────────────────────╮
│  ⚠️  Permission Required                       │
│                                                │
│  Tool: bash                                    │
│  Command: rm -rf ./build                       │
│                                                │
│  This will delete files recursively.           │
│                                                │
│  [1] Yes, execute once                         │
│  [2] Yes, always allow this pattern            │
│  [3] No, reject                                │
│  [4] No, always deny this pattern              │
│                                                │
│  ↑/↓ to select · Enter to confirm · Esc cancel │
╰────────────────────────────────────────────────╯
```

### 颜色系统
- `permission` - 蓝色，权限相关
- `error` - 红色，错误/危险
- `success` - 绿色，成功
- `warning` - 黄色，警告
- `dimColor` - 灰色，次要信息

## Implementation Tasks

### Phase 1: Permission Dialog UI (Priority: Critical)

- [ ] 1.1 Create permission dialog component
  - File: `crates/sage-core/src/ui/permission_dialog.rs`
  - Display confirmation dialog with border
  - Support Yes/No selection
  - Show command details and warning

- [ ] 1.2 Integrate with tool executor
  - Modify: `crates/sage-core/src/tools/executor.rs`
  - Check for dangerous commands before execution
  - Show dialog and wait for user input
  - Only proceed if user confirms

- [ ] 1.3 Update bash tool
  - Already done: `user_confirmed` parameter
  - Already done: `ConfirmationRequired` error
  - TODO: Connect to permission dialog

### Phase 2: Improve Step Display UI

- [ ] 2.1 Add border to step separator
  - Modify: `crates/sage-core/src/ui/display.rs`
  - Use box-drawing characters for borders
  - Add colors for different states

- [ ] 2.2 Improve tool execution feedback
  - Show tool name and arguments clearly
  - Display progress with animation
  - Show success/error status with colors

### Phase 3: Input System Enhancement

- [ ] 3.1 Add keyboard shortcuts
  - `Ctrl+C` - Interrupt execution
  - `Esc` - Cancel current operation
  - `y/n` - Quick confirmation

- [ ] 3.2 Add confirmation prompt helpers
  - Simple yes/no prompt function
  - Multi-choice selection
  - Text input with validation

## Files to Create/Modify

### New Files
```
crates/sage-core/src/ui/permission_dialog.rs  # Permission confirmation UI
crates/sage-core/src/ui/prompt.rs             # User prompt utilities
crates/sage-core/src/ui/styles.rs             # Color and style definitions
```

### Modified Files
```
crates/sage-core/src/ui/display.rs            # Add border styles
crates/sage-core/src/ui/mod.rs                # Export new modules
crates/sage-core/src/tools/executor.rs        # Integrate permission check
crates/sage-core/src/agent/unified/step_execution.rs  # Handle permission UI
```

## Testing Strategy

### Unit Tests
- Permission dialog rendering
- Color output correctness
- Border character generation

### Integration Tests
- Permission flow with mock input
- Tool execution with confirmation
- **IMPORTANT**: No actual file deletion (rm) in tests

### Manual Testing
- Run sage with dangerous commands
- Verify dialog appears before execution
- Test all confirmation options

## Implementation Order

1. **Task 1**: Create basic permission dialog UI
2. **Task 2**: Integrate with tool executor
3. **Task 3**: Test permission flow
4. **Task 4**: Improve step display with borders
5. **Task 5**: Run full test suite
6. **Task 6**: Commit to GitHub

## Success Criteria

- [ ] Dangerous commands ALWAYS show confirmation dialog
- [ ] User can select Yes/No before execution
- [ ] No command executes without explicit user consent
- [ ] UI has clear visual hierarchy with borders and colors
- [ ] All tests pass without any rm operations
