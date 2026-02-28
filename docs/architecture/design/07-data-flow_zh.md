# Data Flow Design

> How data flows through the Sage Agent system

## 1. Overview

This document describes the major data flows in the Sage Agent system:

1. **Chat Flow**: User message to agent response
2. **Streaming Flow**: SSE events from LLM to UI
3. **Tool Execution Flow**: Tool call to result
4. **Event Distribution Flow**: System events to subscribers

---

## 2. Chat Flow

### 2.1 Sequence Diagram

```
+=====================================================================+
|                         CHAT FLOW                                    |
+=====================================================================+
|                                                                      |
|  User    CLI    Session    Agent    LLMClient    Tool     EventBus  |
|   |       |        |         |          |         |          |      |
|   | input |        |         |          |         |          |      |
|   |------>|        |         |          |         |          |      |
|   |       | chat() |         |          |         |          |      |
|   |       |------->|         |          |         |          |      |
|   |       |        | spawn() |          |         |          |      |
|   |       |        |-------->|          |         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         | stream() |         |          |      |
|   |       |        |         |--------->|         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         |<---------|         |          |      |
|   |       |        |         | events   |         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         |------ publish -----|--------->|      |
|   |       |        |         |          |         |          |      |
|   |       |        |         | tool_use |         |          |      |
|   |       |        |         |----------------->  |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         |<-----------------  |          |      |
|   |       |        |         | result   |         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         | continue |         |          |      |
|   |       |        |         |--------->|         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |         |<---------|         |          |      |
|   |       |        |         | done     |         |          |      |
|   |       |        |         |          |         |          |      |
|   |       |        |<--------|          |         |          |      |
|   |       |        | result  |          |         |          |      |
|   |       |<-------|         |          |         |          |      |
|   |       | display|         |          |         |          |      |
|   |<------|        |         |          |         |          |      |
|   | output|        |         |          |         |          |      |
|   |       |        |         |          |         |          |      |
|                                                                      |
+======================================================================+
```

### 2.2 Data Transformations

```
+------------------------------------------------------------------+
|                    CHAT DATA TRANSFORMATIONS                      |
+------------------------------------------------------------------+
|                                                                   |
|  User Input (String)                                              |
|       |                                                           |
|       v                                                           |
|  +------------------+                                             |
|  | Parse & Validate |                                             |
|  +--------+---------+                                             |
|           |                                                       |
|           v                                                       |
|  UserMessage {                                                    |
|    content: String,                                               |
|    attachments: Vec<Attachment>,                                  |
|    metadata: MessageMetadata,                                     |
|  }                                                                |
|       |                                                           |
|       v                                                           |
|  +------------------+                                             |
|  | Build Context    |                                             |
|  +--------+---------+                                             |
|           |                                                       |
|           v                                                       |
|  ChatRequest {                                                    |
|    system: String,                                                |
|    messages: Vec<Message>,                                        |
|    tools: Vec<ToolSchema>,                                        |
|    max_tokens: usize,                                             |
|    stream: true,                                                  |
|  }                                                                |
|       |                                                           |
|       v                                                           |
|  +------------------+                                             |
|  | Serialize JSON   |                                             |
|  +--------+---------+                                             |
|           |                                                       |
|           v                                                       |
|  HTTP POST /messages (body: JSON)                                 |
|       |                                                           |
|       v                                                           |
|  SSE Stream                                                       |
|       |                                                           |
|       v                                                           |
|  +------------------+                                             |
|  | Parse SSE Events |                                             |
|  +--------+---------+                                             |
|           |                                                       |
|           v                                                       |
|  StreamEvent::ContentBlockDelta { text }                          |
|       |                                                           |
|       v                                                           |
|  +------------------+                                             |
|  | Accumulate       |                                             |
|  +--------+---------+                                             |
|           |                                                       |
|           v                                                       |
|  Message {                                                        |
|    role: Assistant,                                               |
|    content: Vec<ContentBlock>,                                    |
|  }                                                                |
|       |                                                           |
|       v                                                           |
|  Display to User                                                  |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 3. Streaming Flow

### 3.1 SSE Event Processing Pipeline

```
+=====================================================================+
|                    SSE STREAMING PIPELINE                            |
+=====================================================================+
|                                                                      |
|  HTTP Response                                                       |
|  (Transfer-Encoding: chunked)                                        |
|       |                                                              |
|       | bytes                                                        |
|       v                                                              |
|  +------------------------+                                          |
|  |    Bytes Stream        |                                          |
|  |  (reqwest Response)    |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | Bytes                                                        |
|       v                                                              |
|  +------------------------+                                          |
|  |    SSE Decoder         |                                          |
|  |                        |                                          |
|  |  - Buffer management   |                                          |
|  |  - Event parsing       |                                          |
|  |  - Incomplete handling |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | SSEEvent { event_type, data }                                |
|       v                                                              |
|  +------------------------+                                          |
|  |   Event Classifier     |                                          |
|  |                        |                                          |
|  |  message_start    ─────┼──> MessageStart                          |
|  |  content_block_start ──┼──> ContentBlockStart                     |
|  |  content_block_delta ──┼──> ContentBlockDelta                     |
|  |  content_block_stop  ──┼──> ContentBlockStop                      |
|  |  message_delta     ────┼──> MessageDelta                          |
|  |  message_stop      ────┼──> MessageStop                           |
|  |  error            ─────┼──> Error                                 |
|  +------------------------+                                          |
|       |                                                              |
|       | StreamEvent                                                  |
|       v                                                              |
|  +------------------------+                                          |
|  |   Content Accumulator  |                                          |
|  |                        |                                          |
|  |  - Text buffer         |                                          |
|  |  - Tool input buffer   |                                          |
|  |  - Thinking buffer     |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | (fork via broadcast)                                         |
|       v                                                              |
|  +------------------------+                                          |
|  |    Event Bus           |                                          |
|  +------------------------+                                          |
|       |                                                              |
|  +----+----+----+----+                                               |
|  |    |    |    |    |                                               |
|  v    v    v    v    v                                               |
| UI  Agent Traj Tele  User                                            |
|     Logic Rec  metry Callback                                        |
|                                                                      |
+======================================================================+
```

### 3.2 SSE Event Types

```rust
/// Raw SSE event from HTTP stream
pub struct RawSSEEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// Anthropic streaming event types
pub enum AnthropicStreamEvent {
    /// Initial message metadata
    MessageStart {
        message: MessageMetadata,
    },

