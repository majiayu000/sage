//! Enhanced error handling for Sage Tools
//!
//! This module provides enhanced error types with additional context, suggestions,
//! and categorization for better error handling and user experience.

mod context;
mod formatters;
pub mod helpers;
mod types;

#[cfg(test)]
mod tests;

// Re-export public types and functions
pub use sage_core::error::ErrorCategory;
pub use types::EnhancedToolError;
