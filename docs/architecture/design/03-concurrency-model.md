# Concurrency Model Design

> The most critical design document for concurrent asynchronous Code Agent

## 1. Executive Summary

This document defines the concurrency architecture for Sage Agent. Getting this right is **critical** - concurrency model mistakes cannot be easily fixed later.

### Core Principles

```
+------------------------------------------------------------------+
|                    Concurrency Principles                         |
+------------------------------------------------------------------+
|                                                                   |
|  1. MESSAGE PASSING OVER SHARED STATE                            |
|     - Agents communicate via channels, not shared memory         |
|     - Reduces race conditions and deadlocks                      |
|                                                                   |
|  2. STRUCTURED CONCURRENCY                                       |
|     - Parent tasks own child tasks                               |
|     - Cancellation propagates through hierarchy                  |
|                                                                   |
|  3. BACKPRESSURE BY DEFAULT                                      |
|     - Bounded channels prevent memory exhaustion                 |
|     - Producers slow down when consumers are slow                |
|                                                                   |
|  4. GRACEFUL DEGRADATION                                         |
|     - Single component failure doesn't crash system              |
|     - Timeouts and retries are first-class citizens              |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 2. Runtime Architecture

### 2.1 Tokio Runtime Configuration

```rust
/// Runtime configuration for Sage
pub struct RuntimeConfig {
    /// Number of worker threads (default: num_cpus)
    pub worker_threads: usize,

    /// Thread stack size (default: 2MB)
    pub thread_stack_size: usize,

    /// Enable I/O driver
    pub enable_io: bool,

    /// Enable time driver
    pub enable_time: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            thread_stack_size: 2 * 1024 * 1024,
            enable_io: true,
            enable_time: true,
        }
    }
}

/// Build the runtime
pub fn build_runtime(config: RuntimeConfig) -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.worker_threads)
        .thread_stack_size(config.thread_stack_size)
        .enable_io()
        .enable_time()
        .build()
        .expect("Failed to build Tokio runtime")
}
```

### 2.2 Task Hierarchy

```
+------------------------------------------------------------------+
|                       Task Hierarchy                              |
+------------------------------------------------------------------+
|                                                                   |
|  Runtime (tokio)                                                  |
|  │                                                                |
|  └── Session Task (long-lived)                                   |
|      │                                                            |
|      ├── MessageStream Task                                      |
|      │   │                                                        |
|      │   ├── SSE Parser Task                                     |
|      │   └── Event Dispatcher Task                               |
|      │                                                            |
|      ├── Agent Task (per agent)                                  |
|      │   │                                                        |
|      │   ├── LLM Request Task                                    |
|      │   └── Tool Execution Task(s)                              |
|      │       │                                                    |
|      │       └── Individual Tool Task                            |
|      │                                                            |
|      ├── Event Bus Task                                          |
|      │                                                            |
|      └── Background Tasks                                        |
|          │                                                        |
|          ├── Trajectory Writer                                   |
|          ├── Telemetry Reporter                                  |
|          └── Cache Manager                                       |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 3. Cancellation Architecture

### 3.1 Cancellation Token Hierarchy

```rust
use tokio_util::sync::CancellationToken;

/// Hierarchical cancellation management
pub struct CancellationHierarchy {
    /// Root token - cancels everything
    root: CancellationToken,

    /// Session level tokens
    sessions: DashMap<SessionId, CancellationToken>,

    /// Agent level tokens
    agents: DashMap<AgentId, CancellationToken>,

    /// Tool call level tokens
    tool_calls: DashMap<ToolCallId, CancellationToken>,
}

impl CancellationHierarchy {
    pub fn new() -> Self {
        Self {
            root: CancellationToken::new(),
            sessions: DashMap::new(),
            agents: DashMap::new(),
            tool_calls: DashMap::new(),
        }
    }

    /// Create a session token (child of root)
    pub fn create_session_token(&self, session_id: SessionId) -> CancellationToken {
        let token = self.root.child_token();
        self.sessions.insert(session_id, token.clone());
        token
    }

    /// Create an agent token (child of session)
    pub fn create_agent_token(
        &self,
        session_id: SessionId,
        agent_id: AgentId,
    ) -> Option<CancellationToken> {
        self.sessions.get(&session_id).map(|session_token| {
            let token = session_token.child_token();
            self.agents.insert(agent_id, token.clone());
            token
        })
    }

    /// Create a tool call token (child of agent)
    pub fn create_tool_call_token(
        &self,
        agent_id: AgentId,
        call_id: ToolCallId,
    ) -> Option<CancellationToken> {
        self.agents.get(&agent_id).map(|agent_token| {
            let token = agent_token.child_token();
            self.tool_calls.insert(call_id, token.clone());
            token
        })
    }

    /// Cancel a specific agent (and all its tool calls)
    pub fn cancel_agent(&self, agent_id: AgentId) {
        if let Some((_, token)) = self.agents.remove(&agent_id) {
            token.cancel();
        }
    }

    /// Cancel a specific tool call
    pub fn cancel_tool_call(&self, call_id: ToolCallId) {
        if let Some((_, token)) = self.tool_calls.remove(&call_id) {
            token.cancel();
        }
    }
}
```

