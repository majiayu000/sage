//! Credential management system
//!
//! This module provides a comprehensive credential management system with:
//! - Multi-source credential resolution (CLI, env, project, global, auto-import)
//! - Priority-based credential selection
//! - Configuration status detection
//! - Unified config loading that never fails
//!
//! # Example
//!
//! ```no_run
//! use sage_core::config::credential::{CredentialResolver, ResolverConfig};
//!
//! let resolver = CredentialResolver::with_defaults();
//! let credentials = resolver.resolve_all();
//!
//! if let Some(key) = credentials.get_api_key("openai") {
//!     println!("OpenAI API key found");
//! }
//!
//! let status = resolver.get_status();
//! if status.status.needs_onboarding() {
//!     println!("No credentials configured, run /login");
//! }
//! ```
//!
//! # Unified Config Loading
//!
//! ```no_run
//! use sage_core::config::credential::{UnifiedConfigLoader, CliOverrides};
//!
//! let loaded = UnifiedConfigLoader::new()
//!     .with_config_file("sage_config.json")
//!     .with_cli_overrides(CliOverrides::new().with_provider("openai"))
//!     .load();
//!
//! if loaded.needs_onboarding() {
//!     println!("Run /login to configure");
//! }
//! ```

mod cli_overrides;
mod credentials_file;
mod hint;
mod loaded_config;
mod providers;
mod resolved;
mod resolver;
mod resolver_config;
mod source;
mod status;
mod unified_loader;

pub use cli_overrides::CliOverrides;
pub use credentials_file::CredentialsFile;
pub use hint::{
    HintType, StatusBarHint, hint_configured, hint_from_status,
    hint_validation_failed, hint_welcome,
};
pub use loaded_config::LoadedConfig;
pub use providers::{ProviderEnvConfig, auto_import_paths, default_providers};
pub use resolved::{ResolvedCredential, ResolvedCredentials};
pub use resolver::CredentialResolver;
pub use resolver_config::ResolverConfig;
pub use source::{CredentialPriority, CredentialSource};
pub use status::{ConfigStatus, ConfigStatusReport};
pub use unified_loader::{UnifiedConfigLoader, load_config_unified};
