//! Error types for Sage Agent
//!
//! This module provides a unified error handling system across all Sage Agent crates.
//! All errors implement the `UnifiedError` trait which provides consistent fields:
//! - error_code: A unique identifier for programmatic error handling
//! - message: Human-readable error message
//! - context: Optional additional context about where/why the error occurred
//! - source: Optional underlying error that caused this error

mod classifiers;
mod constructors;
mod conversions;
mod types;
mod unified_error;
mod user_messages;

// Re-export all public types and traits
pub use types::{OptionExt, ResultExt, SageError, SageResult, UnifiedError};
pub use user_messages::{ErrorCategory, UserFriendlyError};
