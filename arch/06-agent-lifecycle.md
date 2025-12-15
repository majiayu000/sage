# Agent Lifecycle

## Lifecycle Phases

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent Lifecycle                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────┐                                                │
│  │  Init   │  on_init()                                     │
│  └────┬────┘                                                │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────┐                                                │
│  │  Task   │  on_task_start(task)                           │
│  │  Start  │                                                │
│  └────┬────┘                                                │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────────────────────────────────┐                    │
│  │         Execution Loop               │                    │
│  │  ┌─────────┐      ┌─────────────┐   │                    │
│  │  │  Step   │      │    Step     │   │                    │
│  │  │  Start  │─────▶│   Complete  │   │                    │
│  │  │         │      │             │   │                    │
│  │  │on_step_ │      │ on_step_    │   │                    │
│  │  │start()  │      │ complete()  │   │                    │
│  │  └─────────┘      └──────┬──────┘   │                    │
│  │                          │          │                    │
│  │        ◀─────────────────┘          │                    │
│  │        (repeat until complete)      │                    │
│  └─────────────────────────────────────┘                    │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────┐                                                │
│  │  Task   │  on_task_complete(task, success, result)       │
│  │Complete │                                                │
│  └────┬────┘                                                │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────┐                                                │
│  │Shutdown │  on_shutdown()                                 │
│  └─────────┘                                                │
│                                                              │
│  ┌─────────────────────────────────────┐                    │
│  │     Cross-cutting Events            │                    │
│  │  - on_state_change(from, to)        │                    │
│  │  - on_error(error)                  │                    │
│  └─────────────────────────────────────┘                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Lifecycle Phase Enum

```rust
pub enum LifecyclePhase {
    Init,            // Agent initialization
    TaskStart,       // Before task execution
    StepStart,       // Before each step
    StepComplete,    // After each step
    TaskComplete,    // After task execution
    Shutdown,        // Agent shutdown
    StateTransition, // State change events
    Error,           // Error events
}
```

## AgentLifecycle Trait

```rust
#[async_trait]
pub trait AgentLifecycle: Send + Sync {
    /// Called when the agent is initialized
    async fn on_init(&mut self) -> LifecycleResult<()> {
        Ok(())
    }

    /// Called before task execution starts
    async fn on_task_start(&mut self, task: &TaskMetadata) -> LifecycleResult<()> {
        let _ = task;
        Ok(())
    }

    /// Called before each step execution
    async fn on_step_start(&mut self, step_number: u32) -> LifecycleResult<()> {
        let _ = step_number;
        Ok(())
    }

    /// Called after each step completes
    async fn on_step_complete(&mut self, step: &AgentStep) -> LifecycleResult<()> {
        let _ = step;
        Ok(())
    }

    /// Called after task execution completes
    async fn on_task_complete(
        &mut self,
        task: &TaskMetadata,
        success: bool,
        result: Option<&str>,
    ) -> LifecycleResult<()> {
        let _ = (task, success, result);
        Ok(())
    }

    /// Called when the agent is shut down
    async fn on_shutdown(&mut self) -> LifecycleResult<()> {
        Ok(())
    }

    /// Called on state transitions
    async fn on_state_change(&mut self, from: AgentState, to: AgentState) -> LifecycleResult<()> {
        let _ = (from, to);
        Ok(())
    }

    /// Called when an error occurs
    async fn on_error(&mut self, error: &SageError) -> LifecycleResult<()> {
        let _ = error;
        Ok(())
    }
}
```

## Lifecycle Hook Trait

```rust
#[async_trait]
pub trait LifecycleHook: Send + Sync {
    /// Name of the hook for logging
    fn name(&self) -> &str;

    /// Phases this hook should run for
    fn phases(&self) -> Vec<LifecyclePhase>;

    /// Priority (higher runs first)
    fn priority(&self) -> i32 {
        0
    }

    /// Execute the hook
    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult>;
}

pub enum HookResult {
    Continue,                        // Continue normal execution
    Skip,                            // Skip remaining hooks for this phase
    Abort(String),                   // Abort the operation
    ModifyContext(Box<LifecycleContext>),  // Modify context and continue
}
```

## Lifecycle Context

```rust
pub struct LifecycleContext {
    pub phase: LifecyclePhase,
    pub agent_id: Option<Id>,
    pub state: AgentState,
    pub previous_state: Option<AgentState>,
    pub task: Option<TaskMetadata>,
    pub step_number: Option<u32>,
    pub step: Option<AgentStep>,
    pub execution: Option<AgentExecution>,
    pub error: Option<String>,
    pub metadata: HashMap<String, Value>,
}

impl LifecycleContext {
    pub fn new(phase: LifecyclePhase, state: AgentState) -> Self;
    pub fn with_agent_id(self, id: Id) -> Self;
    pub fn with_task(self, task: TaskMetadata) -> Self;
    pub fn with_step_number(self, step: u32) -> Self;
    pub fn with_step(self, step: AgentStep) -> Self;
    pub fn with_execution(self, execution: AgentExecution) -> Self;
    pub fn with_previous_state(self, state: AgentState) -> Self;
    pub fn with_error(self, error: impl Into<String>) -> Self;
    pub fn with_metadata(self, key: impl Into<String>, value: Value) -> Self;
}
```

## Hook Registry

