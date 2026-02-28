# Sage Tink UI 重构计划

## 版本目标

- 当前版本: 0.1.0
- 重构开始版本: 0.1.1
- 目标版本: 0.2.0 (完成 Tink 集成)

## 依赖

- **rnk**: 0.2.0 (crates.io)
- 移除: `colored`, `indicatif`, `dialoguer`, `console`

---

## Phase 1: 基础架构 [P0]

### 1.1 添加依赖
- [ ] 添加 `rnk = "0.2"` 到 workspace
- [ ] 更新版本号到 0.1.1

### 1.2 创建 UI Bridge 模块
- [ ] `sage-core/src/ui/bridge/mod.rs` - 模块入口
- [ ] `sage-core/src/ui/bridge/state.rs` - AppState 状态定义
- [ ] `sage-core/src/ui/bridge/events.rs` - AgentEvent 事件定义
- [ ] `sage-core/src/ui/bridge/adapter.rs` - 事件到状态转换

### 1.3 创建主题系统
- [ ] `sage-core/src/ui/theme/mod.rs` - 主题模块入口
- [ ] `sage-core/src/ui/theme/colors.rs` - 颜色定义
- [ ] `sage-core/src/ui/theme/icons.rs` - 图标系统 (迁移)
- [ ] `sage-core/src/ui/theme/styles.rs` - 样式常量

---

## Phase 2: 核心组件 [P0]

### 2.1 基础组件
- [ ] `sage-core/src/ui/components/mod.rs` - 组件模块入口
- [ ] `sage-core/src/ui/components/spinner.rs` - Spinner 动画
- [ ] `sage-core/src/ui/components/message.rs` - 消息显示
- [ ] `sage-core/src/ui/components/thinking.rs` - 思考指示器

### 2.2 工具组件
- [ ] `sage-core/src/ui/components/tool_call.rs` - 工具调用显示

### 2.3 交互组件
- [ ] `sage-core/src/ui/components/status_bar.rs` - 状态栏
- [ ] `sage-core/src/ui/components/input_box.rs` - 输入框

---

## Phase 3: 主应用 [P0]

### 3.1 CLI 应用
- [ ] `sage-cli/src/app.rs` - Tink 主应用组件
- [ ] 修改 `sage-cli/src/main.rs` - 添加 --new-ui 参数

### 3.2 集成测试
- [ ] 验证基本渲染
- [ ] 验证用户输入
- [ ] 验证流式输出

---

## Phase 4: Agent 集成 [P1]

### 4.1 EventManager 重构
- [ ] 修改 `event_manager/mod.rs` - 发送 AgentEvent
- [ ] 实现 ExecutionEvent → AgentEvent 转换

### 4.2 LLM 流式输出
- [ ] 修改 `llm_orchestrator.rs` - 适配流式输出

### 4.3 工具显示
- [ ] 修改 `tool_display.rs` - 通过事件更新

---

## Phase 5: 删除旧代码 [P2]

### 5.1 删除 sage-core/src/ui/ 旧文件
- [ ] 删除 `animation.rs`
- [ ] 删除 `claude_style.rs`
- [ ] 删除 `display.rs`
- [ ] 删除 `enhanced_console.rs`
- [ ] 删除 `progress.rs`
- [ ] 删除 `prompt.rs` (保留权限逻辑)

### 5.2 删除 sage-cli/src/ui/
- [ ] 删除 `nerd_console.rs`
- [ ] 删除 `components.rs`
- [ ] 删除 `icons.rs`

### 5.3 删除 sage-cli/src/ 旧文件
- [ ] 删除 `console.rs`
- [ ] 删除 `progress.rs`

### 5.4 清理依赖
- [ ] 移除 `colored`
- [ ] 移除 `indicatif`
- [ ] 移除 `dialoguer`
- [ ] 移除 `console`

---

## Phase 6: 优化和测试 [P2]

### 6.1 性能优化
- [ ] 使用 Static 组件缓存
- [ ] 细粒度状态更新

### 6.2 测试
- [ ] 单元测试
- [ ] 集成测试
- [ ] 手动测试

### 6.3 文档
- [ ] 更新 README
- [ ] 更新 CHANGELOG
- [ ] 更新版本到 0.2.0

---

## 文件清单

### 新建文件 (15 个)

```
crates/sage-core/src/ui/
├── bridge/
│   ├── mod.rs
│   ├── state.rs
│   ├── events.rs
│   └── adapter.rs
├── components/
│   ├── mod.rs
│   ├── spinner.rs
│   ├── message.rs
│   ├── thinking.rs
│   ├── tool_call.rs
│   ├── status_bar.rs
│   └── input_box.rs
├── theme/
│   ├── mod.rs
│   ├── colors.rs
│   ├── icons.rs
│   └── styles.rs
└── traits.rs

crates/sage-cli/src/
└── app.rs
```

### 删除文件 (17 个)

```
crates/sage-core/src/ui/
├── animation.rs
├── claude_style.rs
├── display.rs
├── enhanced_console.rs
├── progress.rs
└── prompt.rs (部分)

crates/sage-cli/src/
├── ui/
│   ├── nerd_console.rs
│   ├── components.rs
│   └── icons.rs
├── console.rs
└── progress.rs
```

### 修改文件 (8 个)

```
Cargo.toml (workspace)
crates/sage-core/Cargo.toml
crates/sage-cli/Cargo.toml
crates/sage-core/src/ui/mod.rs
crates/sage-core/src/ui/markdown.rs (适配)
crates/sage-core/src/agent/unified/event_manager/mod.rs
crates/sage-core/src/agent/unified/llm_orchestrator.rs
crates/sage-cli/src/main.rs
```

---

## 进度跟踪

| Phase | 状态 | 版本 |
|-------|------|------|
| Phase 1: 基础架构 | 🔄 进行中 | 0.1.1 |
| Phase 2: 核心组件 | ⏳ 待开始 | 0.1.2 |
| Phase 3: 主应用 | ⏳ 待开始 | 0.1.3 |
| Phase 4: Agent 集成 | ⏳ 待开始 | 0.1.4 |
| Phase 5: 删除旧代码 | ⏳ 待开始 | 0.1.5 |
| Phase 6: 优化测试 | ⏳ 待开始 | 0.2.0 |

---

## 参考文档

- [rnk API 文档](https://docs.rs/rnk)
- [Sage UI Design Skill](.sage/skills/sage-ui-design/SKILL.md)
- [Tink UI Migration Skill](.sage/skills/tink-ui-migration/SKILL.md)
- [Version Management Skill](.sage/skills/version-management/SKILL.md)