### 3.2 Cancellation Propagation Diagram

```
+------------------------------------------------------------------+
|                   Cancellation Propagation                        |
+------------------------------------------------------------------+
|                                                                   |
|  User presses Ctrl+C                                              |
|         │                                                         |
|         v                                                         |
|  ┌─────────────┐                                                 |
|  │ Root Token  │ ─────────────────────────────────────────────┐  |
|  │  .cancel()  │                                              │  |
|  └──────┬──────┘                                              │  |
|         │                                                      │  |
|         │ propagates to children                               │  |
|         v                                                      v  |
|  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐       │  |
|  │  Session 1  │    │  Session 2  │    │  Session N  │       │  |
|  │   Token     │    │   Token     │    │   Token     │       │  |
|  └──────┬──────┘    └──────┬──────┘    └─────────────┘       │  |
|         │                  │                                   │  |
|         v                  v                                   │  |
|  ┌─────────────┐    ┌─────────────┐                          │  |
|  │   Agent 1   │    │   Agent 2   │                          │  |
|  │   Token     │    │   Token     │                          │  |
|  └──────┬──────┘    └─────────────┘                          │  |
|         │                                                      │  |
|    ┌────┴────┐                                                │  |
|    v         v                                                │  |
|  ┌─────┐  ┌─────┐                                            │  |
|  │Tool1│  │Tool2│                                            │  |
|  │Token│  │Token│ ◄──────────────────────────────────────────┘  |
|  └─────┘  └─────┘                                               |
|                                                                   |
|  All tasks check: token.is_cancelled()                           |
|  All selects use: token.cancelled()                              |
|                                                                   |
+------------------------------------------------------------------+
```

### 3.3 Graceful Shutdown Pattern

```rust
/// Pattern for graceful shutdown in async tasks
async fn cancellable_task(
    cancel_token: CancellationToken,
    // other parameters...
) -> Result<TaskOutput> {
    loop {
        tokio::select! {
            // Check for cancellation first (biased)
            biased;

            _ = cancel_token.cancelled() => {
                // Perform cleanup
                cleanup_resources().await;
                return Err(SageError::Cancelled);
            }

            // Normal operation
            result = do_work() => {
                match result {
                    Ok(output) => {
                        if is_complete(&output) {
                            return Ok(output);
                        }
                        // Continue loop
                    }
                    Err(e) => {
                        // Handle error, possibly retry
                        handle_error(e).await?;
                    }
                }
            }
        }
    }
}
```

---

## 4. Channel Architecture

### 4.1 Channel Types and Purposes

```
+------------------------------------------------------------------+
|                      Channel Architecture                         |
+------------------------------------------------------------------+
|                                                                   |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                      Event Bus                               │ |
|  │                  (broadcast channel)                         │ |
|  │                                                              │ |
|  │   Capacity: 1024 events                                     │ |
|  │   Lagging policy: Drop oldest                               │ |
|  │                                                              │ |
|  │   Publishers:                                                │ |
|  │   - MessageStreamHandler                                    │ |
|  │   - AgentExecutor                                           │ |
|  │   - ToolExecutor                                            │ |
|  │                                                              │ |
|  │   Subscribers:                                               │ |
|  │   - UI Renderer                                             │ |
|  │   - Trajectory Recorder                                     │ |
|  │   - Telemetry Reporter                                      │ |
|  │   - User callbacks                                          │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                   Agent Mailbox                              │ |
|  │                   (mpsc channel)                             │ |
|  │                                                              │ |
|  │   Capacity: 32 messages                                     │ |
|  │   Backpressure: Sender blocks                               │ |
|  │                                                              │ |
|  │   Message types:                                             │ |
|  │   - UserMessage                                             │ |
|  │   - ToolResult                                              │ |
|  │   - PermissionResponse                                      │ |
|  │   - ControlMessage (pause, resume, cancel)                  │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                 Tool Result Channel                          │ |
|  │                  (oneshot channel)                           │ |
|  │                                                              │ |
|  │   Used for: Single tool call -> result                      │ |
|  │   Pattern: Spawn task, await result                         │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
+------------------------------------------------------------------+
```

