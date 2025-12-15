//! Sage Agent SDK
//!
//! This crate provides a high-level SDK for using Sage Agent programmatically.

pub mod client;

pub use client::{SageAgentSDK, RunOptions, ExecutionResult, ExecutionOutcome, ExecutionError, ExecutionErrorKind};

// Re-export commonly used types from core
pub use sage_core::{
    agent::{AgentExecution, AgentStep, AgentState},
    config::{Config, ModelParameters},
    error::{SageError, SageResult},
    types::{TaskMetadata, LLMUsage},
};
