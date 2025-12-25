//! Data structures for the AskUserQuestion tool

use serde::{Deserialize, Serialize};

/// Represents a single option in a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display text for the option
    pub label: String,
    /// Explanation of what this option means
    pub description: String,
}

/// Represents a single question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// The question text to ask
    pub question: String,
    /// Short label for the question (max 12 chars) like "Auth method", "Library"
    pub header: String,
    /// List of options to choose from (2-4 options)
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default)]
    pub multi_select: bool,
}
