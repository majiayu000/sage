//! Core input types for structured questions and options

use serde::{Deserialize, Serialize};

// ============================================================================
// Question Types (for AskUserQuestion tool)
// ============================================================================

/// A structured question with options (Claude Code style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// The question text to ask
    pub question: String,
    /// Short label/header (max 12 chars) like "Auth method", "Library"
    pub header: String,
    /// List of options to choose from (2-4 options)
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default)]
    pub multi_select: bool,
}

impl Question {
    /// Create a new question
    pub fn new(
        question: impl Into<String>,
        header: impl Into<String>,
        options: Vec<QuestionOption>,
    ) -> Self {
        Self {
            question: question.into(),
            header: header.into(),
            options,
            multi_select: false,
        }
    }

    /// Enable multi-select
    pub fn with_multi_select(mut self) -> Self {
        self.multi_select = true;
        self
    }

    /// Validate the question structure
    pub fn validate(&self) -> Result<(), String> {
        if self.header.len() > 12 {
            return Err(format!(
                "Header '{}' exceeds 12 characters (length: {})",
                self.header,
                self.header.len()
            ));
        }
        if self.question.trim().is_empty() {
            return Err("Question text cannot be empty".to_string());
        }
        if self.options.len() < 2 {
            return Err("Question must have at least 2 options".to_string());
        }
        if self.options.len() > 4 {
            return Err("Question cannot have more than 4 options".to_string());
        }
        for (i, opt) in self.options.iter().enumerate() {
            if opt.label.trim().is_empty() {
                return Err(format!("Option {} has empty label", i + 1));
            }
        }
        Ok(())
    }
}

/// Option for a structured question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display label (1-5 words)
    pub label: String,
    /// Explanation of what this option means
    pub description: String,
}

impl QuestionOption {
    /// Create a new option
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
        }
    }
}

/// A single option for selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputOption {
    /// Display label
    pub label: String,
    /// Description of what this option means
    pub description: String,
    /// Value to return if selected (defaults to label if not set)
    pub value: Option<String>,
}

impl InputOption {
    /// Create a new option
    pub fn new(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            value: None,
        }
    }

    /// Set a custom value
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Get the value (returns label if no custom value set)
    pub fn get_value(&self) -> &str {
        self.value.as_deref().unwrap_or(&self.label)
    }
}

/// Context about why input is needed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputContext {
    /// Agent needs clarification on task
    Clarification,
    /// Agent needs user decision
    Decision,
    /// Agent needs confirmation to proceed
    Confirmation,
    /// Agent wants to provide information and get feedback
    Feedback,
    /// Agent is asking about preferences
    Preference,
}
