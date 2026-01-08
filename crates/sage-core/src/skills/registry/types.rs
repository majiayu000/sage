//! Skill registry types and core implementation

use crate::error::SageResult;
use std::collections::HashMap;
use std::path::PathBuf;

use super::super::types::{Skill, SkillSource};

/// Skill registry for managing skills
pub struct SkillRegistry {
    /// Registered skills by name
    pub(super) skills: HashMap<String, Skill>,
    /// Project root directory
    pub(super) project_root: PathBuf,
    /// User config directory
    pub(super) user_config_dir: PathBuf,
}

impl SkillRegistry {
    /// Create a new skill registry
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let user_config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sage");

        Self {
            skills: HashMap::new(),
            project_root: project_root.into(),
            user_config_dir,
        }
    }

    /// Set user config directory
    pub fn with_user_config_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.user_config_dir = dir.into();
        self
    }

    /// Register a skill
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name().to_string(), skill);
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all skills
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// List enabled skills
    pub fn list_enabled(&self) -> Vec<&Skill> {
        self.skills.values().filter(|s| s.enabled()).collect()
    }

    /// Check if a skill exists
    pub fn contains(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Remove a skill
    pub fn remove(&mut self, name: &str) -> Option<Skill> {
        self.skills.remove(name)
    }

    /// Enable a skill
    pub fn enable(&mut self, name: &str) -> bool {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.set_enabled(true);
            true
        } else {
            false
        }
    }

    /// Disable a skill
    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.set_enabled(false);
            true
        } else {
            false
        }
    }

    /// Get skill count
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Get builtin skill count
    pub fn builtin_count(&self) -> usize {
        self.skills
            .values()
            .filter(|s| *s.source() == SkillSource::Builtin)
            .count()
    }

    /// Discover skills from file system
    pub async fn discover(&mut self) -> SageResult<usize> {
        let mut count = 0;

        // Discover project skills (highest priority)
        let project_dir = self.project_root.join(".sage").join("skills");
        count += self.discover_from_dir(&project_dir, true).await?;

        // Discover user skills
        let user_dir = self.user_config_dir.join("skills");
        count += self.discover_from_dir(&user_dir, false).await?;

        Ok(count)
    }

    /// List skills that can be auto-invoked by AI
    ///
    /// Returns skills that have `when_to_use` set or have triggers defined,
    /// and are not disabled for model invocation.
    pub fn list_auto_invocable(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.is_auto_invocable())
            .collect()
    }

    /// List skills that can be invoked by user (via /skill-name)
    pub fn list_user_invocable(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.enabled() && s.user_invocable())
            .collect()
    }

    /// Generate XML for system prompt injection (Claude Code compatible)
    ///
    /// Returns an XML string containing all auto-invocable skills in the format:
    /// ```xml
    /// <available_skills>
    ///   <skill>
    ///     <name>skill-name</name>
    ///     <description>description - when_to_use</description>
    ///     <location>user|project|builtin</location>
    ///   </skill>
    /// </available_skills>
    /// ```
    pub fn generate_skills_xml(&self) -> String {
        let skills = self.list_auto_invocable();

        if skills.is_empty() {
            return String::new();
        }

        let skill_xml: Vec<String> = skills.iter().map(|s| s.to_xml()).collect();

        format!(
            "<available_skills>\n{}\n</available_skills>",
            skill_xml.join("\n")
        )
    }

    /// Generate the full skill tool description for system prompt
    ///
    /// Returns a complete description including instructions and available skills XML.
    pub fn generate_skill_tool_prompt(&self) -> String {
        let skills_xml = self.generate_skills_xml();

        if skills_xml.is_empty() {
            return String::new();
        }

        format!(
            r#"Execute a skill within the main conversation

<skills_instructions>
When users ask you to perform tasks, check if any of the available skills below can help complete the task more effectively. Skills provide specialized capabilities and domain knowledge.

When users ask you to run a "slash command" or reference "/<something>" (e.g., "/commit", "/review-pr"), they are referring to a skill. Use the Skill tool to invoke the corresponding skill.

<example>
User: "run /commit"
Assistant: [Calls Skill tool with skill: "commit"]
</example>

How to invoke:
- Use the Skill tool with the skill name and optional arguments
- Examples:
  - skill: "pdf" - invoke the pdf skill
  - skill: "commit", args: "-m 'Fix bug'" - invoke with arguments
  - skill: "review-pr", args: "123" - invoke with arguments

Important:
- When a skill is relevant, you must invoke the Skill tool IMMEDIATELY as your first action
- NEVER just announce or mention a skill without actually calling the Skill tool
- This is a BLOCKING REQUIREMENT: invoke the relevant Skill tool BEFORE generating any other response about the task
- Only use skills listed in <available_skills> below
- Do not invoke a skill that is already running
</skills_instructions>

{}
"#,
            skills_xml
        )
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new(".")
    }
}
