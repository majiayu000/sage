//! IPC module for Node.js ↔ Rust communication
//!
//! This module provides the infrastructure for the Modern UI architecture
//! where Node.js (Ink/React) acts as the main process for terminal UI,
//! and Rust runs as a subprocess handling agent execution.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐    stdin (JSON-Lines)    ┌─────────────────┐
//! │   Node.js/Ink   │ ──────────────────────► │   Rust IPC      │
//! │   (UI Main)     │                          │   (Agent Core)  │
//! │                 │ ◄────────────────────── │                 │
//! └─────────────────┘    stdout (JSON-Lines)   └─────────────────┘
//! ```
//!
//! ## Protocol
//!
//! - **Request**: Node.js → Rust (via stdin)
//!   - `chat`: Send a message to the agent
//!   - `cancel`: Cancel a running task
//!   - `get_config`: Get current configuration
//!   - `list_tools`: List available tools
//!   - `ping`: Health check
//!   - `shutdown`: Graceful shutdown
//!
//! - **Events**: Rust → Node.js (via stdout)
//!   - `ready`: Backend is ready
//!   - `tool_started/progress/completed`: Tool execution lifecycle
//!   - `llm_thinking/chunk/done`: LLM response lifecycle
//!   - `chat_completed`: Final response
//!   - `error`: Error occurred

pub mod protocol;
pub mod server;

pub use server::run_ipc_mode;
