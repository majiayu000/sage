//! Agent operational modes
//!
//! This module provides different operational modes for the agent:
//!
//! - **Normal**: All tools available, full functionality
//! - **Plan**: Read-only mode for exploration and planning
//! - **Review**: Read-only mode focused on code review
//! - **Debug**: Full tools with debugging focus
//!
//! # Plan Mode
//!
//! Plan mode restricts the agent to read-only tools, enabling safe exploration
//! of the codebase without risk of accidental modifications.
//!
//! ## Allowed Tools in Plan Mode
//! - Read, Glob, Grep - File exploration
//! - WebFetch, WebSearch - Research
//! - AskUserQuestion - Clarification
//!
//! ## Blocked Tools in Plan Mode
//! - Write, Edit - File modifications
//! - Bash - Command execution
//! - NotebookEdit - Notebook modifications
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::modes::{ModeManager, AgentMode};
//!
//! let manager = ModeManager::new();
//!
//! // Enter plan mode
//! let context = manager.enter_plan_mode(Some("architecture-review")).await?;
//! println!("Plan file: {:?}", context.plan_file);
//!
//! // Check tool permissions
//! assert!(manager.is_tool_allowed("Read").await);
//! assert!(!manager.is_tool_allowed("Write").await);
//!
//! // Save plan content
//! manager.save_plan("# Architecture Plan\n\n1. Refactor auth").await?;
//!
//! // Exit plan mode (requires approval)
//! let result = manager.exit_plan_mode(true).await?;
//! println!("Blocked {} tool calls", result.blocked_tool_calls);
//! ```

pub mod manager;
pub mod types;

pub use manager::{ModeExitResult, ModeManager, PlanModeContext};
pub use types::{AgentMode, ModeState, ModeTransition, PlanModeConfig, ToolFilter};
