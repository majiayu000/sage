//! Onboarding manager for handling the setup flow
//!
//! This module provides the OnboardingManager which coordinates the setup process
//! for new users, including provider selection, API key configuration, and validation.

use super::config_saver;
use super::provider_option::{ProviderOption, default_provider_options};
use super::state::{OnboardingState, OnboardingStep};
use super::validation::{ApiKeyValidationResult, validate_api_key_format};
use crate::config::credential::{ConfigStatus, CredentialResolver, ResolverConfig};
use crate::error::{SageError, SageResult};
use std::path::PathBuf;
use tracing::{debug, info};

/// Manages the onboarding process
pub struct OnboardingManager {
    /// Current onboarding state
    state: OnboardingState,
    /// Global config directory
    global_dir: PathBuf,
    /// Available providers
    providers: Vec<ProviderOption>,
    /// Whether to skip validation (for testing)
    skip_validation: bool,
}

impl OnboardingManager {
    /// Create a new onboarding manager
    pub fn new(global_dir: impl Into<PathBuf>) -> Self {
        Self {
            state: OnboardingState::new(),
            global_dir: global_dir.into(),
            providers: default_provider_options(),
            skip_validation: false,
        }
    }

    /// Create with default global directory (~/.sage)
    pub fn with_defaults() -> Self {
        Self::new(dirs::home_dir().unwrap_or_default().join(".sage"))
    }

    /// Skip API key validation (useful for testing)
    pub fn skip_validation(mut self) -> Self {
        self.skip_validation = true;
        self
    }

    /// Get the current state
    pub fn state(&self) -> &OnboardingState {
        &self.state
    }

    /// Get a mutable reference to the state
    pub fn state_mut(&mut self) -> &mut OnboardingState {
        &mut self.state
    }

    /// Get the current step
    pub fn current_step(&self) -> OnboardingStep {
        self.state.current_step
    }

    /// Get available providers
    pub fn providers(&self) -> &[ProviderOption] {
        &self.providers
    }

    /// Get the selected provider option
    pub fn selected_provider(&self) -> Option<&ProviderOption> {
        self.state
            .selected_provider
            .as_ref()
            .and_then(|id| self.providers.iter().find(|p| &p.id == id))
    }

    /// Check if onboarding is needed
    pub fn is_needed(&self) -> bool {
        let config = ResolverConfig::default();
        let resolver = CredentialResolver::new(config);
        let status = resolver.get_status();
        status.status == ConfigStatus::Unconfigured
    }

    /// Select a provider
    pub fn select_provider(&mut self, provider_id: &str) -> SageResult<()> {
        if !self.providers.iter().any(|p| p.id == provider_id) {
            return Err(SageError::config(format!(
                "Unknown provider: {}",
                provider_id
            )));
        }
        self.state.set_provider(provider_id);
        debug!("Selected provider: {}", provider_id);
        Ok(())
    }

    /// Set the API key
    pub fn set_api_key(&mut self, api_key: &str) -> SageResult<()> {
        if api_key.is_empty() {
            return Err(SageError::config("API key cannot be empty"));
        }
        self.state.set_api_key(api_key);
        debug!(
            "API key set for {}",
            self.state.selected_provider.as_deref().unwrap_or("unknown")
        );
        Ok(())
    }

    /// Validate the API key
    pub async fn validate_api_key(&mut self) -> ApiKeyValidationResult {
        let Some(provider) = &self.state.selected_provider else {
            return ApiKeyValidationResult::failure("No provider selected");
        };
        let Some(api_key) = &self.state.api_key else {
            return ApiKeyValidationResult::failure("No API key provided");
        };

        if self.skip_validation {
            self.state.mark_key_validated();
            return ApiKeyValidationResult::success("Validation skipped");
        }

        let result = validate_api_key_format(provider, api_key).await;
        if result.valid {
            self.state.mark_key_validated();
        } else {
            self.state
                .mark_key_invalid(result.error.clone().unwrap_or_default());
        }
        result
    }

    /// Advance to the next step
    pub fn next_step(&mut self) -> SageResult<()> {
        if !self.state.can_proceed() {
            return Err(SageError::config(format!(
                "Cannot proceed from step: {}. Check requirements.",
                self.state.current_step.title()
            )));
        }
        if self.state.advance() {
            debug!("Advanced to step: {}", self.state.current_step.title());
            Ok(())
        } else {
            Err(SageError::config("Already at the last step"))
        }
    }

    /// Go back to the previous step
    pub fn previous_step(&mut self) -> SageResult<()> {
        if self.state.go_back() {
            debug!("Went back to step: {}", self.state.current_step.title());
            Ok(())
        } else {
            Err(SageError::config("Already at the first step"))
        }
    }

    /// Save the configuration
    pub fn save_configuration(&self) -> SageResult<()> {
        let Some(provider) = &self.state.selected_provider else {
            return Err(SageError::config("No provider selected"));
        };
        let Some(api_key) = &self.state.api_key else {
            return Err(SageError::config("No API key set"));
        };

        config_saver::save_credentials(&self.global_dir, provider, api_key)?;
        config_saver::save_global_config(&self.global_dir, provider)?;
        Ok(())
    }

    /// Complete the onboarding process
    pub fn complete(&mut self) -> SageResult<()> {
        while self.state.current_step != OnboardingStep::Complete {
            if !self.state.can_proceed() {
                return Err(SageError::config(format!(
                    "Cannot complete onboarding: stuck at {}",
                    self.state.current_step.title()
                )));
            }
            self.state.advance();
        }
        self.save_configuration()?;
        info!("Onboarding completed successfully");
        Ok(())
    }

    /// Reset the onboarding process
    pub fn reset(&mut self) {
        self.state.reset();
        debug!("Onboarding reset");
    }

    /// Get the credentials path
    pub fn credentials_path(&self) -> PathBuf {
        self.global_dir.join("credentials.json")
    }
}

impl Default for OnboardingManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests;
