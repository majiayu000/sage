# State Machine Design

> Formal state definitions and transitions for Sage components

## 1. Overview

State machines provide a rigorous way to model component behavior. Each
state machine defines:

- **States**: All possible states the component can be in
- **Events**: Inputs that trigger state transitions
- **Transitions**: Rules for moving between states
- **Actions**: Side effects that occur during transitions
- **Guards**: Conditions that must be true for a transition

---

## 2. Session State Machine

### 2.1 State Diagram

```
+=====================================================================+
|                    SESSION STATE MACHINE                             |
+=====================================================================+
|                                                                      |
|                          ┌─────────────┐                            |
|                          │             │                            |
|                   ┌─────▶│ INITIALIZING│                            |
|                   │      │             │                            |
|                   │      └──────┬──────┘                            |
|                   │             │                                    |
|                   │             │ initialized()                      |
|                   │             ▼                                    |
|     new()         │      ┌─────────────┐                            |
|     ─────────────►│      │             │                            |
|                   │      │   ACTIVE    │◄───────────────────┐       |
|                   │      │             │                    │       |
|                   │      └──────┬──────┘                    │       |
|                   │             │                           │       |
|                   │      ┌──────┴──────┐                    │       |
|                   │      │             │                    │       |
|                   │      │ chat()      │ agent_complete()   │       |
|                   │      ▼             │                    │       |
|                   │ ┌─────────────┐    │                    │       |
|                   │ │             │    │                    │       |
|                   │ │ PROCESSING  │────┘                    │       |
|                   │ │             │                         │       |
|                   │ └──────┬──────┘                         │       |
|                   │        │                                │       |
|                   │        │ need_input()                   │       |
|                   │        ▼                                │       |
|                   │ ┌─────────────┐     resume()           │       |
|                   │ │             │─────────────────────────┘       |
|                   │ │   PAUSED    │                                 |
|                   │ │             │                                 |
|                   │ └──────┬──────┘                                 |
|                   │        │                                        |
|                   │        │ end() / error() / timeout()           |
|                   │        ▼                                        |
|                   │ ┌─────────────┐                                 |
|                   │ │             │                                 |
|                   └─│   ENDED     │                                 |
|                     │             │                                 |
|                     └─────────────┘                                 |
|                                                                      |
+======================================================================+
```

### 2.2 Formal Definition

```rust
/// Session states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session is being initialized
    Initializing,

    /// Session is active and ready for input
    Active,

    /// Session is processing a request
    Processing {
        /// ID of the currently active agent
        agent_id: AgentId,
    },

    /// Session is paused waiting for user input
    Paused {
        /// Reason for pause
        reason: PauseReason,
    },

    /// Session has ended
    Ended {
        /// How the session ended
        reason: EndReason,
    },
}

/// Events that trigger session state transitions
#[derive(Debug, Clone)]
pub enum SessionEvent {
    /// Session initialization completed
    Initialized,

    /// User submitted a chat message
    Chat { message: String },

    /// Agent needs user input
    NeedInput { prompt: String },

    /// User provided input
    Resume { input: String },

    /// Agent completed execution
    AgentComplete { result: AgentResult },

    /// Session should end
    End,

    /// Error occurred
    Error { error: SessionError },

    /// Timeout occurred
    Timeout,
}

/// Session state machine implementation
impl SessionState {
    /// Applies an event to the current state, returning the new state
    pub fn transition(self, event: SessionEvent) -> Result<Self, TransitionError> {
        match (self, event) {
            // Initializing -> Active
            (SessionState::Initializing, SessionEvent::Initialized) => {
                Ok(SessionState::Active)
            }

            // Active -> Processing
            (SessionState::Active, SessionEvent::Chat { .. }) => {
                Ok(SessionState::Processing {
                    agent_id: AgentId::new(),
                })
            }

            // Processing -> Active (agent complete)
            (SessionState::Processing { .. }, SessionEvent::AgentComplete { .. }) => {
                Ok(SessionState::Active)
            }

            // Processing -> Paused (need input)
            (SessionState::Processing { .. }, SessionEvent::NeedInput { prompt }) => {
                Ok(SessionState::Paused {
                    reason: PauseReason::NeedInput { prompt },
                })
            }

            // Paused -> Processing (resume)
            (SessionState::Paused { .. }, SessionEvent::Resume { .. }) => {
                Ok(SessionState::Processing {
                    agent_id: AgentId::new(),
                })
            }

            // Any -> Ended (end/error/timeout)
            (_, SessionEvent::End) => {
                Ok(SessionState::Ended {
                    reason: EndReason::Normal,
                })
            }
            (_, SessionEvent::Error { error }) => {
                Ok(SessionState::Ended {
                    reason: EndReason::Error(error),
                })
            }
            (_, SessionEvent::Timeout) => {
                Ok(SessionState::Ended {
                    reason: EndReason::Timeout,
                })
            }

            // Invalid transitions
            (state, event) => {
                Err(TransitionError::InvalidTransition {
                    from: format!("{:?}", state),
                    event: format!("{:?}", event),
                })
            }
        }
    }
}
```