### 4.2 Channel Implementations

```rust
use tokio::sync::{broadcast, mpsc, oneshot};

/// Event bus for system-wide event distribution
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: Event) {
        // Ignore error if no subscribers
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

/// Agent mailbox for receiving messages
pub struct AgentMailbox {
    sender: mpsc::Sender<AgentMessage>,
    receiver: mpsc::Receiver<AgentMessage>,
}

pub enum AgentMessage {
    UserInput(String),
    ToolResult(ToolCallId, ToolResult),
    PermissionResponse(RequestId, PermissionDecision),
    Control(ControlMessage),
}

pub enum ControlMessage {
    Pause,
    Resume,
    Cancel,
    UpdateConfig(AgentConfig),
}

impl AgentMailbox {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self { sender, receiver }
    }

    pub fn sender(&self) -> mpsc::Sender<AgentMessage> {
        self.sender.clone()
    }

    pub async fn recv(&mut self) -> Option<AgentMessage> {
        self.receiver.recv().await
    }
}

/// Tool result delivery
pub fn tool_result_channel() -> (
    oneshot::Sender<ToolResult>,
    oneshot::Receiver<ToolResult>,
) {
    oneshot::channel()
}
```

### 4.3 Channel Topology Diagram

```
+------------------------------------------------------------------+
|                     Channel Topology                              |
+------------------------------------------------------------------+
|                                                                   |
|                    ┌──────────────┐                              |
|                    │   Session    │                              |
|                    │ Orchestrator │                              |
|                    └──────┬───────┘                              |
|                           │                                       |
|            ┌──────────────┼──────────────┐                       |
|            │              │              │                        |
|            v              v              v                        |
|     ┌──────────┐   ┌──────────┐   ┌──────────┐                  |
|     │  Agent   │   │  Agent   │   │  Agent   │                  |
|     │ Mailbox  │   │ Mailbox  │   │ Mailbox  │                  |
|     │  (mpsc)  │   │  (mpsc)  │   │  (mpsc)  │                  |
|     └────┬─────┘   └────┬─────┘   └────┬─────┘                  |
|          │              │              │                          |
|          └──────────────┼──────────────┘                          |
|                         │                                         |
|                         v                                         |
|          ┌──────────────────────────────┐                        |
|          │         Event Bus            │                        |
|          │       (broadcast)            │                        |
|          └──────────────┬───────────────┘                        |
|                         │                                         |
|       ┌─────────────────┼─────────────────┐                      |
|       │                 │                 │                       |
|       v                 v                 v                       |
|  ┌─────────┐      ┌─────────┐      ┌─────────┐                  |
|  │   UI    │      │Trajectory│     │Telemetry│                   |
|  │Renderer │      │ Recorder │     │Reporter │                   |
|  └─────────┘      └─────────┘      └─────────┘                  |
|                                                                   |
|                                                                   |
|  Tool Execution:                                                  |
|                                                                   |
|  ┌─────────┐  oneshot   ┌─────────┐                             |
|  │  Agent  │───────────▶│  Tool   │                             |
|  │         │◀───────────│Executor │                             |
|  └─────────┘   result   └─────────┘                             |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 5. Tool Execution Concurrency

### 5.1 Semaphore-Based Limiting

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

/// Tool executor with concurrency control
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,

    /// Global concurrency limit
    global_semaphore: Arc<Semaphore>,

    /// Per-tool-type limits
    tool_semaphores: HashMap<String, Arc<Semaphore>>,

    /// Event bus for publishing events
    event_bus: EventBus,

    /// Cancellation hierarchy
    cancel_hierarchy: Arc<CancellationHierarchy>,
}

impl ToolExecutor {
    pub fn new(
        registry: Arc<ToolRegistry>,
        max_concurrent: usize,
    ) -> Self {
        Self {
            registry,
            global_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            tool_semaphores: HashMap::new(),
            event_bus: EventBus::new(1024),
            cancel_hierarchy: Arc::new(CancellationHierarchy::new()),
        }
    }

    /// Execute a single tool call
    pub async fn execute(
        &self,
        call: ToolCall,
        cancel_token: CancellationToken,
    ) -> ToolResult {
        // Create child token for this call
        let call_token = cancel_token.child_token();

        // Acquire global permit
        let _global_permit = tokio::select! {
            biased;
            _ = call_token.cancelled() => {
                return ToolResult::cancelled(call.id);
            }
            permit = self.global_semaphore.acquire() => {
                permit.expect("semaphore closed")
            }
        };

        // Get tool
        let tool = match self.registry.get(&call.tool_name) {
            Some(t) => t,
            None => return ToolResult::error(call.id, "Tool not found"),
        };

        // Check permissions
        let permission = tool.check_permission(&call, &call.context).await;
        if let PermissionResult::Deny { reason } = permission {
            return ToolResult::permission_denied(call.id, reason);
        }

        // Execute with timeout
        let timeout = tool.max_execution_time()
            .unwrap_or(Duration::from_secs(120));

        self.event_bus.publish(Event::ToolCallStart { call: call.clone() });

        let start = Instant::now();

        let result = tokio::select! {
            biased;
            _ = call_token.cancelled() => {
                ToolResult::cancelled(call.id)
            }
            _ = tokio::time::sleep(timeout) => {
                ToolResult::timeout(call.id)
            }
            output = tool.execute(call.clone()) => {
                match output {
                    Ok(output) => ToolResult::success(call.id, output, start.elapsed()),
                    Err(e) => ToolResult::error(call.id, e.to_string()),
                }
            }
        };

        self.event_bus.publish(Event::ToolCallComplete {
            call_id: call.id,
            result: result.clone(),
        });

        result
    }

    /// Execute multiple tool calls with smart batching
    pub async fn execute_batch(
        &self,
        calls: Vec<ToolCall>,
        cancel_token: CancellationToken,
    ) -> Vec<ToolResult> {
        // Partition into parallel-safe and sequential
        let (parallel, sequential) = self.partition_calls(&calls);

        // Execute parallel calls concurrently
        let parallel_futures: Vec<_> = parallel
            .into_iter()
            .map(|call| {
                let token = cancel_token.clone();
                self.execute(call, token)
            })
            .collect();

        let mut results = futures::future::join_all(parallel_futures).await;

        // Execute sequential calls in order
        for call in sequential {
            if cancel_token.is_cancelled() {
                results.push(ToolResult::cancelled(call.id));
            } else {
                results.push(self.execute(call, cancel_token.clone()).await);
            }
        }

        results
    }

    fn partition_calls(&self, calls: &[ToolCall]) -> (Vec<ToolCall>, Vec<ToolCall>) {
        let mut parallel = Vec::new();
        let mut sequential = Vec::new();

        for call in calls {
            if let Some(tool) = self.registry.get(&call.tool_name) {
                match tool.concurrency_mode() {
                    ConcurrencyMode::Parallel => parallel.push(call.clone()),
                    ConcurrencyMode::Sequential => sequential.push(call.clone()),
                    ConcurrencyMode::Limited(_) => parallel.push(call.clone()),
                }
            }
        }

        (parallel, sequential)
    }
}
```

