//! Prompt template system
//!
//! This module provides a modular system for managing prompts,
//! following Claude Code's design with separate files for different concerns.
//!
//! # Architecture
//!
//! - `system_prompt`: Core system prompt components
//! - `system_reminders`: Runtime context reminders
//! - `builder`: Fluent API for constructing prompts
//! - `tool_descriptions`: Detailed tool usage guidance
//! - `agent_prompts`: Specialized prompts for sub-agents
//! - `variables`: Template variable system
//! - `template`: Legacy template system with variable substitution
//! - `registry`: Prompt registry for dynamic management
//! - `builtin`: Common prompt patterns
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::{SystemPromptBuilder, SystemReminder};
//!
//! let prompt = SystemPromptBuilder::new()
//!     .with_agent_name("Sage Agent v1.0")
//!     .with_task("Implement a weather API client")
//!     .with_working_dir("/path/to/project")
//!     .with_git_info(true, "main", "main")
//!     .with_reminder(SystemReminder::TaskCompletionReminder)
//!     .build();
//! ```

// Core prompt modules (Claude Code style)
pub mod system_prompt;
pub mod system_reminders;
pub mod builder;
pub mod tool_descriptions;
pub mod agent_prompts;
pub mod variables;

// Legacy template system
pub mod template;
pub mod registry;
pub mod builtin;

// Re-exports for new modular system
pub use system_prompt::{GitPrompts, SecurityPolicy, SystemPrompt};
pub use system_reminders::{PlanPhase, SystemReminder};
pub use builder::SystemPromptBuilder;
pub use tool_descriptions::ToolDescriptions;
pub use agent_prompts::AgentPrompts;
pub use variables::{PromptVariables, TemplateRenderer};

// Re-exports for legacy template system
pub use template::{PromptTemplate, PromptVariable, RenderError};
pub use registry::PromptRegistry;
pub use builtin::BuiltinPrompts;