---

## 3. Agent State Machine

### 3.1 State Diagram

```
+=====================================================================+
|                      AGENT STATE MACHINE                             |
+=====================================================================+
|                                                                      |
|                     ┌──────────────┐                                |
|                     │              │                                |
|              ┌─────▶│ INITIALIZING │                                |
|              │      │              │                                |
|              │      └──────┬───────┘                                |
|              │             │                                        |
|   spawn()    │             │ ready()                                |
|   ──────────►│             ▼                                        |
|              │      ┌──────────────┐                                |
|              │      │              │◄─────────────────────────┐     |
|              │      │    READY     │                          │     |
|              │      │              │                          │     |
|              │      └──────┬───────┘                          │     |
|              │             │                                  │     |
|              │             │ execute(task)                    │     |
|              │             ▼                                  │     |
|              │      ┌──────────────┐                          │     |
|              │      │              │                          │     |
|              │ ┌───▶│   THINKING   │◄─────────┐              │     |
|              │ │    │              │          │              │     |
|              │ │    └──────┬───────┘          │              │     |
|              │ │           │                  │              │     |
|              │ │    ┌──────┴──────┐          │              │     |
|              │ │    │             │          │              │     |
|              │ │    ▼             ▼          │              │     |
|              │ │ ┌────────┐  ┌────────────┐  │              │     |
|              │ │ │  TEXT  │  │ TOOL_CALLS │  │              │     |
|              │ │ │ OUTPUT │  │            │  │              │     |
|              │ │ └───┬────┘  └─────┬──────┘  │              │     |
|              │ │     │            │          │              │     |
|              │ │     │            │ dispatch_tools()        │     |
|              │ │     │            ▼          │              │     |
|              │ │     │     ┌─────────────┐   │              │     |
|              │ │     │     │  EXECUTING  │   │              │     |
|              │ │     │     │   TOOLS     │   │              │     |
|              │ │     │     └──────┬──────┘   │              │     |
|              │ │     │            │          │              │     |
|              │ │     │            │ tools_complete()        │     |
|              │ │     │            │          │              │     |
|              │ │     │            └──────────┘              │     |
|              │ │     │                                      │     |
|              │ │     │ end_turn()                           │     |
|              │ │     ▼                                      │     |
|              │ │ ┌──────────────┐    permission_granted()   │     |
|              │ │ │              │────────────────────────────┘     |
|              │ └─│   WAITING    │                                  |
|              │   │  PERMISSION  │                                  |
|              │   │              │                                  |
|              │   └──────┬───────┘                                  |
|              │          │                                          |
|              │          │ complete() / cancel() / error()          |
|              │          ▼                                          |
|              │   ┌──────────────┐                                  |
|              │   │              │                                  |
|              └───│   TERMINAL   │                                  |
|                  │  (Completed/ │                                  |
|                  │   Cancelled/ │                                  |
|                  │   Error)     │                                  |
|                  │              │                                  |
|                  └──────────────┘                                  |
|                                                                      |
+======================================================================+
```

### 3.2 Formal Definition