### 5.2 Tool Concurrency Diagram

```
+------------------------------------------------------------------+
|                   Tool Execution Flow                             |
+------------------------------------------------------------------+
|                                                                   |
|  Incoming Tool Calls: [T1, T2, T3, T4, T5]                       |
|                                                                   |
|         │                                                         |
|         v                                                         |
|  ┌─────────────────┐                                             |
|  │   Partitioner   │                                             |
|  └────────┬────────┘                                             |
|           │                                                       |
|     ┌─────┴─────┐                                                |
|     │           │                                                 |
|     v           v                                                 |
|  Parallel    Sequential                                           |
|  [T1,T2,T4]  [T3,T5]                                             |
|     │           │                                                 |
|     v           │                                                 |
|  ┌──────────────────────┐                                        |
|  │   Global Semaphore   │  (max: 8)                              |
|  │   ┌──┬──┬──┬──┬──┐  │                                        |
|  │   │T1│T2│T4│  │  │  │  (3 permits acquired)                   |
|  │   └──┴──┴──┴──┴──┘  │                                        |
|  └──────────────────────┘                                        |
|           │                                                       |
|     ┌─────┼─────┐                                                |
|     │     │     │                                                 |
|     v     v     v                                                 |
|  ┌────┐┌────┐┌────┐                                             |
|  │ T1 ││ T2 ││ T4 │  Running in parallel                        |
|  └──┬─┘└──┬─┘└──┬─┘                                             |
|     │     │     │                                                 |
|     v     v     v                                                 |
|  ┌──────────────────────┐                                        |
|  │   join_all(futures)  │                                        |
|  └──────────┬───────────┘                                        |
|             │                                                     |
|             v                                                     |
|  [R1, R2, R4]  ◄─── Results                                      |
|             │                                                     |
|             │  Then sequential:                                   |
|             v                                                     |
|          ┌────┐                                                  |
|          │ T3 │  ──────▶  R3                                     |
|          └────┘                                                  |
|             │                                                     |
|             v                                                     |
|          ┌────┐                                                  |
|          │ T5 │  ──────▶  R5                                     |
|          └────┘                                                  |
|             │                                                     |
|             v                                                     |
|  Final: [R1, R2, R4, R3, R5]                                     |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 6. Streaming Architecture

### 6.1 EventSourceIterator Pattern

```rust
use futures::{Stream, StreamExt};
use pin_project::pin_project;

