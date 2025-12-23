//! Thoroughness level for exploration tasks

use serde::{Deserialize, Serialize};
use std::fmt;

/// Thoroughness level for exploration tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Thoroughness {
    /// Basic search - fast but may miss edge cases
    Quick,
    /// Balanced search - good coverage with reasonable speed
    #[default]
    Medium,
    /// Comprehensive analysis - thorough but slower
    VeryThorough,
}

impl fmt::Display for Thoroughness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Thoroughness {
    /// Get the string identifier for this thoroughness level
    pub fn as_str(&self) -> &str {
        match self {
            Thoroughness::Quick => "quick",
            Thoroughness::Medium => "medium",
            Thoroughness::VeryThorough => "very_thorough",
        }
    }

    /// Get suggested max steps for this thoroughness level
    pub fn suggested_max_steps(&self) -> usize {
        match self {
            Thoroughness::Quick => 5,
            Thoroughness::Medium => 15,
            Thoroughness::VeryThorough => 30,
        }
    }

    /// Get description for prompting
    pub fn description(&self) -> &str {
        match self {
            Thoroughness::Quick => {
                "Perform a quick search. Focus on the most obvious locations and patterns. Stop early if you find good matches."
            }
            Thoroughness::Medium => {
                "Perform a moderate search. Check multiple locations and naming conventions. Balance thoroughness with speed."
            }
            Thoroughness::VeryThorough => {
                "Perform a comprehensive search. Check all possible locations, naming patterns, and variations. Be thorough even if it takes longer."
            }
        }
    }
}