```rust
/// Agent states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is being initialized
    Initializing,

    /// Agent is ready to receive tasks
    Ready,

    /// Agent is waiting for LLM response
    Thinking {
        /// Current step number
        step: usize,
    },

    /// Agent is executing tools
    ExecutingTools {
        /// Tool calls being executed
        tool_calls: Vec<ToolCallId>,
        /// Number of completed calls
        completed: usize,
    },

    /// Agent is waiting for user permission
    WaitingPermission {
        /// The permission request
        request: PermissionRequest,
    },

    /// Agent completed successfully
    Completed {
        /// The final result
        result: TaskResult,
    },

    /// Agent was cancelled
    Cancelled,

    /// Agent encountered an error
    Error {
        /// The error that occurred
        error: AgentError,
    },
}

/// Events that trigger agent state transitions
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent finished initializing
    Ready,

    /// Start executing a task
    Execute { task: Task },

    /// LLM returned text output
    TextOutput { text: String },

    /// LLM requested tool calls
    ToolCalls { calls: Vec<ToolCall> },

    /// Tool execution completed
    ToolComplete { call_id: ToolCallId, result: ToolResult },

    /// All tools completed
    ToolsComplete,

    /// Need permission for operation
    NeedPermission { request: PermissionRequest },

    /// Permission was granted
    PermissionGranted,

    /// Permission was denied
    PermissionDenied { reason: String },

    /// Continue to next step
    Continue,

    /// Agent completed
    Complete { result: TaskResult },

    /// Agent cancelled
    Cancel,

    /// Error occurred
    Error { error: AgentError },
}

impl AgentState {
    /// Applies an event to the current state
    pub fn transition(self, event: AgentEvent) -> Result<Self, TransitionError> {
        match (self, event) {
            // Initializing -> Ready
            (AgentState::Initializing, AgentEvent::Ready) => {
                Ok(AgentState::Ready)
            }

            // Ready -> Thinking
            (AgentState::Ready, AgentEvent::Execute { .. }) => {
                Ok(AgentState::Thinking { step: 1 })
            }

            // Thinking -> ExecutingTools
            (AgentState::Thinking { .. }, AgentEvent::ToolCalls { calls }) => {
                let tool_calls = calls.iter().map(|c| c.id.clone()).collect();
                Ok(AgentState::ExecutingTools {
                    tool_calls,
                    completed: 0,
                })
            }

            // ExecutingTools -> ExecutingTools (tool complete)
            (
                AgentState::ExecutingTools { tool_calls, completed },
                AgentEvent::ToolComplete { .. }
            ) => {
                Ok(AgentState::ExecutingTools {
                    tool_calls,
                    completed: completed + 1,
                })
            }

            // ExecutingTools -> Thinking (all tools complete)
            (
                AgentState::ExecutingTools { .. },
                AgentEvent::ToolsComplete
            ) => {
                Ok(AgentState::Thinking { step: 1 }) // Increment step
            }

            // Thinking -> WaitingPermission
            (
                AgentState::Thinking { .. },
                AgentEvent::NeedPermission { request }
            ) => {
                Ok(AgentState::WaitingPermission { request })
            }

            // WaitingPermission -> Thinking
            (
                AgentState::WaitingPermission { .. },
                AgentEvent::PermissionGranted
            ) => {
                Ok(AgentState::Thinking { step: 1 })
            }

            // Any -> Completed
            (_, AgentEvent::Complete { result }) => {
                Ok(AgentState::Completed { result })
            }

            // Any -> Cancelled
            (_, AgentEvent::Cancel) => {
                Ok(AgentState::Cancelled)
            }

            // Any -> Error
            (_, AgentEvent::Error { error }) => {
                Ok(AgentState::Error { error })
            }

            // Invalid transition
            (state, event) => {
                Err(TransitionError::InvalidTransition {
                    from: format!("{:?}", state),
                    event: format!("{:?}", event),
                })
            }
        }
    }

    /// Check if state is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentState::Completed { .. } |
            AgentState::Cancelled |
            AgentState::Error { .. }
        )
    }
}
```

---

## 4. Tool Execution State Machine

### 4.1 State Diagram

