//! Violation tracking module following Claude Code patterns.
//!
//! This module provides:
//! - Violation type definitions
//! - Thread-safe violation storage
//! - Stderr annotation for violation reporting

mod types;
mod store;
mod annotator;

pub use types::{Violation, ViolationType, ViolationSeverity};
pub use store::{ViolationStore, SharedViolationStore};
pub use annotator::{annotate_stderr, format_violations_xml};
