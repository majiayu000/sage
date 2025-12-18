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
pub mod types;

// Re-export main types
pub use events::HookEvent;
pub use executor::{HookExecutionResult, HookExecutor};
pub use matcher::{matches, PatternMatcher};
pub use registry::{HookRegistry, HooksConfig};
pub use types::{
    CallbackHook, CommandHook, HookConfig, HookImplementation, HookInput, HookMatcher,
    HookOutput, HookType, HookVariant, PermissionDecision, PromptHook,
};
