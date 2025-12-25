//! Pattern detection and analysis for learning

pub mod analyzer;
pub mod detector;
pub mod extractor;
pub mod matcher;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export public types
pub use analyzer::PatternDetector;
pub use detector::analyze_user_message;
pub use types::{CorrectionRecord, CorrectionStats, PreferenceIndicator, StylePattern};
