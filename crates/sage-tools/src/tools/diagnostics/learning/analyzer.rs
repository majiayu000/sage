//! Pattern analysis and formatting utilities

use sage_core::learning::{Pattern, PatternType, SharedLearningEngine};

/// Parse pattern type string to PatternType enum
pub fn parse_pattern_type(type_str: &str) -> PatternType {
    match type_str.to_lowercase().as_str() {
        "correction" => PatternType::Correction,
        "preference" | "tool_preference" => PatternType::ToolPreference,
        "style" | "coding_style" => PatternType::CodingStyle,
        "workflow" | "workflow_preference" => PatternType::WorkflowPreference,
        _ => PatternType::Custom,
    }
}

/// Get all pattern types to iterate over
pub fn all_pattern_types() -> Vec<PatternType> {
    vec![
        PatternType::Correction,
        PatternType::ToolPreference,
        PatternType::CodingStyle,
        PatternType::WorkflowPreference,
        PatternType::Custom,
    ]
}

/// Get all patterns from the engine
pub async fn get_all_patterns(engine: &SharedLearningEngine) -> Vec<Pattern> {
    let mut all = Vec::new();
    for pt in all_pattern_types() {
        all.extend(engine.get_patterns_by_type(pt).await);
    }
    all
}

/// Search patterns by query string
pub async fn search_patterns(engine: &SharedLearningEngine, query: &str) -> Vec<Pattern> {
    let query_lower = query.to_lowercase();
    let mut matches = Vec::new();

    for pt in all_pattern_types() {
        for p in engine.get_patterns_by_type(pt).await {
            if p.description.to_lowercase().contains(&query_lower)
                || p.rule.to_lowercase().contains(&query_lower)
                || p.context
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query_lower))
            {
                matches.push(p);
            }
        }
    }

    matches
}

/// Format a single pattern for display
pub fn format_pattern(p: &Pattern, index: usize) -> String {
    format!(
        "{}. [{}] {}\n   Rule: {}\n   Confidence: {:.0}%, Context: {}\n   ID: {}\n\n",
        index,
        p.pattern_type.name(),
        p.description,
        p.rule,
        p.confidence.value() * 100.0,
        if p.context.is_empty() {
            "none".to_string()
        } else {
            p.context.join(", ")
        },
        p.id.as_str()
    )
}

/// Format a pattern for search results (shorter format)
pub fn format_pattern_search(p: &Pattern, index: usize) -> String {
    format!(
        "{}. [{}] {}\n   Rule: {}\n   ID: {}\n\n",
        index,
        p.pattern_type.name(),
        p.description,
        p.rule,
        p.id.as_str()
    )
}

/// Format a list of patterns
pub fn format_pattern_list(patterns: &[Pattern]) -> String {
    if patterns.is_empty() {
        return "No patterns found.".to_string();
    }

    let mut output = format!("Found {} patterns:\n\n", patterns.len());
    for (i, p) in patterns.iter().enumerate() {
        output.push_str(&format_pattern(p, i + 1));
    }
    output
}

/// Format search results
pub fn format_search_results(patterns: &[Pattern], query: &str) -> String {
    if patterns.is_empty() {
        return format!("No patterns found matching '{}'.", query);
    }

    let mut output = format!("Found {} patterns matching '{}':\n\n", patterns.len(), query);
    for (i, p) in patterns.iter().enumerate() {
        output.push_str(&format_pattern_search(p, i + 1));
    }
    output
}
