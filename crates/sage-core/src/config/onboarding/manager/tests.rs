//! Tests for the onboarding manager

use super::*;
use crate::config::credential::CredentialsFile;
use tempfile::tempdir;

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
    manager
        .set_api_key("abcdefghij1234567890abcdefghij12")
        .unwrap();
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
