//! Learning event tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::base::PatternId;

/// Learning event for tracking what was learned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Type of learning event
    pub event_type: LearningEventType,
    /// Pattern that was affected
    pub pattern_id: Option<PatternId>,
    /// Description of what happened
    pub description: String,
    /// Associated data
    pub data: HashMap<String, String>,
}

/// Type of learning event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LearningEventType {
    /// New pattern discovered
    PatternDiscovered,
    /// Existing pattern reinforced
    PatternReinforced,
    /// Pattern was contradicted
    PatternContradicted,
    /// Pattern invalidated
    PatternInvalidated,
    /// User explicitly taught something
    UserTeaching,
    /// Pattern applied successfully
    PatternApplied,
}

impl LearningEvent {
    /// Create a new learning event
    pub fn new(event_type: LearningEventType, description: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            pattern_id: None,
            description: description.into(),
            data: HashMap::new(),
        }
    }

    /// Associate with a pattern
    pub fn with_pattern(mut self, pattern_id: PatternId) -> Self {
        self.pattern_id = Some(pattern_id);
        self
    }

    /// Add data
    pub fn with_data(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_event() {
        let event = LearningEvent::new(
            LearningEventType::PatternDiscovered,
            "Learned new coding style preference",
        )
        .with_pattern(PatternId::from_string("pattern-1"))
        .with_data("file_type", "rust");

        assert_eq!(event.event_type, LearningEventType::PatternDiscovered);
        assert!(event.pattern_id.is_some());
        assert_eq!(event.data.get("file_type"), Some(&"rust".to_string()));
    }
}
