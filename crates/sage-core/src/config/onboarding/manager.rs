//! Onboarding manager for handling the setup flow
//!
//! This module provides the OnboardingManager which coordinates the setup process
//! for new users, including provider selection, API key configuration, and validation.

use super::state::{OnboardingState, OnboardingStep};
use crate::config::credential::{
    ConfigStatus, CredentialResolver, CredentialsFile, ResolverConfig,
};
use crate::config::Config;
use crate::config::ModelParameters;
use crate::error::{SageError, SageResult};
use std::path::PathBuf;
use tracing::{debug, info};

/// Available providers for onboarding
#[derive(Debug, Clone)]
pub struct ProviderOption {
    /// Provider identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether this provider is recommended
    pub recommended: bool,
    /// URL to get an API key
    pub api_key_url: String,
}

impl ProviderOption {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        api_key_url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            recommended: false,
            api_key_url: api_key_url.into(),
        }
    }

    pub fn recommended(mut self) -> Self {
        self.recommended = true;
        self
    }
}

/// Get the default provider options
pub fn default_provider_options() -> Vec<ProviderOption> {
    vec![
        ProviderOption::new(
            "anthropic",
            "Anthropic (Claude)",
            "Claude models - excellent for code generation and analysis",
            "https://console.anthropic.com/account/keys",
        )
        .recommended(),
        ProviderOption::new(
            "openai",
            "OpenAI (GPT)",
            "GPT-4 and GPT-3.5 models - widely used and well-documented",
            "https://platform.openai.com/api-keys",
        ),
        ProviderOption::new(
            "google",
            "Google (Gemini)",
            "Gemini models - multimodal capabilities",
            "https://makersuite.google.com/app/apikey",
        ),
        ProviderOption::new(
            "glm",
            "智谱AI (GLM)",
            "GLM-4 models - powerful Chinese and English capabilities",
            "https://open.bigmodel.cn/",
        ),
        ProviderOption::new(
            "ollama",
            "Ollama (Local)",
            "Run models locally - no API key required",
            "https://ollama.ai/",
        ),
    ]
}

/// Result of attempting to validate an API key
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the key is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Model information if valid
    pub model_info: Option<String>,
}

