//! Data types for pattern detection and analysis

use super::super::types::{Confidence, PatternType};

/// Record of a correction made
#[derive(Debug, Clone)]
pub struct CorrectionRecord {
    /// What was wrong
    pub original: String,
    /// What was corrected to
    pub corrected: String,
    /// Context (tool, file type, etc.)
    pub context: Vec<String>,
    /// How many times this correction was made
    pub count: u32,
}

/// Detected coding style pattern
#[derive(Debug, Clone)]
pub struct StylePattern {
    /// Aspect of coding style
    pub aspect: String,
    /// Detected preference
    pub preference: String,
    /// Confidence
    pub confidence: f32,
    /// Sample count
    pub samples: u32,
}

/// Correction statistics
#[derive(Debug, Clone)]
pub struct CorrectionStats {
    /// Total number of unique corrections
    pub total_corrections: usize,
    /// Number of repeated corrections
    pub repeated_corrections: usize,
    /// Most common correction
    pub most_common: Option<(String, u32)>,
}

/// Indicator of a preference in user message
#[derive(Debug, Clone)]
pub struct PreferenceIndicator {
    /// The phrase that indicated preference
    pub phrase: String,
    /// Type of pattern this might be
    pub pattern_type: PatternType,
    /// Confidence in this indicator
    pub confidence: Confidence,
}