```
+=====================================================================+
|                  TOOL EXECUTION STATE MACHINE                        |
+=====================================================================+
|                                                                      |
|                      ┌──────────────┐                               |
|                      │              │                               |
|               ┌─────▶│   PENDING    │                               |
|               │      │              │                               |
|               │      └──────┬───────┘                               |
|               │             │                                        |
|    call()     │             │ acquire_permit()                       |
|    ──────────►│             ▼                                        |
|               │      ┌──────────────┐                               |
|               │      │              │                               |
|               │      │   QUEUED     │                               |
|               │      │              │                               |
|               │      └──────┬───────┘                               |
|               │             │                                        |
|               │             │ permit_acquired()                      |
|               │             ▼                                        |
|               │      ┌──────────────┐    deny()    ┌───────────┐    |
|               │      │              │─────────────▶│           │    |
|               │      │  CHECKING    │              │  DENIED   │    |
|               │      │  PERMISSION  │              │           │    |
|               │      │              │              └───────────┘    |
|               │      └──────┬───────┘                               |
|               │             │                                        |
|               │             │ allow()                                |
|               │             ▼                                        |
|               │      ┌──────────────┐                               |
|               │      │              │                               |
|               │      │   RUNNING    │───────────────┐               |
|               │      │              │               │               |
|               │      └──────┬───────┘               │               |
|               │             │                       │               |
|               │      ┌──────┴──────┐               │               |
|               │      │             │               │               |
|               │      ▼             ▼               │               |
|               │ ┌──────────┐ ┌──────────┐         │               |
|               │ │          │ │          │         │ timeout()     |
|               │ │ SUCCESS  │ │  FAILED  │         │               |
|               │ │          │ │          │         │               |
|               │ └──────────┘ └──────────┘         │               |
|               │                                    │               |
|               │                                    ▼               |
|               │                             ┌──────────┐           |
|               │                             │          │           |
|               └─────────────────────────────│ TIMEOUT  │           |
|                                             │          │           |
|                                             └──────────┘           |
|                                                                      |
|   * Cancel event can transition from any state to CANCELLED         |
|                                                                      |
+======================================================================+
```

### 4.2 Formal Definition

```rust
/// Tool execution states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolExecutionState {
    /// Waiting to be scheduled
    Pending,

    /// Waiting for execution permit
    Queued {
        queued_at: Instant,
    },

    /// Checking if operation is permitted
    CheckingPermission,

    /// Execution in progress
    Running {
        started_at: Instant,
        progress: Option<f32>,
    },

    /// Execution completed successfully
    Success {
        result: ToolResult,
        duration: Duration,
    },

    /// Execution failed
    Failed {
        error: ToolError,
        duration: Duration,
    },

    /// Execution timed out
    Timeout {
        duration: Duration,
    },

    /// Execution was cancelled
    Cancelled,

    /// Permission was denied
    Denied {
        reason: String,
    },
}

/// Tool execution events
#[derive(Debug, Clone)]
pub enum ToolExecutionEvent {
    /// Start execution
    Start,

    /// Acquired execution permit
    PermitAcquired,

    /// Permission check passed
    PermissionAllowed,

    /// Permission check failed
    PermissionDenied { reason: String },

    /// Progress update
    Progress { percent: f32 },

    /// Execution completed
    Complete { result: ToolResult },

    /// Execution failed
    Fail { error: ToolError },

    /// Execution timed out
    Timeout,

    /// Cancel execution
    Cancel,
}

impl ToolExecutionState {
    pub fn transition(
        self,
        event: ToolExecutionEvent,
        now: Instant,
    ) -> Result<Self, TransitionError> {
        match (self, event) {
            // Pending -> Queued
            (ToolExecutionState::Pending, ToolExecutionEvent::Start) => {
                Ok(ToolExecutionState::Queued { queued_at: now })
            }

            // Queued -> CheckingPermission
            (ToolExecutionState::Queued { .. }, ToolExecutionEvent::PermitAcquired) => {
                Ok(ToolExecutionState::CheckingPermission)
            }

            // CheckingPermission -> Running
            (ToolExecutionState::CheckingPermission, ToolExecutionEvent::PermissionAllowed) => {
                Ok(ToolExecutionState::Running {
                    started_at: now,
                    progress: None,
                })
            }

            // CheckingPermission -> Denied
            (
                ToolExecutionState::CheckingPermission,
                ToolExecutionEvent::PermissionDenied { reason }
            ) => {
                Ok(ToolExecutionState::Denied { reason })
            }

            // Running -> Running (progress)
            (
                ToolExecutionState::Running { started_at, .. },
                ToolExecutionEvent::Progress { percent }
            ) => {
                Ok(ToolExecutionState::Running {
                    started_at,
                    progress: Some(percent),
                })
            }

            // Running -> Success
            (
                ToolExecutionState::Running { started_at, .. },
                ToolExecutionEvent::Complete { result }
            ) => {
                Ok(ToolExecutionState::Success {
                    result,
                    duration: now.duration_since(started_at),
                })
            }

            // Running -> Failed
            (
                ToolExecutionState::Running { started_at, .. },
                ToolExecutionEvent::Fail { error }
            ) => {
                Ok(ToolExecutionState::Failed {
                    error,
                    duration: now.duration_since(started_at),
                })
            }

            // Running -> Timeout
            (
                ToolExecutionState::Running { started_at, .. },
                ToolExecutionEvent::Timeout
            ) => {
                Ok(ToolExecutionState::Timeout {
                    duration: now.duration_since(started_at),
                })
            }

            // Any -> Cancelled
            (_, ToolExecutionEvent::Cancel) => {
                Ok(ToolExecutionState::Cancelled)
            }

            // Invalid
            (state, event) => {
                Err(TransitionError::InvalidTransition {
                    from: format!("{:?}", state),
                    event: format!("{:?}", event),
                })
            }
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ToolExecutionState::Success { .. } |
            ToolExecutionState::Failed { .. } |
            ToolExecutionState::Timeout { .. } |
            ToolExecutionState::Cancelled |
            ToolExecutionState::Denied { .. }
        )
    }
}
```

