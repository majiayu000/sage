# Concurrency Model

## Overview

Sage Agent is built on Tokio async runtime with structured concurrency patterns for safe, efficient parallel execution.

## Async Runtime

```rust
// All async operations use Tokio
#[tokio::main]
async fn main() {
    let agent = SageBuilder::new()
        .with_anthropic("key")
        .build()
        .await?;
}
```

## Cancellation Hierarchy

### Structure

```
                    ┌─────────────────┐
                    │   Root Token    │
                    │  (Application)  │
                    └────────┬────────┘
                             │
            ┌────────────────┼────────────────┐
            │                │                │
    ┌───────▼───────┐ ┌──────▼──────┐ ┌──────▼──────┐
    │Session Token 1│ │Session Token│ │Session Token│
    │   (User A)    │ │  (User B)   │ │  (User C)   │
    └───────┬───────┘ └─────────────┘ └─────────────┘
            │
    ┌───────┼───────┐
    │               │
┌───▼───┐     ┌─────▼─────┐
│Agent 1│     │  Agent 2  │
└───┬───┘     └───────────┘
    │
┌───┼───┬───────┐
│       │       │
▼       ▼       ▼
Tool1  Tool2  Tool3
```

### Implementation

```rust
pub struct CancellationHierarchy {
    root: CancellationToken,
    sessions: DashMap<SessionId, CancellationToken>,
    agents: DashMap<AgentId, CancellationToken>,
    tools: DashMap<ToolCallId, CancellationToken>,
}

impl CancellationHierarchy {
    // Create child token - automatically cancelled when parent is cancelled
    pub fn create_session_token(&self, id: SessionId) -> CancellationToken {
        let token = self.root.child_token();
        self.sessions.insert(id, token.clone());
        token
    }

    pub fn create_agent_token(&self, session_id: SessionId, agent_id: AgentId) -> Option<CancellationToken> {
        self.sessions.get(&session_id).map(|session_token| {
            let token = session_token.child_token();
            self.agents.insert(agent_id, token.clone());
            token
        })
    }

    // Cancel propagates down the hierarchy
    pub fn cancel_session(&self, id: SessionId) {
        if let Some((_, token)) = self.sessions.remove(&id) {
            token.cancel();
        }
    }
}
```

### Usage Pattern

```rust
// In agent execution
async fn execute_step(&mut self) -> SageResult<AgentStep> {
    let cancellation_token = self.cancellation.create_tool_token(agent_id, tool_id)?;

    tokio::select! {
        result = self.llm_client.chat(&messages, tools) => {
            result?
        }
        _ = cancellation_token.cancelled() => {
            return Err(SageError::Cancelled);
        }
    }
}
```

## Parallel Tool Execution

### Semaphore-Based Concurrency Control

```rust
pub struct ParallelToolExecutor {
    global_semaphore: Arc<Semaphore>,      // Global concurrency limit
    type_semaphores: DashMap<String, Arc<Semaphore>>,  // Per-type limits
    sequential_lock: Arc<Mutex<()>>,        // For sequential mode
}

pub enum ConcurrencyMode {
    Parallel,           // No limit (uses global semaphore)
    Sequential,         // One at a time
    Limited(usize),     // Fixed concurrency
    ExclusiveByType,    // One per tool type
}
```

### Execution Flow

```
┌──────────────────────────────────────────────────────────────┐
│                  Parallel Tool Execution                      │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  Tool Calls: [A, B, C, D]                                    │
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Check Concurrency Mode                      │ │
│  └────────────────────────┬────────────────────────────────┘ │
│                           │                                   │
│      ┌────────────────────┼────────────────────┐             │
│      │                    │                    │              │
│      ▼                    ▼                    ▼              │
│  Parallel            Sequential           Limited(2)          │
│      │                    │                    │              │
│      ▼                    ▼                    ▼              │
│  ┌──────┐            ┌──────┐            ┌──────┐            │
│  │Spawn │            │Lock  │            │Acquire│            │
│  │All   │            │      │            │Permit │            │
│  └──┬───┘            └──┬───┘            └──┬───┘            │
│     │                   │                   │                 │
│  ┌──▼──┐  ┌──▼──┐   ┌──▼──┐             ┌──▼──┐  ┌──▼──┐    │
│  │ A   │  │ B   │   │ A   │             │ A   │  │ B   │    │
│  │     │  │     │   │     │             │     │  │     │    │
│  │ C   │  │ D   │   │ B   │             │ C   │  │ D   │    │
│  └─────┘  └─────┘   │     │             └─────┘  └─────┘    │
│     │        │      │ C   │                │        │        │
│     │        │      │     │                │        │        │
│     └────┬───┘      │ D   │                └────┬───┘        │
│          │          └─────┘                     │             │
│          ▼              │                       ▼             │
│     join_all()     sequential              join_all()         │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Implementation

```rust
pub async fn execute_parallel(&self, calls: &[ToolCall]) -> Vec<ToolResult> {
    let futures: Vec<_> = calls.iter().map(|call| {
        let executor = self.clone();
        let call = call.clone();

        async move {
            // Acquire semaphore permit
            let _permit = executor.global_semaphore.acquire().await.unwrap();

            // Execute tool
            executor.execute_single(&call).await
        }
    }).collect();

    futures::future::join_all(futures).await
}
```

## Event Bus

### Pub/Sub Pattern

```rust
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
```

### Usage

```rust
// Publisher (in tool executor)
self.event_bus.publish(Event::ToolCallStart {
    id: call.id.clone(),
    name: call.name.clone(),
});

// Subscriber (in UI or logging)
let mut rx = event_bus.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        match event {
            Event::ToolCallStart { id, name } => {
                println!("Tool {} started: {}", name, id);
            }
            Event::TextDelta(text) => {
                print!("{}", text);
            }
            _ => {}
        }
    }
});
```

## Thread Safety

### DashMap for Concurrent Access

```rust
// Thread-safe concurrent hashmap
tools: DashMap<String, Arc<dyn Tool>>,
type_semaphores: DashMap<String, Arc<Semaphore>>,

// Safe concurrent reads and writes
self.tools.insert(name, tool);
if let Some(tool) = self.tools.get(&name) {
    tool.execute(&call).await
}
```

### RwLock for State

```rust
pub struct McpClient {
    server_info: RwLock<Option<McpServerInfo>>,
    capabilities: RwLock<McpCapabilities>,
    initialized: RwLock<bool>,
}

// Multiple readers, single writer
let info = self.server_info.read().await.clone();
*self.initialized.write().await = true;
```

### Mutex for Exclusive Access

```rust
pub struct StdioTransport {
    transport: Mutex<Box<dyn McpTransport>>,
}

// Exclusive access for send/receive
let mut transport = self.transport.lock().await;
transport.send(message).await?;
```

## Best Practices

1. **Use structured concurrency** - Always use cancellation tokens for cleanup
2. **Prefer DashMap** over Mutex<HashMap> for concurrent access
3. **Use semaphores** for rate limiting, not mutexes
4. **Broadcast channels** for fan-out event distribution
5. **tokio::select!** for cancellation-aware async operations
