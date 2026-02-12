//! Sage Agent SDK
//!
//! This crate provides a high-level SDK for using Sage Agent programmatically.
//!
//! # API Versioning
//!
//! The SDK follows semantic versioning (SemVer 2.0.0) for its public API.
//! Version information and compatibility checks are available through the
//! [`version`] module.
//!
//! Current API version: **0.1.0**
//!
//! ## Version Compatibility
//!
//! The SDK maintains backward compatibility within the same MAJOR version.
//! Clients can check compatibility using [`version::is_compatible`] or
//! [`version::negotiate_version`].
//!
//! ## Deprecation Policy
//!
//! - Deprecated APIs are marked with `#[deprecated]` attributes
//! - Deprecated APIs are maintained for at least one MINOR version
//! - Migration paths are provided in documentation
//! - Removed in next MAJOR version
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
    ExecutionError, ExecutionErrorKind, ExecutionOutcome, ExecutionResult, RunOptions,
    SageAgentSdk, UnifiedRunOptions,
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