---

## 5. Stream Connection State Machine

### 5.1 State Diagram

```
+=====================================================================+
|                STREAM CONNECTION STATE MACHINE                       |
+=====================================================================+
|                                                                      |
|                    ┌──────────────┐                                 |
|                    │              │                                 |
|             ┌─────▶│ DISCONNECTED │◄─────────────────────┐          |
|             │      │              │                      │          |
|             │      └──────┬───────┘                      │          |
|             │             │                              │          |
|             │             │ connect()                    │          |
|             │             ▼                              │          |
|             │      ┌──────────────┐                      │          |
|             │      │              │     error()          │          |
|             │      │ CONNECTING   │──────────────────────┤          |
|             │      │              │                      │          |
|             │      └──────┬───────┘                      │          |
|             │             │                              │          |
|             │             │ connected()                  │          |
|             │             ▼                              │          |
|             │      ┌──────────────┐                      │          |
|             │      │              │     disconnect()     │          |
|             │      │  CONNECTED   │──────────────────────┤          |
|             │      │              │                      │          |
|             │      └──────┬───────┘                      │          |
|             │             │                              │          |
|             │             │ start_streaming()            │          |
|             │             ▼                              │          |
|             │      ┌──────────────┐                      │          |
|             │      │              │                      │          |
|             │      │  STREAMING   │──────────────────────┤          |
|             │      │              │     error() /        │          |
|             │      └──────┬───────┘     complete()       │          |
|             │             │                              │          |
|             │             │ complete()                   │          |
|             │             │                              │          |
|             └─────────────┘                              │          |
|                                                          │          |
|                 reconnect() ─────────────────────────────┘          |
|                                                                      |
+======================================================================+
```

### 5.2 Formal Definition

```rust
/// Stream connection states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    /// Not connected
    Disconnected,

    /// Connection in progress
    Connecting {
        attempt: u32,
        started_at: Instant,
    },

    /// Connected and ready
    Connected,

    /// Actively streaming data
    Streaming {
        started_at: Instant,
        bytes_received: usize,
    },

    /// Stream completed normally
    Completed,

    /// Connection failed
    Failed {
        error: StreamError,
        attempts: u32,
    },
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Connect,
    Connected,
    StartStreaming,
    DataReceived { bytes: usize },
    Complete,
    Error { error: StreamError },
    Disconnect,
    Reconnect,
}

impl StreamState {
    pub fn transition(self, event: StreamEvent, now: Instant) -> Result<Self, TransitionError> {
        match (self, event) {
            // Disconnected -> Connecting
            (StreamState::Disconnected, StreamEvent::Connect) => {
                Ok(StreamState::Connecting {
                    attempt: 1,
                    started_at: now,
                })
            }

            // Connecting -> Connected
            (StreamState::Connecting { .. }, StreamEvent::Connected) => {
                Ok(StreamState::Connected)
            }

            // Connected -> Streaming
            (StreamState::Connected, StreamEvent::StartStreaming) => {
                Ok(StreamState::Streaming {
                    started_at: now,
                    bytes_received: 0,
                })
            }

            // Streaming -> Streaming (data received)
            (
                StreamState::Streaming { started_at, bytes_received },
                StreamEvent::DataReceived { bytes }
            ) => {
                Ok(StreamState::Streaming {
                    started_at,
                    bytes_received: bytes_received + bytes,
                })
            }

            // Streaming -> Completed
            (StreamState::Streaming { .. }, StreamEvent::Complete) => {
                Ok(StreamState::Completed)
            }

            // Any -> Disconnected
            (_, StreamEvent::Disconnect) => {
                Ok(StreamState::Disconnected)
            }

            // Any -> Failed
            (state, StreamEvent::Error { error }) => {
                let attempts = match state {
                    StreamState::Connecting { attempt, .. } => attempt,
                    StreamState::Failed { attempts, .. } => attempts,
                    _ => 0,
                };
                Ok(StreamState::Failed { error, attempts })
            }

            // Failed -> Connecting (reconnect)
            (StreamState::Failed { attempts, .. }, StreamEvent::Reconnect) => {
                Ok(StreamState::Connecting {
                    attempt: attempts + 1,
                    started_at: now,
                })
            }

            (state, event) => {
                Err(TransitionError::InvalidTransition {
                    from: format!("{:?}", state),
                    event: format!("{:?}", event),
                })
            }
        }
    }
}
```