/// Iterator for Server-Sent Events
#[pin_project]
pub struct EventSourceIterator<S> {
    #[pin]
    inner: S,
    cancel_token: CancellationToken,
    decoder: SSEDecoder,
}

impl<S> EventSourceIterator<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send,
{
    pub fn new(stream: S, cancel_token: CancellationToken) -> Self {
        Self {
            inner: stream,
            cancel_token,
            decoder: SSEDecoder::new(),
        }
    }

    /// Split into two independent iterators
    pub fn tee(self) -> (Self, Self) {
        // Implementation using broadcast channel
        todo!()
    }
}

impl<S> Stream for EventSourceIterator<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send,
{
    type Item = Result<SSEEvent, StreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // Check cancellation
        if this.cancel_token.is_cancelled() {
            return Poll::Ready(None);
        }

        // Poll inner stream
        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let events = this.decoder.decode(&bytes);
                if let Some(event) = events.into_iter().next() {
                    Poll::Ready(Some(Ok(event)))
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(StreamError::Network(e))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// SSE Decoder
pub struct SSEDecoder {
    buffer: String,
}

impl SSEDecoder {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    pub fn decode(&mut self, chunk: &[u8]) -> Vec<SSEEvent> {
        let text = String::from_utf8_lossy(chunk);
        self.buffer.push_str(&text);

        let mut events = Vec::new();

        // Parse complete events (ended with double newline)
        while let Some(pos) = self.buffer.find("\n\n") {
            let event_text = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 2..].to_string();

            if let Some(event) = self.parse_event(&event_text) {
                events.push(event);
            }
        }

        events
    }

    fn parse_event(&self, text: &str) -> Option<SSEEvent> {
        let mut event_type = None;
        let mut data = String::new();

        for line in text.lines() {
            if line.starts_with("event:") {
                event_type = Some(line[6..].trim().to_string());
            } else if line.starts_with("data:") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(line[5..].trim());
            }
        }

        Some(SSEEvent {
            event_type,
            data,
        })
    }
}
```

### 6.2 MessageStream Implementation

```rust
/// Handles streaming messages from LLM
pub struct MessageStream {
    /// Event sender
    event_tx: broadcast::Sender<StreamEvent>,

    /// Cancellation token
    cancel_token: CancellationToken,

    /// Accumulated message
    message: Arc<RwLock<PartialMessage>>,
}

pub enum StreamEvent {
    Connected,
    MessageStart(MessageId),
    ContentBlockStart { index: usize, block_type: BlockType },
    TextDelta(String),
    InputJsonDelta(String),
    ThinkingDelta(String),
    ContentBlockStop { index: usize },
    ToolUse(ToolCall),
    MessageComplete(Message),
    Error(StreamError),
    Disconnected,
}

impl MessageStream {
    /// Create from SSE response
    pub async fn from_response(
        response: reqwest::Response,
        cancel_token: CancellationToken,
    ) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(256);
        let message = Arc::new(RwLock::new(PartialMessage::new()));

