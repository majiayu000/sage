use super::*;

#[test]
fn test_onboarding_step_next() {
    assert_eq!(
        OnboardingStep::Welcome.next(),
        Some(OnboardingStep::SelectProvider)
    );
    assert_eq!(
        OnboardingStep::SelectProvider.next(),
        Some(OnboardingStep::EnterApiKey)
    );
    assert_eq!(
        OnboardingStep::EnterApiKey.next(),
        Some(OnboardingStep::ValidateKey)
    );
    assert_eq!(
        OnboardingStep::ValidateKey.next(),
        Some(OnboardingStep::OptionalSettings)
    );
    assert_eq!(
        OnboardingStep::OptionalSettings.next(),
        Some(OnboardingStep::Complete)
    );
    assert_eq!(OnboardingStep::Complete.next(), None);
}

#[test]
fn test_onboarding_step_previous() {
    assert_eq!(OnboardingStep::Welcome.previous(), None);
    assert_eq!(
        OnboardingStep::SelectProvider.previous(),
        Some(OnboardingStep::Welcome)
    );
    assert_eq!(
        OnboardingStep::EnterApiKey.previous(),
        Some(OnboardingStep::SelectProvider)
    );
    assert_eq!(
        OnboardingStep::ValidateKey.previous(),
        Some(OnboardingStep::EnterApiKey)
    );
    assert_eq!(
        OnboardingStep::OptionalSettings.previous(),
        Some(OnboardingStep::ValidateKey)
    );
    assert_eq!(
        OnboardingStep::Complete.previous(),
        Some(OnboardingStep::OptionalSettings)
    );
}

#[test]
fn test_onboarding_step_is_first_last() {
    assert!(OnboardingStep::Welcome.is_first());
    assert!(!OnboardingStep::SelectProvider.is_first());

    assert!(!OnboardingStep::Welcome.is_last());
    assert!(OnboardingStep::Complete.is_last());
}

#[test]
fn test_onboarding_step_title() {
    assert_eq!(OnboardingStep::Welcome.title(), "Welcome to Sage Agent");
    assert_eq!(OnboardingStep::SelectProvider.title(), "Select AI Provider");
    assert_eq!(OnboardingStep::Complete.title(), "Setup Complete");
}

#[test]
fn test_onboarding_step_description() {
    assert!(!OnboardingStep::Welcome.description().is_empty());
    assert!(!OnboardingStep::Complete.description().is_empty());
}

#[test]
fn test_onboarding_step_all() {
    let all = OnboardingStep::all();
    assert_eq!(all.len(), 6);
    assert_eq!(all[0], OnboardingStep::Welcome);
    assert_eq!(all[5], OnboardingStep::Complete);
}

#[test]
fn test_onboarding_step_number() {
    assert_eq!(OnboardingStep::Welcome.number(), 1);
    assert_eq!(OnboardingStep::Complete.number(), 6);
}

#[test]
fn test_onboarding_step_total() {
    assert_eq!(OnboardingStep::total(), 6);
}

#[test]
fn test_onboarding_step_default() {
    assert_eq!(OnboardingStep::default(), OnboardingStep::Welcome);
}

#[test]
fn test_onboarding_step_display() {
    assert_eq!(
        format!("{}", OnboardingStep::Welcome),
        "Welcome to Sage Agent"
    );
}

#[test]
fn test_onboarding_state_new() {
    let state = OnboardingState::new();
    assert_eq!(state.current_step, OnboardingStep::Welcome);
    assert!(state.started_at.is_some());
    assert!(state.completed_at.is_none());
}

#[test]
fn test_onboarding_state_advance() {
    let mut state = OnboardingState::new();
    assert!(state.advance());
    assert_eq!(state.current_step, OnboardingStep::SelectProvider);

    // Advance through all steps
    while state.advance() {}
    assert_eq!(state.current_step, OnboardingStep::Complete);
    assert!(state.completed_at.is_some());
}