---

## 6. State Machine Utilities

### 6.1 Transition Table Generator

```rust
/// Macro for generating state machine transition tables
macro_rules! state_machine {
    (
        name: $name:ident,
        states: [$($state:ident),+ $(,)?],
        events: [$($event:ident),+ $(,)?],
        transitions: [
            $(($from:ident, $evt:ident) => $to:ident),+ $(,)?
        ]
    ) => {
        impl $name {
            pub fn valid_transitions() -> &'static [(&'static str, &'static str, &'static str)] {
                &[
                    $((stringify!($from), stringify!($evt), stringify!($to)),)+
                ]
            }

            pub fn can_transition(&self, event: &${name}Event) -> bool {
                match (self, event) {
                    $(
                        ($name::$from, ${name}Event::$evt) => true,
                    )+
                    _ => false,
                }
            }
        }
    };
}
```

### 6.2 State Machine Observer

```rust
/// Observer for state machine transitions
pub trait StateObserver<S, E> {
    /// Called before a transition
    fn on_before_transition(&mut self, from: &S, event: &E);

    /// Called after a successful transition
    fn on_after_transition(&mut self, from: &S, to: &S, event: &E);

    /// Called when a transition fails
    fn on_transition_error(&mut self, from: &S, event: &E, error: &TransitionError);
}

/// State machine with observation support
pub struct ObservableStateMachine<S, E, O: StateObserver<S, E>> {
    state: S,
    observer: O,
    _phantom: PhantomData<E>,
}

impl<S, E, O> ObservableStateMachine<S, E, O>
where
    S: Clone,
    O: StateObserver<S, E>,
{
    pub fn new(initial_state: S, observer: O) -> Self {
        Self {
            state: initial_state,
            observer,
            _phantom: PhantomData,
        }
    }

    pub fn apply<F>(&mut self, event: E, transition_fn: F) -> Result<(), TransitionError>
    where
        F: FnOnce(S, E) -> Result<S, TransitionError>,
    {
        self.observer.on_before_transition(&self.state, &event);

        let old_state = self.state.clone();
        match transition_fn(self.state.clone(), event.clone()) {
            Ok(new_state) => {
                self.state = new_state;
                self.observer.on_after_transition(&old_state, &self.state, &event);
                Ok(())
            }
            Err(error) => {
                self.observer.on_transition_error(&old_state, &event, &error);
                Err(error)
            }
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}
```

---

## 7. Testing State Machines

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_happy_path() {
        let mut state = SessionState::Initializing;

        // Initialize
        state = state.transition(SessionEvent::Initialized).unwrap();
        assert_eq!(state, SessionState::Active);

        // Start chat
        state = state.transition(SessionEvent::Chat {
            message: "Hello".into()
        }).unwrap();
        assert!(matches!(state, SessionState::Processing { .. }));

        // Complete
        state = state.transition(SessionEvent::AgentComplete {
            result: AgentResult::Success("Done".into())
        }).unwrap();
        assert_eq!(state, SessionState::Active);
    }

    #[test]
    fn test_invalid_transition() {
        let state = SessionState::Ended {
            reason: EndReason::Normal
        };

        // Cannot chat after ended
        let result = state.transition(SessionEvent::Chat {
            message: "Hello".into()
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_tool_execution_flow() {
        let mut state = AgentState::Initializing;

        state = state.transition(AgentEvent::Ready).unwrap();
        state = state.transition(AgentEvent::Execute {
            task: Task::new("test")
        }).unwrap();

        assert!(matches!(state, AgentState::Thinking { step: 1 }));

        // Tool calls
        state = state.transition(AgentEvent::ToolCalls {
            calls: vec![ToolCall::new("read", json!({}))]
        }).unwrap();

        assert!(matches!(state, AgentState::ExecutingTools { .. }));
    }
}
```
