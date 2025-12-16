//! AI-activated skills system
//!
//! This module provides a skills system that allows AI to automatically
//! activate domain-specific expertise based on context.
//!
//! # Overview
//!
//! Skills are activated when the AI detects relevant context:
//! - Keywords in user messages
//! - File extensions being worked on
//! - Task types (debugging, testing, etc.)
//! - Tool usage patterns
//!
//! # Built-in Skills
//!
//! | Skill | Triggers | Description |
//! |-------|----------|-------------|
//! | `rust-expert` | .rs files, "rust" keyword | Rust programming expertise |
//! | `comprehensive-testing` | "test" keyword | TDD and testing practices |
//! | `systematic-debugging` | "bug", "error" keywords | Debugging methodology |
//! | `code-review` | "review", "pr" keywords | Code review best practices |
//! | `architecture` | "design", "pattern" keywords | Software architecture |
//! | `security-analysis` | "security" keyword | Vulnerability detection |
//! | `git-commit` | "commit" keyword | Git commit best practices |
//!
//! # Custom Skills
//!
//! Create skills in `.sage/skills/` or `~/.config/sage/skills/`:
//!
//! ```markdown
//! ---
//! description: My custom skill
//! triggers: keyword:myword, extension:py
//! priority: 10
//! ---
//! Your expertise prompt here...
//! ```
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use sage_core::skills::{SkillRegistry, SkillContext};
//!
//! // Create registry and discover skills
//! let mut registry = SkillRegistry::new("./project");
//! registry.register_builtins();
//! registry.discover().await?;
//!
//! // Find matching skill for context
//! let context = SkillContext::new("Help me fix this rust bug")
//!     .with_file("main.rs");
//!
//! if let Some(skill) = registry.find_best_match(&context) {
//!     let activation = skill.activate(&context);
//!     println!("Activated: {}", activation.skill_name);
//! }
//! ```

pub mod registry;
pub mod types;

pub use registry::SkillRegistry;
pub use types::{
    Skill, SkillActivation, SkillContext, SkillSource, SkillTrigger, TaskType, ToolAccess,
};
