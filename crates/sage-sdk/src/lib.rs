//! Sage Agent SDK
//!
//! This crate provides a high-level SDK for using Sage Agent programmatically.

pub mod client;

pub use client::{
    ExecutionError, ExecutionErrorKind, ExecutionOutcome, ExecutionResult, RunOptions, SageAgentSDK,
};

// Re-export commonly used types from core
pub use sage_core::{
    agent::{AgentExecution, AgentState, AgentStep},
    config::{Config, ModelParameters},
    error::{SageError, SageResult},
    types::{LLMUsage, TaskMetadata},
};
