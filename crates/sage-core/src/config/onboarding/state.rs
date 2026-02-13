//! Onboarding state management
//!
//! This module provides types for tracking the onboarding progress.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Steps in the onboarding process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    /// Welcome message and overview
    Welcome,
    /// Choose default provider
    SelectProvider,
    /// Enter API key for the provider
    EnterApiKey,
    /// Validate the API key works
    ValidateKey,
    /// Configure optional settings
    OptionalSettings,
    /// Onboarding complete
    Complete,
}

impl OnboardingStep {
    /// Get the next step in the sequence
    pub fn next(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => Some(OnboardingStep::SelectProvider),
            OnboardingStep::SelectProvider => Some(OnboardingStep::EnterApiKey),
            OnboardingStep::EnterApiKey => Some(OnboardingStep::ValidateKey),
            OnboardingStep::ValidateKey => Some(OnboardingStep::OptionalSettings),
            OnboardingStep::OptionalSettings => Some(OnboardingStep::Complete),
            OnboardingStep::Complete => None,
        }
    }

    /// Get the previous step
    pub fn previous(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => None,
            OnboardingStep::SelectProvider => Some(OnboardingStep::Welcome),
            OnboardingStep::EnterApiKey => Some(OnboardingStep::SelectProvider),
            OnboardingStep::ValidateKey => Some(OnboardingStep::EnterApiKey),
            OnboardingStep::OptionalSettings => Some(OnboardingStep::ValidateKey),
            OnboardingStep::Complete => Some(OnboardingStep::OptionalSettings),
        }
    }

    /// Check if this is the first step
    pub fn is_first(&self) -> bool {
        matches!(self, OnboardingStep::Welcome)
    }

    /// Check if this is the last step
    pub fn is_last(&self) -> bool {
        matches!(self, OnboardingStep::Complete)
    }

    /// Get the step title
    pub fn title(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "Welcome to Sage Agent",
            OnboardingStep::SelectProvider => "Select AI Provider",
            OnboardingStep::EnterApiKey => "Enter API Key",
            OnboardingStep::ValidateKey => "Validating...",
            OnboardingStep::OptionalSettings => "Optional Settings",
            OnboardingStep::Complete => "Setup Complete",
        }
    }

    /// Get the step description
    pub fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => {
                "Sage Agent is a powerful AI assistant for software engineering tasks."
            }
            OnboardingStep::SelectProvider => {
                "Choose which AI provider to use (you can add more later)."
            }
            OnboardingStep::EnterApiKey => "Enter your API key for the selected provider.",
            OnboardingStep::ValidateKey => "Verifying your API key works correctly.",
            OnboardingStep::OptionalSettings => {
                "Configure optional settings like model and temperature."
            }
            OnboardingStep::Complete => "You're all set! Start using Sage Agent.",
        }
    }

    /// Get all steps in order
    pub fn all() -> &'static [OnboardingStep] {
        &[
            OnboardingStep::Welcome,
            OnboardingStep::SelectProvider,
            OnboardingStep::EnterApiKey,
            OnboardingStep::ValidateKey,
            OnboardingStep::OptionalSettings,
            OnboardingStep::Complete,
        ]
    }

    /// Get the step number (1-indexed)
    pub fn number(&self) -> usize {
        match self {
            OnboardingStep::Welcome => 1,
            OnboardingStep::SelectProvider => 2,
            OnboardingStep::EnterApiKey => 3,
            OnboardingStep::ValidateKey => 4,
            OnboardingStep::OptionalSettings => 5,
            OnboardingStep::Complete => 6,
        }
    }

    /// Get total number of steps
    pub fn total() -> usize {
        6
    }
}

impl Default for OnboardingStep {
    fn default() -> Self {
        OnboardingStep::Welcome
    }
}

impl std::fmt::Display for OnboardingStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title())
    }
}

/// Current state of the onboarding process
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OnboardingState {
    /// Current step
    pub current_step: OnboardingStep,
    /// Selected provider
    pub selected_provider: Option<String>,
    /// Entered API key (only stored temporarily during onboarding)
    #[serde(skip)]
    pub api_key: Option<String>,
    /// Whether the API key was validated
    pub key_validated: bool,
    /// Validation error message (if any)
    pub validation_error: Option<String>,
    /// When onboarding started
    pub started_at: Option<DateTime<Utc>>,
    /// When onboarding completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether to skip optional settings
    pub skip_optional: bool,
}

impl OnboardingState {
    /// Create a new onboarding state
    pub fn new() -> Self {
        Self {
            started_at: Some(Utc::now()),
            ..Default::default()
        }
    }

    /// Advance to the next step
    pub fn advance(&mut self) -> bool {
        if let Some(next) = self.current_step.next() {
            self.current_step = next;
            if next == OnboardingStep::Complete {
                self.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    /// Go back to the previous step
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.current_step.previous() {
            self.current_step = prev;
            true
        } else {
            false
        }
    }

    /// Set the selected provider
    pub fn set_provider(&mut self, provider: impl Into<String>) {
        self.selected_provider = Some(provider.into());
    }

    /// Set the API key
    pub fn set_api_key(&mut self, key: impl Into<String>) {
        self.api_key = Some(key.into());
        self.key_validated = false;
        self.validation_error = None;
    }

    /// Mark the key as validated
    pub fn mark_key_validated(&mut self) {
        self.key_validated = true;
        self.validation_error = None;
    }

    /// Mark the key validation as failed
    pub fn mark_key_invalid(&mut self, error: impl Into<String>) {
        self.key_validated = false;
        self.validation_error = Some(error.into());
    }

    /// Check if onboarding is complete
    pub fn is_complete(&self) -> bool {
        self.current_step == OnboardingStep::Complete && self.completed_at.is_some()
    }

    /// Check if the user can proceed to the next step
    pub fn can_proceed(&self) -> bool {
        match self.current_step {
            OnboardingStep::Welcome => true,
            OnboardingStep::SelectProvider => self.selected_provider.is_some(),
            OnboardingStep::EnterApiKey => {
                self.api_key.is_some() && !self.api_key.as_ref().unwrap().is_empty()
            }
            OnboardingStep::ValidateKey => self.key_validated,
            OnboardingStep::OptionalSettings => true,
            OnboardingStep::Complete => false, // Already at the end
        }
    }

    /// Get progress as a fraction (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        self.current_step.number() as f32 / OnboardingStep::total() as f32
    }

    /// Get progress as a percentage string
    pub fn progress_string(&self) -> String {
        format!(
            "Step {} of {}",
            self.current_step.number(),
            OnboardingStep::total()
        )
    }

    /// Reset the state to start over
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
