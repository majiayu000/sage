//! User message analysis for preference detection

use super::super::types::{Confidence, PatternType};
use super::types::PreferenceIndicator;

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