    /// Start of a content block
    ContentBlockStart {
        index: usize,
        content_block: ContentBlockType,
    },

    /// Incremental content update
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },

    /// End of a content block
    ContentBlockStop {
        index: usize,
    },

    /// Message-level update (usage stats)
    MessageDelta {
        delta: MessageDeltaInfo,
        usage: Usage,
    },

    /// End of message
    MessageStop,

    /// Ping (keepalive)
    Ping,

    /// Error event
    Error {
        error_type: String,
        message: String,
    },
}

/// Content delta types
pub enum ContentDelta {
    TextDelta {
        text: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    ThinkingDelta {
        thinking: String,
    },
}
```

### 3.3 Streaming State Management

```
+------------------------------------------------------------------+
|                  STREAMING STATE MANAGEMENT                       |
+------------------------------------------------------------------+
|                                                                   |
|  PartialMessage {                                                 |
|    id: MessageId,                                                 |
|    content_blocks: Vec<PartialContentBlock>,                      |
|    usage: Option<Usage>,                                          |
|  }                                                                |
|                                                                   |
|  PartialContentBlock {                                            |
|    index: usize,                                                  |
|    block_type: BlockType,                                         |
|    buffer: ContentBuffer,                                         |
|    complete: bool,                                                |
|  }                                                                |
|                                                                   |
|  ContentBuffer {                                                  |
|    Text(String),                                                  |
|    ToolUse {                                                      |
|      id: String,                                                  |
|      name: String,                                                |
|      input_json: String,   // Accumulated JSON                    |
|    },                                                             |
|    Thinking(String),                                              |
|  }                                                                |
|                                                                   |
|  State Transitions:                                               |
|  ─────────────────                                                |
|                                                                   |
|  1. message_start                                                 |
|     -> Create PartialMessage                                      |
|                                                                   |
|  2. content_block_start { index, type }                           |
|     -> Add PartialContentBlock at index                           |
|                                                                   |
|  3. content_block_delta { index, delta }                          |
|     -> Append delta to block[index].buffer                        |
|                                                                   |
|  4. content_block_stop { index }                                  |
|     -> Mark block[index].complete = true                          |
|     -> If ToolUse: Parse JSON, emit ToolCall                      |
|                                                                   |
|  5. message_stop                                                  |
|     -> Finalize message                                           |
|     -> Emit complete Message                                      |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 4. Tool Execution Flow

### 4.1 Tool Call Pipeline

```
+=====================================================================+
|                    TOOL EXECUTION PIPELINE                           |
+=====================================================================+
|                                                                      |
|  ToolCall {                                                          |
|    id: "toolu_xxx",                                                  |
|    name: "read_file",                                                |
|    input: { "path": "/src/main.rs" },                                |
|  }                                                                   |
|       |                                                              |
|       v                                                              |
|  +------------------------+                                          |
|  |   Tool Registry        |                                          |
|  |   Lookup by name       |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | Arc<dyn Tool>                                                |
|       v                                                              |
|  +------------------------+                                          |
|  |   Input Validation     |                                          |
|  |   (JSON Schema)        |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | Valid input                                                  |
|       v                                                              |
|  +------------------------+                                          |
|  |  Permission Check      |                                          |
|  |                        |                                          |
|  |  Allow? ───────────────┼──> Continue                              |
|  |  Deny?  ───────────────┼──> Return PermissionDenied               |
|  |  Ask?   ───────────────┼──> Pause, wait for user                  |
|  +------------------------+                                          |
|       |                                                              |
|       | Allowed                                                      |
|       v                                                              |
|  +------------------------+                                          |
|  |  Acquire Semaphore     |                                          |
|  |  Permit                 |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | Permit acquired                                              |
|       v                                                              |
|  +------------------------+                                          |
|  |  Execute with Timeout  |                                          |
|  |                        |                                          |
|  |  select! {             |                                          |
|  |    tool.execute() ─────┼──> Result                                |
|  |    timeout() ──────────┼──> Timeout                               |
|  |    cancel() ───────────┼──> Cancelled                             |
|  |  }                     |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       | ToolResult                                                   |
|       v                                                              |
|  +------------------------+                                          |
|  |  Emit Event            |                                          |
|  |  ToolCallComplete      |                                          |
|  +------------------------+                                          |
|       |                                                              |
|       v                                                              |
|  Return to Agent for next LLM call                                   |
|                                                                      |
+======================================================================+
```

### 4.2 Parallel Tool Execution

```
+------------------------------------------------------------------+
|                  PARALLEL TOOL EXECUTION                          |
+------------------------------------------------------------------+
|                                                                   |
|  Input: [ToolCall1, ToolCall2, ToolCall3, ToolCall4]              |
|                                                                   |
|       |                                                           |
|       v                                                           |
|  +------------------------+                                       |
|  |   Partition by         |                                       |
|  |   Concurrency Mode     |                                       |
|  +------------------------+                                       |
|       |                                                           |
|  +----+----+                                                      |
|  |         |                                                      |
|  v         v                                                      |
| Parallel  Sequential                                              |
| [T1,T2,T4] [T3]                                                   |
|  |         |                                                      |
|  v         |                                                      |
|  +------------------------+                                       |
|  |   join_all(            |                                       |
|  |     T1.execute(),      |                                       |
|  |     T2.execute(),      |                                       |
|  |     T4.execute(),      |                                       |
|  |   )                    |                                       |
|  +------------------------+                                       |
|       |                                                           |
|       | [R1, R2, R4]                                              |
|       |                                                           |
|       |         |                                                 |
|       |         v                                                 |
|       |    +------------------------+                             |
|       |    |   T3.execute()         |                             |
|       |    +------------------------+                             |
|       |         |                                                 |
|       |         | R3                                              |
|       |         |                                                 |
|       +----+----+                                                 |
|            |                                                      |
|            v                                                      |
|  Output: [R1, R2, R3, R4]                                         |
|                                                                   |
+------------------------------------------------------------------+
```

### 4.3 Tool Result Format

```rust
/// Tool execution result
pub struct ToolResult {
    /// Reference to the tool call
    pub call_id: ToolCallId,

    /// Whether execution succeeded
    pub is_error: bool,

    /// Result content
    pub content: ToolResultContent,

    /// Execution metrics
    pub metrics: ExecutionMetrics,
}

/// Tool result content variants
pub enum ToolResultContent {
    /// Text output
    Text(String),

    /// Structured JSON output
    Json(Value),

    /// Image output
    Image {
        media_type: String,
        base64_data: String,
    },

    /// Multiple content items
    Multiple(Vec<ToolResultContent>),

    /// Error message
    Error {
        message: String,
        details: Option<String>,
    },
}

/// Execution metrics
pub struct ExecutionMetrics {
    /// Total execution time
    pub duration: Duration,

    /// Time spent waiting for permit
    pub queue_time: Duration,

    /// Bytes read/written
    pub io_bytes: IoMetrics,
}
```

---

## 5. Event Distribution Flow

### 5.1 Event Bus Architecture

```
+=====================================================================+
|                    EVENT BUS ARCHITECTURE                            |
+=====================================================================+
|                                                                      |
|                     Publishers                                       |
|  ┌─────────────┬─────────────┬─────────────┬─────────────┐          |
|  │   Session   │    Agent    │    Tool     │   Stream    │          |
|  │ Orchestrator│   Executor  │  Executor   │   Handler   │          |
|  └──────┬──────┴──────┬──────┴──────┬──────┴──────┬──────┘          |
|         │             │             │             │                  |
|         │  Event      │  Event      │  Event      │  Event          |
|         │             │             │             │                  |
|         v             v             v             v                  |
|  +─────────────────────────────────────────────────────────+        |
|  │                                                         │        |
|  │                    EVENT BUS                            │        |
|  │                                                         │        |
|  │  ┌───────────────────────────────────────────────────┐ │        |
|  │  │           broadcast::channel<Event>               │ │        |
|  │  │                                                   │ │        |
|  │  │  Capacity: 1024                                   │ │        |
|  │  │  Lagging: RecvError::Lagged (oldest dropped)     │ │        |
|  │  └───────────────────────────────────────────────────┘ │        |
|  │                                                         │        |
|  +──────────────────────────┬──────────────────────────────+        |
|                             │                                        |
|              ┌──────────────┼──────────────┐                        |
|              │              │              │                         |
|              v              v              v                         |
|         Subscriber 1   Subscriber 2   Subscriber N                  |
|         (UI)           (Trajectory)  (User callback)                |
|                                                                      |
|  Each subscriber gets its own Receiver                              |
|  Events are cloned to each receiver                                 |
|                                                                      |
+======================================================================+
```

### 5.2 Event Types and Routing

```rust
/// System events
#[derive(Debug, Clone)]
pub enum Event {
    // Session lifecycle
    SessionStarted(SessionId),
    SessionEnded(SessionId, EndReason),

    // Stream events
    StreamConnected,
    StreamDisconnected(DisconnectReason),

    // Message events
    MessageStart(MessageId),
    ContentBlockStart { index: usize, block_type: BlockType },
    TextDelta(String),
    ThinkingDelta(String),
    ContentBlockStop(usize),
    MessageComplete(Message),

    // Agent events
    AgentSpawned(AgentId, AgentType),
    AgentStateChanged(AgentId, AgentState, AgentState),
    AgentCompleted(AgentId, TaskResult),

    // Tool events
    ToolCallStart(ToolCall),
    ToolCallProgress(ToolCallId, f32, Option<String>),
    ToolCallComplete(ToolCallId, ToolResult),

    // Permission events
    PermissionRequested(RequestId, PermissionRequest),
    PermissionResponded(RequestId, PermissionDecision),

    // Error events
    Error(SageError),
}

/// Event subscriber with filtering
pub struct FilteredSubscriber {
    receiver: broadcast::Receiver<Event>,
    filter: EventFilter,
}

pub enum EventFilter {
    All,
    ByType(Vec<EventType>),
    ByAgent(AgentId),
    BySession(SessionId),
    Custom(Box<dyn Fn(&Event) -> bool + Send + Sync>),
}

impl FilteredSubscriber {
    pub async fn next(&mut self) -> Option<Event> {
        loop {
            match self.receiver.recv().await {
                Ok(event) if self.filter.matches(&event) => return Some(event),
                Ok(_) => continue, // Filtered out
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!("Subscriber lagged, missed {} events", n);
                    continue;
                }
                Err(RecvError::Closed) => return None,
            }
        }
    }
}
```

### 5.3 Event Processing Patterns

```
+------------------------------------------------------------------+
|                   EVENT PROCESSING PATTERNS                       |
+------------------------------------------------------------------+
|                                                                   |
|  Pattern 1: UI Renderer (Real-time display)                       |
|  ─────────────────────────────────────────                        |
|                                                                   |
|  loop {                                                           |
|    match event_bus.subscribe().recv().await {                     |
|      Event::TextDelta(text) => {                                  |
|        terminal.print(text);                                      |
|        terminal.flush();                                          |
|      }                                                            |
|      Event::ToolCallStart(call) => {                              |
|        spinner.start(format!("Running {}...", call.name));        |
|      }                                                            |
|      Event::ToolCallComplete(id, result) => {                     |
|        spinner.stop();                                            |
|        display_result(result);                                    |
|      }                                                            |
|      _ => {}                                                      |
|    }                                                              |
|  }                                                                |
|                                                                   |
|                                                                   |
|  Pattern 2: Trajectory Recorder (Batch processing)                |
|  ─────────────────────────────────────────────────                |
|                                                                   |
|  let mut batch = Vec::with_capacity(100);                         |
|                                                                   |
|  loop {                                                           |
|    select! {                                                      |
|      event = subscriber.recv() => {                               |
|        batch.push(TrajectoryEntry::from(event));                  |
|        if batch.len() >= 100 {                                    |
|          writer.write_batch(&batch).await;                        |
|          batch.clear();                                           |
|        }                                                          |
|      }                                                            |
|      _ = flush_interval.tick() => {                               |
|        if !batch.is_empty() {                                     |
|          writer.write_batch(&batch).await;                        |
|          batch.clear();                                           |
|        }                                                          |
|      }                                                            |
|    }                                                              |
|  }                                                                |
|                                                                   |
|                                                                   |
|  Pattern 3: Telemetry Aggregator (Sampling)                       |
|  ──────────────────────────────────────────                       |
|                                                                   |
|  let mut metrics = Metrics::new();                                |
|                                                                   |
|  loop {                                                           |
|    select! {                                                      |
|      event = subscriber.recv() => {                               |
|        match event {                                              |
|          Event::ToolCallComplete(_, result) => {                  |
|            metrics.record_tool_duration(result.duration);         |
|          }                                                        |
|          Event::Error(e) => {                                     |
|            metrics.increment_errors();                            |
|          }                                                        |
|          _ => {}                                                  |
|        }                                                          |
|      }                                                            |
|      _ = report_interval.tick() => {                              |
|        telemetry.report(metrics.snapshot()).await;                |
|        metrics.reset();                                           |
|      }                                                            |
|    }                                                              |
|  }                                                                |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 6. Data Persistence Flow

### 6.1 Session Persistence

```
+------------------------------------------------------------------+
|                   SESSION PERSISTENCE FLOW                        |
+------------------------------------------------------------------+
|                                                                   |
|  In-Memory State                                                  |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                                                             │ |
|  │  Session {                                                  │ |
|  │    messages: Vec<Message>,                                  │ |
|  │    agents: Vec<AgentHandle>,                                │ |
|  │    state: SessionState,                                     │ |
|  │  }                                                          │ |
|  │                                                             │ |
|  └────────────────────────┬────────────────────────────────────┘ |
|                           │                                       |
|                           │ Periodic / On-Change                  |
|                           v                                       |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                   Serializer                                │ |
|  │                                                             │ |
|  │  Session -> SessionSnapshot {                               │ |
|  │    id,                                                      │ |
|  │    messages: Vec<SerializedMessage>,                        │ |
|  │    state,                                                   │ |
|  │    timestamp,                                               │ |
|  │  }                                                          │ |
|  └────────────────────────┬────────────────────────────────────┘ |
|                           │                                       |
|                           │ JSON/MessagePack                      |
|                           v                                       |
|  ┌─────────────────────────────────────────────────────────────┐ |
|  │                   Storage Backend                           │ |
|  │                                                             │ |
|  │  Option 1: File System (JSONL)                              │ |
|  │  ~/.sage/sessions/{session_id}.jsonl                        │ |
|  │                                                             │ |
|  │  Option 2: SQLite                                           │ |
|  │  ~/.sage/sage.db                                            │ |
|  │                                                             │ |
|  │  Option 3: Remote (future)                                  │ |
|  │  https://api.sage.dev/sessions                              │ |
|  │                                                             │ |
|  └─────────────────────────────────────────────────────────────┘ |
|                                                                   |
+------------------------------------------------------------------+
```

### 6.2 Trajectory Recording

```
+------------------------------------------------------------------+
|                   TRAJECTORY RECORDING FLOW                       |
+------------------------------------------------------------------+
|                                                                   |
|  Events                                                           |
|       │                                                           |
|       v                                                           |
|  ┌────────────────────┐                                          |
|  │  Event to Entry    │                                          |
|  │  Transformation    │                                          |
|  └─────────┬──────────┘                                          |
|            │                                                      |
|            v                                                      |
|  TrajectoryEntry {                                                |
|    timestamp: DateTime<Utc>,                                      |
|    entry_type: EntryType,                                         |
|    data: EntryData,                                               |
|  }                                                                |
|            │                                                      |
|            v                                                      |
|  ┌────────────────────┐                                          |
|  │   Buffer (100)     │                                          |
|  └─────────┬──────────┘                                          |
|            │                                                      |
|            │ flush on: buffer full / timeout / shutdown           |
|            v                                                      |
|  ┌────────────────────┐                                          |
|  │   Write to File    │                                          |
|  │                    │                                          |
|  │   Format: JSONL    │                                          |
|  │   Compression: zstd│                                          |
|  └────────────────────┘                                          |
|            │                                                      |
|            v                                                      |
|  ~/.sage/trajectories/{task_id}.jsonl.zst                        |
|                                                                   |
|  Entry Types:                                                     |
|  - TaskStart { description, agent_type }                         |
|  - LLMRequest { messages, tools }                                |
|  - LLMResponse { content, tool_calls }                           |
|  - ToolExecution { call, result, duration }                      |
|  - TaskComplete { result }                                       |
|  - Error { error, context }                                      |
|                                                                   |
+------------------------------------------------------------------+
```

---

## 7. Configuration Flow

```
+------------------------------------------------------------------+
|                    CONFIGURATION LOADING FLOW                     |
+------------------------------------------------------------------+
|                                                                   |
|  Priority (highest to lowest):                                    |
|                                                                   |
|  1. CLI Arguments                                                 |
|     sage --model opus --timeout 300                               |
|            │                                                      |
|            v                                                      |
|  2. Environment Variables                                         |
|     SAGE_MODEL=opus                                               |
|     ANTHROPIC_API_KEY=xxx                                         |
|            │                                                      |
|            v                                                      |
|  3. Project Config                                                |
|     ./.sage/config.toml                                           |
|            │                                                      |
|            v                                                      |
|  4. User Config                                                   |
|     ~/.config/sage/config.toml                                    |
|            │                                                      |
|            v                                                      |
|  5. Default Values                                                |
|     Compiled into binary                                          |
|            │                                                      |
|            v                                                      |
|  ┌────────────────────────────────────────────────────────────┐  |
|  │                    Config Merger                            │  |
|  │                                                             │  |
|  │  for each field:                                            │  |
|  │    if cli.field.is_some() { use cli.field }                │  |
|  │    else if env.field.is_some() { use env.field }           │  |
|  │    else if project.field.is_some() { use project.field }   │  |
|  │    else if user.field.is_some() { use user.field }         │  |
|  │    else { use default.field }                              │  |
|  │                                                             │  |
|  └────────────────────────────────────────────────────────────┘  |
|            │                                                      |
|            v                                                      |
|  ┌────────────────────────────────────────────────────────────┐  |
|  │                    Validator                                │  |
|  │                                                             │  |
|  │  - Check required fields present                           │  |
|  │  - Validate value ranges                                   │  |
|  │  - Check file paths exist                                  │  |
|  │  - Verify API keys format                                  │  |
|  │                                                             │  |
|  └────────────────────────────────────────────────────────────┘  |
|            │                                                      |
|            v                                                      |
|  Config {                                                         |
|    llm: LLMConfig { provider, model, api_key, ... },             |
|    tools: ToolsConfig { enabled, permissions, ... },             |
|    session: SessionConfig { timeout, max_steps, ... },           |
|    ui: UIConfig { theme, verbosity, ... },                       |
|  }                                                                |
|                                                                   |
+------------------------------------------------------------------+
```