        let stream = Self {
            event_tx: event_tx.clone(),
            cancel_token: cancel_token.clone(),
            message: message.clone(),
        };

        // Spawn task to process stream
        tokio::spawn(async move {
            let sse = EventSourceIterator::new(
                response.bytes_stream(),
                cancel_token,
            );

            tokio::pin!(sse);

            while let Some(event) = sse.next().await {
                match event {
                    Ok(sse_event) => {
                        let stream_event = Self::parse_sse_event(sse_event);
                        let _ = event_tx.send(stream_event);
                    }
                    Err(e) => {
                        let _ = event_tx.send(StreamEvent::Error(e.into()));
                        break;
                    }
                }
            }

            let _ = event_tx.send(StreamEvent::Disconnected);
        });

        Ok(stream)
    }

    /// Subscribe to stream events
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.event_tx.subscribe()
    }

    /// Collect all events into complete message
    pub async fn collect(self) -> Result<Message> {
        let mut receiver = self.subscribe();

        while let Ok(event) = receiver.recv().await {
            match event {
                StreamEvent::MessageComplete(msg) => return Ok(msg),
                StreamEvent::Error(e) => return Err(e.into()),
                StreamEvent::Disconnected => break,
                _ => continue,
            }
        }

        Err(SageError::stream("Stream ended without complete message"))
    }

    fn parse_sse_event(sse: SSEEvent) -> StreamEvent {
        // Parse based on event type
        match sse.event_type.as_deref() {
            Some("message_start") => {
                let data: MessageStartData = serde_json::from_str(&sse.data).unwrap();
                StreamEvent::MessageStart(data.message.id)
            }
            Some("content_block_start") => {
                let data: ContentBlockStartData = serde_json::from_str(&sse.data).unwrap();
                StreamEvent::ContentBlockStart {
                    index: data.index,
                    block_type: data.content_block.block_type,
                }
            }
            Some("content_block_delta") => {
                let data: ContentBlockDeltaData = serde_json::from_str(&sse.data).unwrap();
                match data.delta {
                    Delta::TextDelta { text } => StreamEvent::TextDelta(text),
                    Delta::InputJsonDelta { partial_json } => {
                        StreamEvent::InputJsonDelta(partial_json)
                    }
                    Delta::ThinkingDelta { thinking } => {
                        StreamEvent::ThinkingDelta(thinking)
                    }
                }
            }
            Some("message_stop") => {
                // Build complete message
                StreamEvent::MessageComplete(Message::default()) // placeholder
            }
            _ => StreamEvent::Error(StreamError::UnknownEvent),
        }
    }
}
```

### 6.3 Streaming Data Flow

```
+------------------------------------------------------------------+
|                    Streaming Data Flow                            |
+------------------------------------------------------------------+
|                                                                   |
|  HTTP Response (SSE)                                              |
|         │                                                         |
|         │  bytes stream                                           |
|         v                                                         |
|  ┌─────────────────────┐                                         |
|  │ EventSourceIterator │                                         |
|  │                     │                                         |
|  │  ┌───────────────┐  │                                         |
|  │  │  SSE Decoder  │  │                                         |
|  │  └───────┬───────┘  │                                         |
|  └──────────┼──────────┘                                         |
|             │                                                     |
|             │  SSE Events                                         |
|             v                                                     |
|  ┌─────────────────────┐                                         |
|  │   MessageStream     │                                         |
|  │                     │                                         |
|  │  ┌───────────────┐  │                                         |
|  │  │  Event Parser │  │                                         |
|  │  └───────┬───────┘  │                                         |
|  └──────────┼──────────┘                                         |
|             │                                                     |
|             │  StreamEvents                                       |
|             v                                                     |
|  ┌─────────────────────┐                                         |
|  │   broadcast::channel │                                         |
|  └──────────┬──────────┘                                         |
|             │                                                     |
|      ┌──────┼──────┬──────────┐                                  |
|      │      │      │          │                                   |
|      v      v      v          v                                   |
|  ┌──────┐┌──────┐┌──────┐┌──────┐                               |
|  │  UI  ││Agent ││Traj. ││Custom│                               |
|  │      ││      ││Record││      │                               |
|  └──────┘└──────┘└──────┘└──────┘                               |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 7. State Synchronization

### 7.1 State Ownership Rules

