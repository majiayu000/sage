//! Sage Agent SDK
//!
//! This crate provides a high-level SDK for using Sage Agent programmatically.
//!
//! # API Versioning
//!
//! The SDK reports semantic API versions through the [`version`] module.
//! While the SDK remains `0.x`, breaking public API changes can ship in minor
//! releases and are recorded in the changelog instead of preserved behind
//! compatibility aliases.
//!
//! Current API version: **0.1.0**
//!
//! ## Version Compatibility
//!
//! Clients can check compatibility using [`version::is_compatible`] or
//! [`version::negotiate_version`].
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_sdk::{SageAgentSdk, version};
//!
//! // Check SDK version
//! println!("SDK Version: {}", version::version_string());
//!
//! // Verify client compatibility
//! let client_version = version::Version::new(0, 1, 0);
//! assert!(version::is_compatible(&client_version));
//! ```

// Allow common clippy lints that are stylistic preferences
#![allow(clippy::collapsible_if)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::manual_range_patterns)]

pub mod client;
pub mod version;

pub use client::{
    ExecutionConfigSummary, ExecutionError, ExecutionErrorKind, ExecutionOutcome, ExecutionResult,
    RunOptions, SageAgentSdk,
};

// Re-export commonly used types from core
pub use sage_core::{
    agent::{AgentExecution, AgentState, AgentStep},
    config::{Config, ModelParameters},
    error::{SageError, SageResult},
    types::{TaskMetadata, TokenUsage},
};

// Re-export version constants for convenience
pub use version::{API_VERSION, MIN_SUPPORTED_VERSION};
