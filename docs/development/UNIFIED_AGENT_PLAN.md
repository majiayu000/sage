# Sage Agent Unified Architecture Plan

## Current Status Analysis

### Problem: Multiple Agent Implementations

Currently sage has **3 main agent implementations** that overlap in functionality:

| Agent | Location | Usage | Status |
|-------|----------|-------|--------|
| `UnifiedExecutor` | `sage-core/src/agent/unified/` | `sage unified` command | **Primary (should be the only core agent)** |
| `ClaudeStyleAgent` | `sage-core/src/agent/reactive_agent/` | Interactive mode via SDK | **Should be deprecated/merged** |
| `SubAgentRunner` | `sage-core/src/agent/subagent/` | Task tool sub-agents | **Correct - stays as specialized runner** |

### Current CLI Entry Points

```
sage                    -> route_default() -> interactive::execute() -> SageAgentSdk -> ClaudeStyleAgent
sage interactive        -> route_interactive() -> interactive::execute() -> SageAgentSdk -> ClaudeStyleAgent
sage run "task"         -> route_run() -> run::execute() -> SageAgentSdk -> ClaudeStyleAgent
sage unified "task"     -> route_unified() -> unified::execute() -> UnifiedExecutor (direct)
```

**Problem**: `interactive` and `run` use `SageAgentSdk` which internally uses `ClaudeStyleAgent`, while `unified` uses `UnifiedExecutor` directly. This causes:
1. Duplicate code paths
2. Inconsistent behavior between modes
3. Difficult maintenance

## Target Architecture (Claude Code Style)

### Single Core Agent: UnifiedExecutor

```
┌─────────────────────────────────────────────────────────────────────┐
│                         UnifiedExecutor                              │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Execution Modes                               │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────────┐ │ │
│  │  │  Interactive │ │  One-shot    │ │  Non-Interactive/Print   │ │ │
│  │  │  (default)   │ │  (run mode)  │ │  (--print/-p mode)       │ │ │
│  │  └──────────────┘ └──────────────┘ └──────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Core Components                               │ │
│  │  • LLM Client                                                   │ │
│  │  • Tool Executor (BatchToolExecutor)                            │ │
│  │  • Session Management (new/resume)                              │ │
│  │  • Input Channel (for user interaction)                         │ │
│  │  • Skill Registry                                               │ │
│  │  • Animation Manager                                            │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │                    Sub-Agent System                              │ │
│  │  • SubAgentRunner (for Task tool)                               │ │
│  │  • Agent Registry (general-purpose, Explore, Plan, etc.)        │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
```

### New CLI Structure

```bash
# Interactive mode (default) - multi-turn conversation
sage                              # Start interactive session
sage -c                           # Resume most recent session
sage -r <session_id>              # Resume specific session

# One-shot mode - execute and exit
sage -p "task description"        # Print mode (non-interactive)
sage run "task description"       # Alias for compatibility

# Other modes
sage --stream-json                # Stream JSON output (for SDK/API)
sage mcp                          # MCP server mode

# Utility commands (unchanged)
sage config show
sage trajectory list
sage tools
```

## Implementation Plan

### Phase 1: Merge ClaudeStyleAgent into UnifiedExecutor (Priority: High) - COMPLETED

**Goal**: Remove `ClaudeStyleAgent` and have `UnifiedExecutor` be the only core agent.

**Status**: Completed on 2024-01-08

**Analysis Result**: SageAgentSdk was already using UnifiedExecutor (not ClaudeStyleAgent). ClaudeStyleAgent was only referenced by:
- 1 example file (deleted)
- SageBuilder.build_claude_style_agent() method (removed)

**Changes Made**:
1. Deleted `sage-core/src/agent/reactive_agent/` directory (8 files, ~1000 lines)
2. Updated `sage-core/src/agent/mod.rs` - removed reactive_agent module and exports
3. Updated `sage-core/src/lib.rs` - removed ClaudeStyleAgent, ReactiveAgent, ReactiveExecutionManager, ReactiveResponse exports
4. Updated `sage-core/src/builder/build.rs` - removed build_claude_style_agent() method
5. Deleted `examples/simple_reactive_demo.rs`
6. Deleted `examples/interactive_demo.rs`

