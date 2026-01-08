//! Violation tracking module following Claude Code patterns.
//!
//! This module provides:
//! - Violation type definitions
//! - Thread-safe violation storage
//! - Stderr annotation for violation reporting

mod annotator;
mod store;
mod types;

pub use annotator::{annotate_stderr, format_violations_xml};
pub use store::{SharedViolationStore, ViolationStore};
pub use types::{Violation, ViolationSeverity, ViolationType};
