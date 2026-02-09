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
pub mod agent_prompts;
pub mod builder;
pub mod context_aware;
pub mod language_prompts;
pub mod system_prompt;
pub mod system_reminders;
pub mod tool_descriptions;
pub mod variables;

// Legacy template system
pub mod builtin;
pub mod registry;
pub mod template;

// Re-exports for new modular system
pub use agent_prompts::AgentPrompts;
pub use builder::SystemPromptBuilder;
pub use context_aware::{
    ContextAwareConfig, ConversationPhase, PhaseDetector, PhasePrompts, PhaseSignals,
};
pub use language_prompts::{Language, LanguagePrompts, detect_primary_language};
pub use system_prompt::{GitPrompts, SecurityPolicy, SystemPrompt};
pub use system_reminders::{PlanPhase, SystemReminder};
pub use tool_descriptions::ToolDescriptions;
pub use variables::{PromptVariables, TemplateRenderer};

// Re-exports for legacy template system
pub use builtin::BuiltinPrompts;
pub use registry::PromptRegistry;
pub use template::{PromptTemplate, PromptVariable, RenderError};