```
+------------------------------------------------------------------+
|                   State Ownership Rules                           |
+------------------------------------------------------------------+
|                                                                   |
|  RULE 1: Single Writer                                           |
|  ─────────────────────                                           |
|  Each piece of mutable state has exactly ONE owner               |
|                                                                   |
|    Session State    → Owned by SessionOrchestrator               |
|    Agent State      → Owned by Agent task                        |
|    Tool State       → Owned by Tool instance (if any)            |
|                                                                   |
|                                                                   |
|  RULE 2: Message Passing for Mutation                            |
|  ────────────────────────────────────                            |
|  To change state owned by another task, send a message           |
|                                                                   |
|    Agent → Session:  via event bus                               |
|    Session → Agent:  via agent mailbox                           |
|    Tool → Agent:     via result channel                          |
|                                                                   |
|                                                                   |
|  RULE 3: Arc<T> for Shared Read-Only                             |
|  ───────────────────────────────────                             |
|  Immutable data can be shared via Arc                            |
|                                                                   |
|    Config            → Arc<Config>                               |
|    ToolRegistry      → Arc<ToolRegistry>                         |
|    SystemPrompt      → Arc<String>                               |
|                                                                   |
|                                                                   |
|  RULE 4: RwLock Only When Necessary                              |
|  ──────────────────────────────────                              |
|  Use RwLock sparingly, with short critical sections              |
|                                                                   |
|    Cache             → Arc<RwLock<Cache>>                        |
|    Metrics           → Arc<RwLock<Metrics>>                      |
|                                                                   |
+------------------------------------------------------------------+
```

### 7.2 Synchronization Patterns

```rust
/// Pattern 1: Message passing for state updates
mod message_passing {
    use tokio::sync::mpsc;

    pub struct StateManager {
        state: SessionState,
        mailbox: mpsc::Receiver<StateUpdate>,
    }

    impl StateManager {
        pub async fn run(mut self) {
            while let Some(update) = self.mailbox.recv().await {
                self.apply_update(update);
            }
        }

        fn apply_update(&mut self, update: StateUpdate) {
            match update {
                StateUpdate::AgentSpawned(id) => {
                    self.state.agents.insert(id);
                }
                StateUpdate::AgentCompleted(id) => {
                    self.state.agents.remove(&id);
                }
                // ...
            }
        }
    }
}

/// Pattern 2: Arc for shared immutable data
mod shared_immutable {
    use std::sync::Arc;

    pub struct SharedConfig {
        inner: Arc<ConfigInner>,
    }

    impl SharedConfig {
        pub fn new(config: ConfigInner) -> Self {
            Self {
                inner: Arc::new(config),
            }
        }

        pub fn clone_ref(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }
}

/// Pattern 3: RwLock for cache
mod cache_pattern {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub struct Cache {
        inner: Arc<RwLock<CacheInner>>,
    }

    impl Cache {
        pub async fn get(&self, key: &str) -> Option<Value> {
            // Short read lock
            let guard = self.inner.read().await;
            guard.data.get(key).cloned()
        }

        pub async fn set(&self, key: String, value: Value) {
            // Short write lock
            let mut guard = self.inner.write().await;
            guard.data.insert(key, value);
        }
    }
}
```

---

## 8. Error Handling in Concurrent Context

### 8.1 Error Propagation Strategy

```rust
/// Errors that can occur in concurrent operations
#[derive(Debug, thiserror::Error)]
pub enum ConcurrencyError {
    #[error("Task was cancelled")]
    Cancelled,

    #[error("Task timed out after {0:?}")]
    Timeout(Duration),

    #[error("Channel closed unexpectedly")]
    ChannelClosed,

    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Semaphore closed")]
    SemaphoreClosed,
}

/// Result type for concurrent operations
pub type ConcurrentResult<T> = Result<T, ConcurrencyError>;

/// Pattern for handling errors in spawned tasks
async fn spawned_task_with_error_handling(
    cancel_token: CancellationToken,
    event_bus: EventBus,
) {
    let result = async {
        // Task work here
        Ok::<_, SageError>(())
    }.await;

    match result {
        Ok(_) => {
            // Success
        }
        Err(e) => {
            // Report error via event bus
            event_bus.publish(Event::Error {
                error: e,
                context: ErrorContext::current(),
            });
        }
    }
}
```

### 8.2 Supervision Strategy

