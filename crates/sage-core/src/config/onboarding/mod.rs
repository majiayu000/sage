//! Onboarding system for first-time setup
//!
//! This module provides a guided setup experience for new users, including:
//! - Provider selection
//! - API key configuration
//! - Key validation
//! - Configuration saving
//!
//! # Example
//!
//! ```no_run
//! use sage_core::config::onboarding::{OnboardingManager, OnboardingStep};
//!
//! let mut manager = OnboardingManager::with_defaults();
//!
//! // Check if onboarding is needed
//! if manager.is_needed() {
//!     // Go through the setup flow
//!     manager.next_step().unwrap(); // Welcome -> SelectProvider
//!     manager.select_provider("anthropic").unwrap();
//!     manager.next_step().unwrap(); // SelectProvider -> EnterApiKey
//!     manager.set_api_key("sk-ant-...").unwrap();
//!     // ... continue with validation and completion
//! }
//! ```

mod manager;
mod state;

pub use manager::{
    OnboardingManager, ProviderOption, ValidationResult,
    default_provider_options,
};
pub use state::{OnboardingState, OnboardingStep};
