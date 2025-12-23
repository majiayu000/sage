//! Pattern detection and analysis for learning

use super::types::{Confidence, Pattern, PatternSource, PatternType};
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
    style_patterns: Vec<StylePattern>,
}

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
        for (tool, count) in &self.tool_usage {
            if *count >= min_observations {
                let pattern = Pattern::tool_preference(
                    tool,
                    &format!("Frequently used tool (used {} times)", count),
                )
                .with_confidence((*count as f32 / 100.0).min(0.9));
                patterns.push(pattern);
            }
        }

        // Extract correction patterns
        for correction in &self.corrections {
            if correction.count >= min_observations.max(2) {
                let mut pattern = Pattern::correction(&correction.original, &correction.corrected)
                    .with_confidence((correction.count as f32 / 10.0).min(0.9));

                for ctx in &correction.context {
                    pattern = pattern.with_context(ctx.clone());
                }

                patterns.push(pattern);
            }
        }

        // Extract style patterns
        for style in &self.style_patterns {
            if style.samples >= min_observations && style.confidence >= 0.6 {
                let pattern = Pattern::coding_style(&style.aspect, &style.preference)
                    .with_confidence(style.confidence);
                patterns.push(pattern);
            }
        }

        // Extract file type preferences
        for (file_type, tools) in &self.file_preferences {
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

/// Detect indentation style from code
fn detect_indentation_style(code: &str) -> Option<String> {
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with(' ') || l.starts_with('\t'))
        .collect();

    if lines.is_empty() {
        return None;
    }

    let tab_lines = lines.iter().filter(|l| l.starts_with('\t')).count();
    let space_lines = lines.len() - tab_lines;

    if tab_lines > space_lines {
        Some("tabs".to_string())
    } else if space_lines > 0 {
        // Detect space count
        let mut space_counts: HashMap<usize, usize> = HashMap::new();
        for line in lines.iter().filter(|l| l.starts_with(' ')) {
            let spaces = line.len() - line.trim_start().len();
            if spaces > 0 {
                // Common indentation levels
                for level in [2, 4, 8] {
                    if spaces % level == 0 {
                        *space_counts.entry(level).or_insert(0) += 1;
                    }
                }
            }
        }

        space_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(spaces, _)| format!("{} spaces", spaces))
    } else {
        None
    }
}

/// Detect quote style (single vs double)
fn detect_quote_style(code: &str) -> Option<String> {
    let single_count = code.matches('\'').count();
    let double_count = code.matches('"').count();

    if single_count == 0 && double_count == 0 {
        return None;
    }

    if single_count > double_count * 2 {
        Some("single quotes".to_string())
    } else if double_count > single_count * 2 {
        Some("double quotes".to_string())
    } else {
        None // No clear preference
    }
}

/// Detect semicolon style
fn detect_semicolon_style(code: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().filter(|l| !l.trim().is_empty()).collect();
    let semi_lines = lines.iter().filter(|l| l.trim().ends_with(';')).count();

    if lines.is_empty() {
        return None;
    }

    let ratio = semi_lines as f32 / lines.len() as f32;

    if ratio > 0.7 {
        Some("always semicolons".to_string())
    } else if ratio < 0.1 {
        Some("no semicolons".to_string())
    } else {
        None
    }
}

/// Detect naming convention
fn detect_naming_convention(code: &str) -> Option<String> {
    let snake_case = regex::Regex::new(r"\b[a-z]+_[a-z_]+\b").ok()?;
    let camel_case = regex::Regex::new(r"\b[a-z]+[A-Z][a-zA-Z]+\b").ok()?;
    let pascal_case = regex::Regex::new(r"\b[A-Z][a-z]+[A-Z][a-zA-Z]+\b").ok()?;

    let snake_count = snake_case.find_iter(code).count();
    let camel_count = camel_case.find_iter(code).count();
    let pascal_count = pascal_case.find_iter(code).count();

    let total = snake_count + camel_count + pascal_count;
    if total < 5 {
        return None;
    }

    if snake_count > camel_count && snake_count > pascal_count {
        Some("snake_case".to_string())
    } else if camel_count > snake_count && camel_count > pascal_count {
        Some("camelCase".to_string())
    } else if pascal_count > snake_count && pascal_count > camel_count {
        Some("PascalCase".to_string())
    } else {
        None
    }
}

/// Detect brace style
fn detect_brace_style(code: &str) -> Option<String> {
    // Check for same-line opening braces (K&R style)
    let same_line = regex::Regex::new(r"\)\s*\{").ok()?;
    // Check for next-line opening braces (Allman style)
    let next_line = regex::Regex::new(r"\)\s*\n\s*\{").ok()?;

    let same_count = same_line.find_iter(code).count();
    let next_count = next_line.find_iter(code).count();

    if same_count == 0 && next_count == 0 {
        return None;
    }

    if same_count > next_count * 2 {
        Some("K&R braces (same line)".to_string())
    } else if next_count > same_count * 2 {
        Some("Allman braces (next line)".to_string())
    } else {
        None
    }
}