**Verification**: All tests pass, build successful

### Phase 2: Unify CLI Entry Points (Priority: High) - COMPLETED

**Goal**: All CLI modes use the same entry point with different `ExecutionOptions`.

**Status**: Completed on 2024-01-08

**Changes Made**:
1. Simplified `args.rs` with unified CLI structure (Claude Code style):
   - `sage` = Interactive mode (default)
   - `sage "task"` = Execute task interactively
   - `sage -p "task"` = Print mode (non-interactive)
   - `sage -c` = Resume recent session (placeholder)
   - `sage -r <id>` = Resume specific session (placeholder)
   - Legacy commands (`run`, `interactive`, `unified`) hidden but still work

2. Simplified `router.rs`:
   - All execution paths now go through `UnifiedExecutor`
   - Legacy commands route to unified executor with appropriate flags

3. Removed dead code:
   - Deleted `cli_mode.rs` (unused)
   - Deleted `session.rs`, `session_resume.rs` (dead code)
   - Deleted resume.rs files in interactive/ and run/

4. Dead code remaining (intentionally kept for now):
   - `commands/interactive/` - can be removed in future cleanup
   - `commands/run/` - can be removed in future cleanup

**New CLI Help Output**:
```
USAGE:
  sage                           # Start interactive mode
  sage "your task"               # Execute task (interactive)
  sage -p "your task"            # Print mode (non-interactive)
  sage -c                        # Resume most recent session
  sage -r <session_id>           # Resume specific session
```

#### Step 2.1: Simplify args.rs

```rust
#[derive(Parser)]
pub struct Cli {
    /// Task description (optional for interactive mode)
    task: Option<String>,

    /// Print mode (non-interactive, single response)
    #[arg(short = 'p', long)]
    print: bool,

    /// Resume most recent session
    #[arg(short = 'c', long = "continue")]
    resume_recent: bool,

    /// Resume specific session
    #[arg(short = 'r', long = "resume")]
    resume_session: Option<String>,

    /// Stream JSON output (for SDK/programmatic use)
    #[arg(long)]
    stream_json: bool,

    /// Non-interactive mode (auto-respond to questions)
    #[arg(long)]
    non_interactive: bool,

    // Common options
    #[arg(long, default_value = "sage_config.json")]
    config_file: String,

    #[arg(long)]
    max_steps: Option<u32>,

    #[arg(long)]
    working_dir: Option<PathBuf>,

    #[arg(long, short)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Alias for print mode (compatibility)
    Run { task: String },

    /// MCP server mode
    Mcp { /* options */ },

    /// Configuration management
    Config { #[command(subcommand)] action: ConfigAction },

    /// Trajectory management
    Trajectory { #[command(subcommand)] action: TrajectoryAction },

    /// List available tools
    Tools,
}
```

#### Step 2.2: Simplify router.rs

```rust
pub async fn route(cli: Cli) -> SageResult<()> {
    // Subcommands have priority
    if let Some(command) = cli.command {
        return match command {
            Commands::Run { task } => execute_main(cli_with_task(cli, task, true)).await,
            Commands::Mcp { .. } => mcp::run_mcp_mode().await,
            Commands::Config { action } => route_config(action).await,
            Commands::Trajectory { action } => route_trajectory(action).await,
            Commands::Tools => commands::tools::show_tools().await,
        };
    }

    // Main execution (unified entry point)
    execute_main(cli).await
}

async fn execute_main(cli: Cli) -> SageResult<()> {
    let mode = determine_execution_mode(&cli);
    let mut executor = create_executor(&cli)?;

    match mode {
        ExecutionMode::Interactive => run_interactive_loop(executor, cli).await,
        ExecutionMode::Print => run_print_mode(executor, cli).await,
        ExecutionMode::Resume(session_id) => resume_session(executor, session_id).await,
        ExecutionMode::StreamJson => run_stream_mode(executor, cli).await,
    }
}
```

### Phase 3: Implement Session Resume (Priority: Medium) - COMPLETED

