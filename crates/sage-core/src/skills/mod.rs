//! AI-activated skills system
//!
//! This module provides a skills system that allows AI to automatically
//! activate domain-specific expertise based on context. It is designed to
//! be compatible with Claude Code's skill format.
//!
//! # Overview
//!
//! Skills are activated when the AI detects relevant context:
//! - Keywords in user messages
//! - File extensions being worked on
//! - Task types (debugging, testing, etc.)
//! - Tool usage patterns
//! - `when_to_use` condition (Claude Code compatible)
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
//! # Custom Skills (Claude Code Compatible)
//!
//! Skills can be defined in two formats:
//!
//! ## 1. Direct markdown file (`skill-name.md`)
//!
//! ```markdown
//! ---
//! description: My custom skill
//! when_to_use: When user asks for help with X
//! allowed_tools:
//!   - Read
//!   - Grep
//! user_invocable: true
//! argument_hint: "[file path]"
//! priority: 10
//! ---
//! Your skill prompt here. Use $ARGUMENTS for user input.
//! ```
//!
//! ## 2. Directory with SKILL.md (`skill-name/SKILL.md`)
//!
//! This format matches Claude Code's skill structure.
//!
//! # Skill Locations
//!
//! Skills are discovered from:
//! - `.sage/skills/` - Project-specific skills (highest priority)
//! - `~/.config/sage/skills/` - User-level skills
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
//!     let prompt = skill.get_prompt_with_args(&context, Some("main.rs"));
//!     println!("Skill prompt: {}", prompt);
//! }
//!
//! // Generate system prompt injection
//! let skills_xml = registry.generate_skills_xml();
//! println!("{}", skills_xml);
//! ```

pub mod registry;
pub mod types;

pub use registry::{
    SkillChangeEvent, SkillFrontmatter, SkillHotReloader, SkillRegistry, SkillWatcher,
    SkillWatcherConfig,
};
pub use types::{
    Skill, SkillActivation, SkillContext, SkillSource, SkillTrigger, TaskType, ToolAccess,
};
