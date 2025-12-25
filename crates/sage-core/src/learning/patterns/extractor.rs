//! Pattern extraction logic

use super::super::types::{Pattern, PatternSource, PatternType};
use super::types::{CorrectionRecord, StylePattern};
use std::collections::HashMap;

/// Extract tool preference patterns from usage data
pub fn extract_tool_patterns(
    tool_usage: &HashMap<String, u32>,
    min_observations: u32,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for (tool, count) in tool_usage {
        if *count >= min_observations {
            let pattern = Pattern::tool_preference(
                tool,
                &format!("Frequently used tool (used {} times)", count),
            )
            .with_confidence((*count as f32 / 100.0).min(0.9));
            patterns.push(pattern);
        }
    }

    patterns
}

/// Extract correction patterns from correction records
pub fn extract_correction_patterns(
    corrections: &[CorrectionRecord],
    min_observations: u32,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for correction in corrections {
        if correction.count >= min_observations.max(2) {
            let mut pattern = Pattern::correction(&correction.original, &correction.corrected)
                .with_confidence((correction.count as f32 / 10.0).min(0.9));

            for ctx in &correction.context {
                pattern = pattern.with_context(ctx.clone());
            }

            patterns.push(pattern);
        }
    }

    patterns
}

/// Extract style patterns from detected coding styles
pub fn extract_style_patterns(
    style_patterns: &[StylePattern],
    min_observations: u32,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for style in style_patterns {
        if style.samples >= min_observations && style.confidence >= 0.6 {
            let pattern =
                Pattern::coding_style(&style.aspect, &style.preference).with_confidence(style.confidence);
            patterns.push(pattern);
        }
    }

    patterns
}

/// Extract file type preference patterns
pub fn extract_file_type_patterns(
    file_preferences: &HashMap<String, HashMap<String, u32>>,
    min_observations: u32,
) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    for (file_type, tools) in file_preferences {
        if let Some((preferred_tool, count)) = tools.iter().max_by_key(|(_, c)| *c) {
            if *count >= min_observations {
                let pattern = Pattern::new(
                    PatternType::ToolPreference,
                    format!("Preferred tool for {} files", file_type),
                    format!("Use {} for {} files", preferred_tool, file_type),
                    PatternSource::ToolUsage,
                )
                .with_context(format!("file_type:{}", file_type))
                .with_confidence((*count as f32 / 20.0).min(0.8));
                patterns.push(pattern);
            }
        }
    }

    patterns
}