**Goal**: Support `-c` and `-r` flags for session continuation.

**Status**: Completed on 2024-01-08

**Changes Made**:
1. Added `restore_session()` and `get_most_recent_session()` methods to `UnifiedExecutor` (`session.rs`)
2. Added `convert_messages_for_resume()` helper to convert EnhancedMessages to LlmMessages
3. Added `set_jsonl_storage()` method for external configuration
4. Updated `UnifiedArgs` with `resume_session_id` and `continue_recent` fields
5. Added `execute_session_resume()` function in `unified.rs` for session resume flow
6. Updated `router.rs` to pass session resume flags to unified executor

**Session Storage Structure**:
```
~/.sage/sessions/{session-id}/
├── messages.jsonl      # Conversation history
├── snapshots.jsonl     # File state snapshots
└── metadata.json       # Session metadata (title, timestamps, etc.)
```

**CLI Usage**:
```bash
sage -c                    # Resume most recent session
sage -r <session_id>       # Resume specific session by ID
```

### Phase 4: Implement Stream JSON Mode (Priority: Medium) - COMPLETED

**Goal**: Support `--stream-json` for SDK/programmatic integration.

**Status**: Completed on 2024-01-08

**Changes Made**:
1. Added `stream_json: bool` field to `UnifiedArgs`
2. Added `execute_stream_json()` function in `unified.rs`
3. Updated `router.rs` to pass `stream_json` flag from CLI
4. Integrated existing `OutputWriter` and `StreamJsonFormatter` infrastructure

**CLI Usage**:
```bash
sage --stream-json "your task"
```

**Output Format (Claude Code compatible JSONL)**:
```json
{"type":"system","message":"Sage Agent starting","timestamp":"..."}
{"type":"system","message":"Task: your task","timestamp":"..."}
{"type":"result","content":"Done","cost":{"input_tokens":100,"output_tokens":50,"total_tokens":150},"duration_ms":1234,"timestamp":"..."}
```

**Event Types**:
- `system`: System status messages
- `assistant`: Assistant responses
- `tool_call_start`: Tool execution started
- `tool_call_result`: Tool execution completed
- `error`: Error occurred
- `result`: Final result with cost info

### Phase 5: Implement MCP Protocol (Priority: Low)

**Goal**: Support `sage mcp` command for MCP server mode.

This allows sage to be used as an MCP server, providing tools to other clients.

## Files to Modify

### Phase 1: Agent Unification

1. `crates/sage-core/src/agent/unified/executor.rs` - Add missing features from ClaudeStyleAgent
2. `crates/sage-core/src/agent/unified/mod.rs` - Update exports
3. `crates/sage-sdk/src/lib.rs` - Switch from ClaudeStyleAgent to UnifiedExecutor
4. `crates/sage-core/src/agent/mod.rs` - Remove reactive_agent exports
5. Delete: `crates/sage-core/src/agent/reactive_agent/` directory

### Phase 2: CLI Unification

1. `crates/sage-cli/src/args.rs` - Simplify to single entry point model
2. `crates/sage-cli/src/router.rs` - Unified routing logic
3. `crates/sage-cli/src/commands/mod.rs` - Remove redundant modules
4. Delete: `crates/sage-cli/src/commands/interactive/` (merge into main)
5. Delete: `crates/sage-cli/src/commands/run/` (merge into main)

### Phase 3-5: New Features

1. `crates/sage-core/src/trajectory/session_resume.rs` - Session resume logic
2. `crates/sage-cli/src/stream_output.rs` - Stream JSON output
3. `crates/sage-cli/src/mcp/` - MCP server implementation

## Success Criteria

1. **Single Core Agent**: Only `UnifiedExecutor` exists as the core agent
2. **Unified CLI**: All modes use the same execution path with different options
3. **Feature Parity**: All existing features work with the new architecture
4. **Clean Codebase**: No duplicate code paths, no deprecated modules
5. **Tests Pass**: All existing tests pass with new architecture

## Migration Notes

- Keep backward compatibility for `sage run "task"` command (alias for `sage -p "task"`)
- The `unified` subcommand can be deprecated after migration
- Document the new CLI flags in help text
