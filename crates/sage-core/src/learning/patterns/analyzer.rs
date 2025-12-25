//! Pattern detection and analysis for learning

use super::extractor::{
    extract_correction_patterns, extract_file_type_patterns, extract_style_patterns,
    extract_tool_patterns,
};
use super::matcher::{
    detect_brace_style, detect_indentation_style, detect_naming_convention, detect_quote_style,
    detect_semicolon_style,
};
use super::super::types::Pattern;
use super::types::{CorrectionRecord, CorrectionStats, StylePattern};
use std::collections::HashMap;

/// Pattern detector for analyzing interactions and extracting patterns
#[derive(Debug, Default)]
pub struct PatternDetector {
    /// Tool usage counts
    tool_usage: HashMap<String, u32>,
    /// Common corrections
    corrections: Vec<CorrectionRecord>,
    /// File type preferences
    file_preferences: HashMap<String, HashMap<String, u32>>,
    /// Detected coding style patterns
    pub(crate) style_patterns: Vec<StylePattern>,
}

impl PatternDetector {
    /// Create a new pattern detector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record tool usage
    pub fn record_tool_use(&mut self, tool_name: &str) {
        *self.tool_usage.entry(tool_name.to_string()).or_insert(0) += 1;
    }

    /// Record a user correction
    pub fn record_correction(&mut self, original: &str, corrected: &str, context: Vec<String>) {
        // Check if we've seen this correction before
        if let Some(record) = self.corrections.iter_mut().find(|c| {
            c.original.to_lowercase() == original.to_lowercase()
                && c.corrected.to_lowercase() == corrected.to_lowercase()
        }) {
            record.count += 1;
            return;
        }

        self.corrections.push(CorrectionRecord {
            original: original.to_string(),
            corrected: corrected.to_string(),
            context,
            count: 1,
        });
    }

    /// Record file type preference for a tool
    pub fn record_file_preference(&mut self, file_type: &str, preferred_tool: &str) {
        let tool_counts = self
            .file_preferences
            .entry(file_type.to_string())
            .or_default();
        *tool_counts.entry(preferred_tool.to_string()).or_insert(0) += 1;
    }

    /// Analyze code for style patterns
    pub fn analyze_code_style(&mut self, code: &str, file_type: &str) {
        // Detect indentation style
        if let Some(indent_pattern) = detect_indentation_style(code) {
            self.update_style_pattern("indentation", &indent_pattern, file_type);
        }

        // Detect quote style (for strings)
        if let Some(quote_pattern) = detect_quote_style(code) {
            self.update_style_pattern("quotes", &quote_pattern, file_type);
        }

        // Detect semicolon usage
        if let Some(semi_pattern) = detect_semicolon_style(code) {
            self.update_style_pattern("semicolons", &semi_pattern, file_type);
        }

        // Detect naming convention
        if let Some(naming_pattern) = detect_naming_convention(code) {
            self.update_style_pattern("naming", &naming_pattern, file_type);
        }

        // Detect brace style
        if let Some(brace_pattern) = detect_brace_style(code) {
            self.update_style_pattern("braces", &brace_pattern, file_type);
        }
    }

    fn update_style_pattern(&mut self, aspect: &str, preference: &str, _file_type: &str) {
        if let Some(pattern) = self
            .style_patterns
            .iter_mut()
            .find(|p| p.aspect == aspect && p.preference == preference)
        {
            pattern.samples += 1;
            pattern.confidence = (pattern.confidence + 0.1).min(1.0);
        } else {
            self.style_patterns.push(StylePattern {
                aspect: aspect.to_string(),
                preference: preference.to_string(),
                confidence: 0.5,
                samples: 1,
            });
        }
    }

    /// Extract patterns that are ready to be learned
    pub fn extract_patterns(&self, min_observations: u32) -> Vec<Pattern> {
        let mut patterns = Vec::new();

        // Extract tool preference patterns
        patterns.extend(extract_tool_patterns(&self.tool_usage, min_observations));

        // Extract correction patterns
        patterns.extend(extract_correction_patterns(&self.corrections, min_observations));

        // Extract style patterns
        patterns.extend(extract_style_patterns(&self.style_patterns, min_observations));

        // Extract file type preferences
        patterns.extend(extract_file_type_patterns(
            &self.file_preferences,
            min_observations,
        ));

        patterns
    }

    /// Get most used tools
    pub fn most_used_tools(&self, limit: usize) -> Vec<(String, u32)> {
        let mut tools: Vec<_> = self
            .tool_usage
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        tools.sort_by(|a, b| b.1.cmp(&a.1));
        tools.truncate(limit);
        tools
    }

    /// Get correction statistics
    pub fn correction_stats(&self) -> CorrectionStats {
        CorrectionStats {
            total_corrections: self.corrections.len(),
            repeated_corrections: self.corrections.iter().filter(|c| c.count > 1).count(),
            most_common: self
                .corrections
                .iter()
                .max_by_key(|c| c.count)
                .map(|c| (c.original.clone(), c.count)),
        }
    }

    /// Clear all collected data
    pub fn clear(&mut self) {
        self.tool_usage.clear();
        self.corrections.clear();
        self.file_preferences.clear();
        self.style_patterns.clear();
    }
}
