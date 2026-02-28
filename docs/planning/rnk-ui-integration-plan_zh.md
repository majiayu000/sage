# Sage CLI rnk UI Integration Plan

## Problem Statement

当前 `sage-cli` 的 UI 是坏的：
1. `OutputMode::Rnk` 发送事件到 `emit_event()`，但没有订阅者
2. `global_adapter()` 返回 `None`，所有事件被静默丢弃
3. 结果是用户看不到任何输出

## Solution Overview

实现 Inline Mode 流式 UI，类似 Claude Code 和 glm_chat 示例。

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Executor      │────▶│   EventAdapter   │────▶│   UIRenderer    │
│  (sage-core)    │     │   (sage-core)    │     │   (sage-cli)    │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                       │                        │
        │ emit_event()          │ watch::channel         │ ANSI output
        ▼                       ▼                        ▼
   AgentEvent              AppState change         Terminal display
```

## Implementation Steps

### Phase 1: Core Infrastructure (sage-core)

**File: `crates/sage-core/src/ui/bridge/adapter.rs`**

1. 添加 `tokio::sync::watch` channel 到 `EventAdapter`
2. 在 `handle_event()` 后通过 channel 广播状态变化
3. 添加 `subscribe()` 方法供 CLI 订阅

```rust
pub struct EventAdapter {
    state: Arc<RwLock<AppState>>,
    state_tx: watch::Sender<AppState>,  // NEW
}

impl EventAdapter {
    pub fn subscribe(&self) -> watch::Receiver<AppState> {
        self.state_tx.subscribe()
    }
}
```

### Phase 2: Streaming Components (sage-cli)

**File: `crates/sage-cli/src/ui/streaming.rs` (NEW)**

1. `ThinkingIndicator`: 后台线程运行的 spinner
   - 显示: `⠋ Thinking... (2.3s)`
   - 支持 Esc 取消

2. `StreamingMessage`: 流式文本输出
   - 接收 chunk，立即打印（无 glimmer）
   - 追踪已打印内容

```rust
pub struct ThinkingIndicator {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

pub struct StreamingMessage {
    buffer: String,
}
```

### Phase 3: App Rewrite (sage-cli)

**File: `crates/sage-cli/src/app.rs`**

1. 初始化时设置 `global_adapter`
2. 创建状态监听 task
3. 根据 `AppState.phase` 切换 UI 组件

```rust
pub async fn run_app() -> Result<()> {
    // 1. Setup adapter
    let adapter = EventAdapter::with_default_state();
    let mut state_rx = adapter.subscribe();
    set_global_adapter(adapter);

    // 2. Start executor
    let executor = create_executor();
    executor.set_output_mode(OutputMode::Rnk);

    // 3. Main loop
    loop {
        tokio::select! {
            Ok(_) = state_rx.changed() => {
                let state = state_rx.borrow();
                handle_state_change(&state);
            }
            input = read_user_input() => {
                handle_input(input);
            }
        }
    }
}
```

### Phase 4: State Machine

```
          ┌─────────┐
          │  Idle   │◀──────────────────┐
          └────┬────┘                   │
               │ user input             │ stream end
               ▼                        │
          ┌─────────┐                   │
          │Thinking │──────────────────▶│
          └────┬────┘                   │
               │ first chunk            │
               ▼                        │
          ┌─────────┐                   │
          │Streaming│───────────────────┘
          └────┬────┘
               │ tool call
               ▼
          ┌─────────┐
          │ToolExec │───────────────────▶ Idle
          └─────────┘
```

## Files to Modify

| File | Action | Description |
|------|--------|-------------|
| `crates/sage-core/src/ui/bridge/adapter.rs` | Modify | Add watch channel |
| `crates/sage-core/src/ui/bridge/mod.rs` | Modify | Re-export subscribe |
| `crates/sage-cli/src/ui/mod.rs` | Create | New UI module |
| `crates/sage-cli/src/ui/streaming.rs` | Create | Streaming components |
| `crates/sage-cli/src/ui/indicators.rs` | Create | ThinkingIndicator |
| `crates/sage-cli/src/app.rs` | Rewrite | New app loop |

## Output Example

```
╭─ Sage Agent v0.2.6 ────────────────────────────────────╮
│  Type your request, or 'exit' to quit                 │
╰────────────────────────────────────────────────────────╯

> 帮我写一个 hello world

⠹ Thinking... (1.2s)

● 好的，我来帮你写一个 Hello World 程序。

  ```rust
  fn main() {
      println!("Hello, World!");
  }
  ```

  这个程序会在控制台输出 "Hello, World!"。

> _
```

## Risk Mitigation

1. **Lock contention**: 使用 `watch` channel 避免频繁加锁
2. **Terminal state**: 使用 RAII guard 确保 raw mode 正确恢复
3. **Thread safety**: `ThinkingIndicator` 使用 `AtomicBool` 控制停止

## Testing Plan

1. Unit test: `StreamingMessage` buffer 管理
2. Integration test: 模拟事件流，验证 UI 更新
3. Manual test: 实际对话，验证流式效果

## Version Bump

此次更改为功能增强，需要 MINOR 版本更新：
- 当前版本: `0.2.5`
- 新版本: `0.3.0`