#[test]
fn test_onboarding_state_go_back() {
    let mut state = OnboardingState::new();
    state.advance(); // Go to SelectProvider
    state.advance(); // Go to EnterApiKey

    assert!(state.go_back());
    assert_eq!(state.current_step, OnboardingStep::SelectProvider);

    assert!(state.go_back());
    assert_eq!(state.current_step, OnboardingStep::Welcome);

    assert!(!state.go_back()); // Can't go back from Welcome
}

#[test]
fn test_onboarding_state_set_provider() {
    let mut state = OnboardingState::new();
    state.set_provider("anthropic");
    assert_eq!(state.selected_provider, Some("anthropic".to_string()));
}

#[test]
fn test_onboarding_state_set_api_key() {
    let mut state = OnboardingState::new();
    state.set_api_key("sk-test-12345");
    assert_eq!(state.api_key, Some("sk-test-12345".to_string()));
    assert!(!state.key_validated);
}

#[test]
fn test_onboarding_state_mark_key_validated() {
    let mut state = OnboardingState::new();
    state.set_api_key("key");
    state.mark_key_validated();
    assert!(state.key_validated);
    assert!(state.validation_error.is_none());
}

#[test]
fn test_onboarding_state_mark_key_invalid() {
    let mut state = OnboardingState::new();
    state.set_api_key("bad-key");
    state.mark_key_invalid("Invalid API key");
    assert!(!state.key_validated);
    assert_eq!(state.validation_error, Some("Invalid API key".to_string()));
}

#[test]
fn test_onboarding_state_is_complete() {
    let mut state = OnboardingState::new();
    assert!(!state.is_complete());

    // Advance to complete
    while state.advance() {}
    assert!(state.is_complete());
}

#[test]
fn test_onboarding_state_can_proceed() {
    let mut state = OnboardingState::new();

    // Welcome step - can always proceed
    assert!(state.can_proceed());

    // SelectProvider - needs provider selected
    state.advance();
    assert!(!state.can_proceed());
    state.set_provider("anthropic");
    assert!(state.can_proceed());

    // EnterApiKey - needs key entered
    state.advance();
    assert!(!state.can_proceed());
    state.set_api_key("test-key");
    assert!(state.can_proceed());

    // ValidateKey - needs validation
    state.advance();
    assert!(!state.can_proceed());
    state.mark_key_validated();
    assert!(state.can_proceed());

    // OptionalSettings - can always proceed
    state.advance();
    assert!(state.can_proceed());

    // Complete - cannot proceed further
    state.advance();
    assert!(!state.can_proceed());
}

#[test]
fn test_onboarding_state_progress() {
    let mut state = OnboardingState::new();
    assert!((state.progress() - 1.0 / 6.0).abs() < 0.01);

    state.advance();
    assert!((state.progress() - 2.0 / 6.0).abs() < 0.01);
}

#[test]
fn test_onboarding_state_progress_string() {
    let state = OnboardingState::new();
    assert_eq!(state.progress_string(), "Step 1 of 6");
}

#[test]
fn test_onboarding_state_reset() {
    let mut state = OnboardingState::new();
    state.set_provider("anthropic");
    state.set_api_key("key");
    state.advance();

    state.reset();
    assert_eq!(state.current_step, OnboardingStep::Welcome);
    assert!(state.selected_provider.is_none());
    assert!(state.api_key.is_none());
}

#[test]
fn test_onboarding_step_serialize() {
    let step = OnboardingStep::SelectProvider;
    let json = serde_json::to_string(&step).unwrap();
    assert_eq!(json, "\"select_provider\"");
}

#[test]
fn test_onboarding_step_deserialize() {
    let step: OnboardingStep = serde_json::from_str("\"enter_api_key\"").unwrap();
    assert_eq!(step, OnboardingStep::EnterApiKey);
}