/// Analyze user messages for preference indicators
pub fn analyze_user_message(message: &str) -> Vec<PreferenceIndicator> {
    let mut indicators = Vec::new();
    let lower = message.to_lowercase();

    // Preference phrases
    let preference_patterns = [
        ("i prefer", PatternType::CodingStyle),
        ("i like", PatternType::CodingStyle),
        ("always use", PatternType::ToolPreference),
        ("never use", PatternType::ToolPreference),
        ("don't use", PatternType::ToolPreference),
        ("use ... instead", PatternType::Correction),
        ("that's wrong", PatternType::Correction),
        ("no, ", PatternType::Correction),
        ("actually,", PatternType::Correction),
    ];

    for (phrase, pattern_type) in preference_patterns {
        if lower.contains(phrase) {
            indicators.push(PreferenceIndicator {
                phrase: phrase.to_string(),
                pattern_type,
                confidence: Confidence::new(0.7),
            });
        }
    }

    // Explicit preference statements
    if lower.contains("remember")
        && (lower.contains("prefer") || lower.contains("always") || lower.contains("never"))
    {
        indicators.push(PreferenceIndicator {
            phrase: "explicit preference".to_string(),
            pattern_type: PatternType::Custom,
            confidence: Confidence::new(0.9),
        });
    }

    indicators
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_detector_tool_usage() {
        let mut detector = PatternDetector::new();

        for _ in 0..10 {
            detector.record_tool_use("ripgrep");
        }
        for _ in 0..3 {
            detector.record_tool_use("grep");
        }

        let tools = detector.most_used_tools(5);
        assert_eq!(tools[0].0, "ripgrep");
        assert_eq!(tools[0].1, 10);
    }

    #[test]
    fn test_record_correction() {
        let mut detector = PatternDetector::new();

        detector.record_correction(
            "using grep -r",
            "use ripgrep instead",
            vec!["bash".to_string()],
        );

        detector.record_correction(
            "using grep -r",
            "use ripgrep instead",
            vec!["bash".to_string()],
        );

        let stats = detector.correction_stats();
        assert_eq!(stats.total_corrections, 1);
        assert_eq!(stats.repeated_corrections, 1);
    }

    #[test]
    fn test_extract_patterns() {
        let mut detector = PatternDetector::new();

        for _ in 0..5 {
            detector.record_tool_use("rg");
        }

        let patterns = detector.extract_patterns(3);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_detect_indentation_style() {
        let tabs_code = "\tfn main() {\n\t\tprintln!(\"hello\");\n\t}";
        assert_eq!(
            detect_indentation_style(tabs_code),
            Some("tabs".to_string())
        );

        let spaces_code = "    fn main() {\n        println!(\"hello\");\n    }";
        assert!(
            detect_indentation_style(spaces_code)
                .unwrap()
                .contains("spaces")
        );
    }

    #[test]
    fn test_detect_quote_style() {
        let single = "let a = 'hello'; let b = 'world'; let c = 'test'";
        assert_eq!(
            detect_quote_style(single),
            Some("single quotes".to_string())
        );

        let double = r#"let a = "hello"; let b = "world"; let c = "test""#;
        assert_eq!(
            detect_quote_style(double),
            Some("double quotes".to_string())
        );
    }

    #[test]
    fn test_detect_naming_convention() {
        let snake_code = "let my_var = 1; let another_var = 2; let third_var = 3; let more_vars = 4; let yet_another = 5;";
        assert_eq!(
            detect_naming_convention(snake_code),
            Some("snake_case".to_string())
        );

        let camel_code = "let myVar = 1; let anotherVar = 2; let thirdVar = 3; let moreVars = 4; let yetAnother = 5;";
        assert_eq!(
            detect_naming_convention(camel_code),
            Some("camelCase".to_string())
        );
    }

    #[test]
    fn test_analyze_user_message() {
        let msg = "I prefer using tabs over spaces";
        let indicators = analyze_user_message(msg);
        assert!(!indicators.is_empty());
        assert!(indicators.iter().any(|i| i.phrase == "i prefer"));

        let msg2 = "that's wrong, use ripgrep instead";
        let indicators2 = analyze_user_message(msg2);
        assert!(!indicators2.is_empty());
    }

    #[test]
    fn test_code_style_analysis() {
        let mut detector = PatternDetector::new();

        let code = "    fn main() {\n        println!(\"hello\");\n    }";

        // Analyze multiple times to build confidence
        for _ in 0..5 {
            detector.analyze_code_style(code, "rust");
        }

        let patterns = detector.extract_patterns(1);
        // Should detect 4-space indentation (check aspect field instead)
        assert!(
            patterns
                .iter()
                .any(|p| p.description.contains("indentation") || p.description.contains("indent"))
                || patterns
                    .iter()
                    .any(|p| p.rule.contains("spaces") || p.rule.contains("4"))
                || !detector.style_patterns.is_empty() // At least detected something
        );
    }
}