```
+------------------------------------------------------------------+
|                    Supervision Strategy                           |
+------------------------------------------------------------------+
|                                                                   |
|  Session Orchestrator (Supervisor)                                |
|         │                                                         |
|         │  monitors                                               |
|         v                                                         |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                      Agent Tasks                             │ |
|  │                                                              │ |
|  │   On Error:                                                  │ |
|  │   1. Log error with context                                 │ |
|  │   2. Publish error event                                    │ |
|  │   3. Attempt graceful cleanup                               │ |
|  │   4. Report to supervisor                                   │ |
|  │                                                              │ |
|  │   Supervisor Response:                                       │ |
|  │   - Transient error → Retry with backoff                    │ |
|  │   - Permanent error → Mark agent as failed                  │ |
|  │   - Panic → Isolate, don't crash session                    │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
|  Tool Executor (Supervisor)                                       |
|         │                                                         |
|         │  monitors                                               |
|         v                                                         |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                       Tool Tasks                             │ |
|  │                                                              │ |
|  │   On Error:                                                  │ |
|  │   1. Capture error in ToolResult                            │ |
|  │   2. Release semaphore permit                               │ |
|  │   3. Return error result to agent                           │ |
|  │                                                              │ |
|  │   On Timeout:                                                │ |
|  │   1. Cancel tool task                                       │ |
|  │   2. Return timeout result                                  │ |
|  │                                                              │ |
|  │   On Panic:                                                  │ |
|  │   1. Catch via catch_unwind or JoinHandle                   │ |
|  │   2. Return error result                                    │ |
|  │   3. Other tools continue unaffected                        │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 9. Performance Considerations

### 9.1 Optimization Guidelines

```
+------------------------------------------------------------------+
|                  Performance Guidelines                           |
+------------------------------------------------------------------+
|                                                                   |
|  1. AVOID HOLDING LOCKS ACROSS AWAIT POINTS                      |
|     ─────────────────────────────────────                        |
|     Bad:                                                          |
|       let guard = lock.read().await;                             |
|       do_async_work().await;  // Still holding lock!             |
|       drop(guard);                                                |
|                                                                   |
|     Good:                                                         |
|       let data = {                                                |
|           let guard = lock.read().await;                         |
|           guard.clone()                                           |
|       };                                                          |
|       do_async_work().await;                                      |
|                                                                   |
|                                                                   |
|  2. USE BOUNDED CHANNELS                                          |
|     ────────────────────                                         |
|     - Prevents memory exhaustion                                 |
|     - Provides natural backpressure                              |
|     - mpsc::channel(32) not unbounded()                          |
|                                                                   |
|                                                                   |
|  3. BATCH OPERATIONS WHERE POSSIBLE                              |
|     ───────────────────────────────                              |
|     - Combine multiple small writes                              |
|     - Use join_all for parallel operations                       |
|     - Buffer events before broadcasting                          |
|                                                                   |
|                                                                   |
|  4. PREFER CLONE OVER ARC<RWLOCK>                                |
|     ─────────────────────────────                                |
|     - Small data: clone is faster than lock                      |
|     - Arc<T> for large immutable data                            |
|     - RwLock only for truly shared mutable state                 |
|                                                                   |
+------------------------------------------------------------------+
```

### 9.2 Metrics to Monitor

```rust
/// Key concurrency metrics
pub struct ConcurrencyMetrics {
    /// Number of active agent tasks
    pub active_agents: AtomicUsize,

    /// Number of tools currently executing
    pub active_tools: AtomicUsize,

    /// Number of pending tool calls in queue
    pub queued_tools: AtomicUsize,

    /// Channel buffer utilization (0-100%)
    pub channel_utilization: AtomicU8,

    /// Number of cancelled operations
    pub cancellation_count: AtomicU64,

    /// Number of timeouts
    pub timeout_count: AtomicU64,
}
```

---

## 10. Summary

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Runtime | Tokio multi-thread | Best ecosystem, performance |
| Inter-task communication | Channels | Safer than shared state |
| Cancellation | Token hierarchy | Structured cancellation |
| Event distribution | Broadcast channel | Multiple subscribers |
| Concurrency limit | Semaphore | Bounded resource usage |
| State ownership | Single writer | Prevents races |

### Critical Invariants

1. **No deadlocks**: Never hold locks across await points
2. **Graceful cancellation**: All tasks respect cancellation tokens
3. **Bounded resources**: All channels and pools are bounded
4. **Error isolation**: One task failure doesn't crash others
5. **Backpressure**: Producers slow down when consumers are slow
