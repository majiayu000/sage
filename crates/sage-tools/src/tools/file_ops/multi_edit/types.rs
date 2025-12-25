//! Type definitions for multi-edit operations

use serde::{Deserialize, Serialize};

/// A single edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditOperation {
    /// The text to replace
    pub old_string: String,
    /// The replacement text
    pub new_string: String,
    /// Whether to replace all occurrences (default: false)
    #[serde(default)]
    pub replace_all: bool,
}

/// Truncate a string for display purposes
pub fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_display() {
        assert_eq!(truncate_for_display("short", 10), "short");
        assert_eq!(
            truncate_for_display("this is a longer string", 10),
            "this is a ..."
        );
        assert_eq!(truncate_for_display("exact", 5), "exact");
    }
}
