//! Sage Agent Self-Evaluation System
//!
//! This crate provides a comprehensive evaluation framework for testing
//! and benchmarking Sage Agent's capabilities across various tasks.
//!
//! # Features
//!
//! - **Built-in Test Suite**: Pre-defined evaluation tasks for code generation,
//!   editing, bug fixing, refactoring, and multi-file operations
//! - **Metrics Collection**: Pass@K rates, token efficiency, turn metrics
//! - **Sandbox Execution**: Isolated environment for safe task execution
//! - **Report Generation**: JSON, Markdown, and HTML output formats
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_eval::{EvalExecutor, TaskLoader, EvalConfig};
//!
//! let config = EvalConfig::default();
//! let executor = EvalExecutor::new(config);
//! let tasks = TaskLoader::load_builtin()?;
//! let results = executor.run_all(tasks).await?;
//! ```

pub mod metrics;
pub mod replay;
pub mod report;
pub mod runner;
pub mod tasks;

// Re-exports for convenience
pub use metrics::{EvalMetrics, PassAtK, TaskResult, TokenEfficiency, TurnMetrics};
pub use runner::{EvalConfig, EvalExecutor, Sandbox};
pub use tasks::{Difficulty, EvalTask, TaskCategory, TaskLoader, Verifier};
