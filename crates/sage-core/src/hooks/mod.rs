//! Hook system for Sage Agent
//!
//! This module provides a flexible hook system that allows executing custom logic
//! at various points in the agent lifecycle. Hooks can be implemented as shell commands
//! or LLM prompts, and can optionally block execution based on their results.
//!
//! # Examples
//!
//! ```rust
//! use sage_core::hooks::{
//!     HookRegistry, HookExecutor, HookConfig, HookType, HookImplementation,
//!     CommandHook, HookMatcher, HookInput, HookEvent,
//! };
//! use std::collections::HashMap;
//! use tokio_util::sync::CancellationToken;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a registry and register a hook matcher for PreToolUse event
//! let registry = HookRegistry::new();
//! let hook_config = HookConfig {
//!     name: "pre_tool_check".to_string(),
//!     hook_type: HookType::PreToolExecution,
//!     implementation: HookImplementation::Command(CommandHook {
//!         command: "echo '{\"should_continue\": true}'".to_string(),
//!         timeout_secs: 30,
//!         status_message: None,
//!         working_dir: None,
//!         env: HashMap::new(),
//!     }),
//!     can_block: true,
//!     timeout_secs: 30,
//!     enabled: true,
//! };
//! let matcher = HookMatcher::new(Some("bash".to_string()), hook_config);
//! registry.register(HookEvent::PreToolUse, matcher)?;
//!
//! // Create an executor and execute hooks
//! let executor = HookExecutor::new(registry);
//! let input = HookInput::new(HookEvent::PreToolUse, "session-123");
//! let cancel = CancellationToken::new();
//!
//! let results = executor.execute(HookEvent::PreToolUse, "bash", input, cancel).await?;
//! for result in results {
//!     println!("Hook result: {:?}", result);
//! }
//! # Ok(())
//! # }
//! ```

pub mod events;
pub mod executor;
pub mod matcher;
pub mod registry;

// Type modules (split from the original large types.rs)
pub mod callback_hook;
pub mod command_hook;
pub mod hook_config;
pub mod hook_input;
pub mod hook_output;
pub mod hook_types;
pub mod prompt_hook;

// Backwards compatibility: keep types module as an alias
#[allow(deprecated)]
pub mod types {
    //! Legacy types module - re-exports all hook types for backward compatibility
    //!
    //! This module is kept for backward compatibility. New code should import
    //! types directly from their respective modules.

    pub use super::callback_hook::CallbackHook;
    pub use super::command_hook::CommandHook;
    pub use super::hook_config::{HookConfig, HookMatcher};
    pub use super::hook_input::HookInput;
    pub use super::hook_output::HookOutput;
    pub use super::hook_types::{HookImplementation, HookType, HookVariant, PermissionDecision};
    pub use super::prompt_hook::PromptHook;
}

// Re-export main types for convenience
pub use callback_hook::CallbackHook;
pub use command_hook::CommandHook;
pub use events::HookEvent;
pub use executor::{HookExecutionResult, HookExecutor};
pub use hook_config::{HookConfig, HookMatcher};
pub use hook_input::HookInput;
pub use hook_output::HookOutput;
pub use hook_types::{HookImplementation, HookType, HookVariant, PermissionDecision};
pub use matcher::{PatternMatcher, matches};
pub use prompt_hook::PromptHook;
pub use registry::{HookRegistry, HooksConfig};