```rust
pub struct LifecycleHookRegistry {
    hooks: RwLock<Vec<Arc<dyn LifecycleHook>>>,
}

impl LifecycleHookRegistry {
    pub fn new() -> Self;

    /// Register a hook (sorted by priority, higher first)
    pub async fn register(&self, hook: Arc<dyn LifecycleHook>);

    /// Unregister a hook by name
    pub async fn unregister(&self, name: &str);

    /// Execute all hooks for a phase
    pub async fn execute_hooks(
        &self,
        phase: LifecyclePhase,
        context: LifecycleContext,
    ) -> LifecycleResult<LifecycleContext>;

    pub async fn hooks(&self) -> Vec<Arc<dyn LifecycleHook>>;
    pub async fn count(&self) -> usize;
}
```

## Lifecycle Manager

```rust
pub struct LifecycleManager {
    registry: Arc<LifecycleHookRegistry>,
    state: RwLock<AgentState>,
    initialized: RwLock<bool>,
}

impl LifecycleManager {
    pub fn new() -> Self;
    pub fn with_registry(registry: Arc<LifecycleHookRegistry>) -> Self;

    pub fn registry(&self) -> Arc<LifecycleHookRegistry>;
    pub async fn state(&self) -> AgentState;
    pub async fn is_initialized(&self) -> bool;

    // Lifecycle notifications
    pub async fn initialize(&self, agent_id: Id) -> LifecycleResult<()>;
    pub async fn notify_task_start(&self, agent_id: Id, task: &TaskMetadata) -> LifecycleResult<()>;
    pub async fn notify_step_start(&self, agent_id: Id, step_number: u32) -> LifecycleResult<()>;
    pub async fn notify_step_complete(&self, agent_id: Id, step: &AgentStep) -> LifecycleResult<()>;
    pub async fn notify_task_complete(&self, agent_id: Id, execution: &AgentExecution) -> LifecycleResult<()>;
    pub async fn notify_state_change(&self, agent_id: Id, from: AgentState, to: AgentState) -> LifecycleResult<()>;
    pub async fn notify_error(&self, agent_id: Id, error: &SageError) -> LifecycleResult<()>;
    pub async fn shutdown(&self, agent_id: Id) -> LifecycleResult<()>;
}
```

## Built-in Hooks

### LoggingHook

```rust
pub struct LoggingHook {
    name: String,
    phases: Vec<LifecyclePhase>,
}

impl LoggingHook {
    pub fn all_phases() -> Self;
    pub fn for_phases(phases: Vec<LifecyclePhase>) -> Self;
}

#[async_trait]
impl LifecycleHook for LoggingHook {
    fn name(&self) -> &str { &self.name }
    fn phases(&self) -> Vec<LifecyclePhase> { self.phases.clone() }

    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult> {
        tracing::debug!(
            phase = %context.phase,
            state = %context.state,
            agent_id = ?context.agent_id,
            step_number = ?context.step_number,
            "Lifecycle hook triggered"
        );
        Ok(HookResult::Continue)
    }
}
```

### MetricsHook

```rust
pub struct MetricsHook {
    name: String,
}

impl MetricsHook {
    pub fn new() -> Self;
}

#[async_trait]
impl LifecycleHook for MetricsHook {
    fn name(&self) -> &str { &self.name }

    fn phases(&self) -> Vec<LifecyclePhase> {
        vec![
            LifecyclePhase::TaskStart,
            LifecyclePhase::StepComplete,
            LifecyclePhase::TaskComplete,
            LifecyclePhase::Error,
        ]
    }

    fn priority(&self) -> i32 { -100 }  // Run after other hooks

    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult> {
        // Collect and report metrics
        Ok(HookResult::Continue)
    }
}
```

## Usage Example

```rust
// Custom hook
struct ValidationHook;

#[async_trait]
impl LifecycleHook for ValidationHook {
    fn name(&self) -> &str { "validation" }

    fn phases(&self) -> Vec<LifecyclePhase> {
        vec![LifecyclePhase::TaskStart]
    }

    fn priority(&self) -> i32 { 100 }  // Run early

    async fn execute(&self, context: &LifecycleContext) -> LifecycleResult<HookResult> {
        if let Some(task) = &context.task {
            if task.description.is_empty() {
                return Ok(HookResult::Abort("Task description required".into()));
            }
        }
        Ok(HookResult::Continue)
    }
}

// Register hooks
let components = SageBuilder::new()
    .with_anthropic("key")
    .with_hook(Arc::new(LoggingHook::all_phases()))
    .with_hook(Arc::new(MetricsHook::new()))
    .with_hook(Arc::new(ValidationHook))
    .build()
    .await?;
```

## State Machine

```
┌─────────────────────────────────────────────────────────────┐
│                    Agent State Machine                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│        ┌──────────────────┐                                 │
│        │   Initializing   │                                 │
│        └────────┬─────────┘                                 │
│                 │                                            │
│        ┌────────▼─────────┐                                 │
│        │     Thinking     │◀──────────────┐                 │
│        └────────┬─────────┘               │                 │
│                 │                          │                 │
│        ┌────────▼─────────┐               │                 │
│        │  ToolExecution   │───────────────┤                 │
│        └────────┬─────────┘               │                 │
│                 │                          │                 │
│        ┌────────▼─────────┐               │                 │
│        │WaitingForTools   │───────────────┘                 │
│        └────────┬─────────┘                                 │
│                 │                                            │
│    ┌────────────┼────────────┬─────────────┐               │
│    │            │            │             │                │
│    ▼            ▼            ▼             ▼                │
│ ┌──────┐  ┌──────────┐  ┌─────────┐  ┌─────────┐          │
│ │Compl-│  │  Error   │  │Cancelled│  │ Timeout │          │
│ │eted  │  │          │  │         │  │         │          │
│ └──────┘  └──────────┘  └─────────┘  └─────────┘          │
│                                                              │
│  Terminal States (no transitions out)                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```