impl ValidationResult {
    pub fn success(model_info: impl Into<String>) -> Self {
        Self {
            valid: true,
            error: None,
            model_info: Some(model_info.into()),
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            error: Some(error.into()),
            model_info: None,
        }
    }
}

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
        Self::new(
            dirs::home_dir()
                .unwrap_or_default()
                .join(".sage"),
        )
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
        debug!("API key set for {}", self.state.selected_provider.as_deref().unwrap_or("unknown"));
        Ok(())
    }

    /// Validate the API key
    pub async fn validate_api_key(&mut self) -> ValidationResult {
        let Some(provider) = &self.state.selected_provider else {
            return ValidationResult::failure("No provider selected");
        };

        let Some(api_key) = &self.state.api_key else {
            return ValidationResult::failure("No API key provided");
        };

        // Skip validation if configured to do so
        if self.skip_validation {
            self.state.mark_key_validated();
            return ValidationResult::success("Validation skipped");
        }

        // Perform basic validation
        let result = self.perform_validation(provider, api_key).await;

        if result.valid {
            self.state.mark_key_validated();
        } else {
            self.state.mark_key_invalid(result.error.clone().unwrap_or_default());
        }

        result
    }

    /// Perform actual API key validation
    async fn perform_validation(&self, provider: &str, api_key: &str) -> ValidationResult {
        // Basic format validation
        match provider {
            "anthropic" => {
                if !api_key.starts_with("sk-ant-") && !api_key.starts_with("sk-") {
                    return ValidationResult::failure(
                        "Anthropic API keys typically start with 'sk-ant-' or 'sk-'",
                    );
                }
            }
            "openai" => {
                if !api_key.starts_with("sk-") {
                    return ValidationResult::failure(
                        "OpenAI API keys typically start with 'sk-'",
                    );
                }
            }
            "google" => {
                if api_key.len() < 30 {
                    return ValidationResult::failure(
                        "Google API keys are typically longer",
                    );
                }
            }
            "glm" => {
                // GLM/ZAI API keys are alphanumeric strings, typically 32+ characters
                // Format: alphanumeric with possible dots (e.g., "xxx.yyy")
                if api_key.len() < 20 {
                    return ValidationResult::failure(
                        "智谱AI API keys are typically longer (20+ characters)",
                    );
                }
            }
            "ollama" => {
                // Ollama doesn't need a real API key
                return ValidationResult::success("Ollama configured (local)");
            }
            _ => {}
        }

        // For now, accept any key that passes format validation
        // Real validation would make an API call
        ValidationResult::success(format!("{} API key format valid", provider))
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

        // Save to global credentials file
        let creds_path = self.global_dir.join("credentials.json");
        let mut creds = CredentialsFile::load(&creds_path).unwrap_or_default();
        creds.set_api_key(provider, api_key);

        creds.save(&creds_path).map_err(|e| {
            SageError::config(format!("Failed to save credentials: {}", e))
        })?;

        info!(
            "Saved {} credentials to {}",
            provider,
            creds_path.display()
        );

        // Save minimal global config to ensure provider is configured
        let config_path = self.global_dir.join("config.json");
        let mut config = Config::default();

        if !config.model_providers.contains_key(provider) {
            let mut params = ModelParameters::default();
            if provider == "glm" || provider == "zhipu" {
                params.model = "glm-4.7".to_string();
                params.base_url = Some("https://open.bigmodel.cn/api/anthropic".to_string());
                params.api_version = Some("2023-06-01".to_string());
                params.parallel_tool_calls = Some(false);
            }
            config.model_providers.insert(provider.clone(), params);
        }

        config.set_default_provider(provider.clone())?;

        if let Some(params) = config.model_providers.get_mut(provider) {
            if provider == "glm" || provider == "zhipu" {
                params.model = "glm-4.7".to_string();
                params.base_url = Some("https://open.bigmodel.cn/api/anthropic".to_string());
                params.api_version = Some("2023-06-01".to_string());
                params.parallel_tool_calls = Some(false);
            }
            if params.api_key.is_none() {
                params.api_key = Some(format!("${{{}_API_KEY}}", provider.to_uppercase()));
            }
        }

        std::fs::create_dir_all(&self.global_dir).map_err(|e| {
            SageError::config(format!("Failed to create config directory: {}", e))
        })?;
        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| SageError::config(format!("Failed to serialize config: {}", e)))?;
        std::fs::write(&config_path, config_json)
            .map_err(|e| SageError::config(format!("Failed to save config: {}", e)))?;

        info!("Saved global config to {}", config_path.display());

        Ok(())
    }

    /// Complete the onboarding process
    pub fn complete(&mut self) -> SageResult<()> {
        // Ensure we're at the right step
        while self.state.current_step != OnboardingStep::Complete {
            if !self.state.can_proceed() {
                return Err(SageError::config(format!(
                    "Cannot complete onboarding: stuck at {}",
                    self.state.current_step.title()
                )));
            }
            self.state.advance();
        }

        // Save configuration
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
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_provider_option_new() {
        let opt = ProviderOption::new("test", "Test Provider", "A test provider", "https://test.com");
        assert_eq!(opt.id, "test");
        assert_eq!(opt.name, "Test Provider");
        assert!(!opt.recommended);
    }

    #[test]
    fn test_provider_option_recommended() {
        let opt = ProviderOption::new("test", "Test", "Desc", "url").recommended();
        assert!(opt.recommended);
    }

    #[test]
    fn test_default_provider_options() {
        let providers = default_provider_options();
        assert!(providers.len() >= 5);

        let anthropic = providers.iter().find(|p| p.id == "anthropic");
        assert!(anthropic.is_some());
        assert!(anthropic.unwrap().recommended);

        // Check GLM provider exists
        let glm = providers.iter().find(|p| p.id == "glm");
        assert!(glm.is_some());
        assert_eq!(glm.unwrap().name, "智谱AI (GLM)");
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success("Model: claude-3");
        assert!(result.valid);
        assert!(result.error.is_none());
        assert_eq!(result.model_info, Some("Model: claude-3".to_string()));
    }

    #[test]
    fn test_validation_result_failure() {
        let result = ValidationResult::failure("Invalid key");
        assert!(!result.valid);
        assert_eq!(result.error, Some("Invalid key".to_string()));
        assert!(result.model_info.is_none());
    }

    #[test]
    fn test_onboarding_manager_new() {
        let dir = tempdir().unwrap();
        let manager = OnboardingManager::new(dir.path());

        assert_eq!(manager.current_step(), OnboardingStep::Welcome);
        assert!(!manager.providers().is_empty());
    }

    #[test]
    fn test_onboarding_manager_select_provider() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());

        assert!(manager.select_provider("anthropic").is_ok());
        assert_eq!(
            manager.state().selected_provider,
            Some("anthropic".to_string())
        );

        assert!(manager.select_provider("unknown").is_err());
    }

    #[test]
    fn test_onboarding_manager_set_api_key() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());

        assert!(manager.set_api_key("sk-test-12345").is_ok());
        assert!(manager.state().api_key.is_some());

        assert!(manager.set_api_key("").is_err());
    }

    #[test]
    fn test_onboarding_manager_selected_provider() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());

        assert!(manager.selected_provider().is_none());

        manager.select_provider("anthropic").unwrap();
        let selected = manager.selected_provider();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id, "anthropic");
    }

    #[test]
    fn test_onboarding_manager_next_step() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());

        // Welcome -> SelectProvider (can always proceed)
        assert!(manager.next_step().is_ok());
        assert_eq!(manager.current_step(), OnboardingStep::SelectProvider);

        // Can't proceed without selecting provider
        assert!(manager.next_step().is_err());

        // Select provider, then proceed
        manager.select_provider("anthropic").unwrap();
        assert!(manager.next_step().is_ok());
        assert_eq!(manager.current_step(), OnboardingStep::EnterApiKey);
    }

    #[test]
    fn test_onboarding_manager_previous_step() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());

        // Can't go back from Welcome
        assert!(manager.previous_step().is_err());

        // Go forward, then back
        manager.next_step().unwrap();
        assert!(manager.previous_step().is_ok());
        assert_eq!(manager.current_step(), OnboardingStep::Welcome);
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_api_key_no_provider() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path()).skip_validation();

        let result = manager.validate_api_key().await;
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("No provider selected"));
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_api_key_no_key() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path()).skip_validation();
        manager.select_provider("anthropic").unwrap();

        let result = manager.validate_api_key().await;
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("No API key provided"));
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_api_key_success() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path()).skip_validation();
        manager.select_provider("anthropic").unwrap();
        manager.set_api_key("sk-test-key").unwrap();

        let result = manager.validate_api_key().await;
        assert!(result.valid);
        assert!(manager.state().key_validated);
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_anthropic_key_format() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("anthropic").unwrap();
        manager.set_api_key("invalid-key").unwrap();

        let result = manager.validate_api_key().await;
        assert!(!result.valid);
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_openai_key_format() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("openai").unwrap();
        manager.set_api_key("sk-valid-format").unwrap();

        let result = manager.validate_api_key().await;
        assert!(result.valid);
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_ollama() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("ollama").unwrap();
        manager.set_api_key("anything").unwrap();

        let result = manager.validate_api_key().await;
        assert!(result.valid);
    }

    #[tokio::test]
    async fn test_onboarding_manager_validate_glm_key_format() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("glm").unwrap();

        // Too short key should fail
        manager.set_api_key("short").unwrap();
        let result = manager.validate_api_key().await;
        assert!(!result.valid);

        // Valid length key should pass
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("glm").unwrap();
        manager.set_api_key("abcdefghij1234567890abcdefghij12").unwrap();
        let result = manager.validate_api_key().await;
        assert!(result.valid);
    }

    #[test]
    fn test_onboarding_manager_save_configuration() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("anthropic").unwrap();
        manager.set_api_key("sk-test-key").unwrap();

        assert!(manager.save_configuration().is_ok());

        // Verify the file was created
        let creds_path = dir.path().join("credentials.json");
        assert!(creds_path.exists());

        // Load and verify
        let creds = CredentialsFile::load(&creds_path).unwrap();
        assert_eq!(creds.get_api_key("anthropic"), Some("sk-test-key"));
    }

    #[tokio::test]
    async fn test_onboarding_manager_complete_flow() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path()).skip_validation();

        // Go through the full flow
        manager.next_step().unwrap(); // Welcome -> SelectProvider
        manager.select_provider("anthropic").unwrap();
        manager.next_step().unwrap(); // SelectProvider -> EnterApiKey
        manager.set_api_key("sk-test-key").unwrap();
        manager.next_step().unwrap(); // EnterApiKey -> ValidateKey
        manager.validate_api_key().await;
        manager.next_step().unwrap(); // ValidateKey -> OptionalSettings
        manager.next_step().unwrap(); // OptionalSettings -> Complete

        assert!(manager.state().is_complete());
    }

    #[test]
    fn test_onboarding_manager_reset() {
        let dir = tempdir().unwrap();
        let mut manager = OnboardingManager::new(dir.path());
        manager.select_provider("anthropic").unwrap();
        manager.set_api_key("key").unwrap();
        manager.next_step().unwrap();

        manager.reset();

        assert_eq!(manager.current_step(), OnboardingStep::Welcome);
        assert!(manager.state().selected_provider.is_none());
    }

    #[test]
    fn test_onboarding_manager_credentials_path() {
        let dir = tempdir().unwrap();
        let manager = OnboardingManager::new(dir.path());

        let path = manager.credentials_path();
        assert!(path.ends_with("credentials.json"));
    }

    #[test]
    fn test_onboarding_manager_default() {
        let manager = OnboardingManager::default();
        assert_eq!(manager.current_step(), OnboardingStep::Welcome);
    }
}
