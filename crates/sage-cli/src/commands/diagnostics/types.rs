//! Diagnostic types

use colored::*;

/// Check item result for diagnostics
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub hint: Option<String>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckResult {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            hint: None,
        }
    }

    pub fn warn(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            hint: None,
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn icon(&self) -> ColoredString {
        match self.status {
            CheckStatus::Pass => "✓".green().bold(),
            CheckStatus::Warn => "⚠".yellow().bold(),
            CheckStatus::Fail => "✗".red().bold(),
        }
    }
}

/// Usage data extracted from a session file
pub struct UsageData {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_created_tokens: u64,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            cache_read_tokens: 0,
            cache_created_tokens: 0,
        }
    }
}

/// Format a number with comma separators
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();

    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }

    result.chars().rev().collect()
}
