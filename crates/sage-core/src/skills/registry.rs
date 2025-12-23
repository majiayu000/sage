//! Skill registry
//!
//! This module provides the skill registry for discovering and
//! managing AI-activated skills.

use crate::error::{SageError, SageResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

use super::types::{Skill, SkillContext, SkillSource, SkillTrigger, TaskType, ToolAccess};

/// Skill registry for managing skills
pub struct SkillRegistry {
    /// Registered skills by name
    skills: HashMap<String, Skill>,
    /// Project root directory
    project_root: PathBuf,
    /// User config directory
    user_config_dir: PathBuf,
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
        self.skills.insert(skill.name.clone(), skill);
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
        self.skills.values().filter(|s| s.enabled).collect()
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
            skill.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a skill
    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.enabled = false;
            true
        } else {
            false
        }
    }

    /// Find matching skills for a context
    pub fn find_matching(&self, context: &SkillContext) -> Vec<&Skill> {
        let mut matching: Vec<_> = self
            .skills
            .values()
            .filter(|s| s.matches(context))
            .collect();

        // Sort by priority (highest first)
        matching.sort_by(|a, b| b.priority.cmp(&a.priority));

        matching
    }

    /// Find the best matching skill for a context
    pub fn find_best_match(&self, context: &SkillContext) -> Option<&Skill> {
        self.find_matching(context).first().copied()
    }

    /// Register built-in skills
    pub fn register_builtins(&mut self) {
        // Rust Expert skill
        self.register(
            Skill::new("rust-expert", "Expert Rust programming assistance")
                .with_prompt(include_str!("builtin_prompts/rust_expert.txt"))
                .with_trigger(SkillTrigger::FileExtension("rs".to_string()))
                .with_trigger(SkillTrigger::Keyword("rust".to_string()))
                .with_trigger(SkillTrigger::Keyword("cargo".to_string()))
                .with_priority(10)
                .with_source(SkillSource::Builtin),
        );

        // Testing skill
        self.register(
            Skill::new(
                "comprehensive-testing",
                "Test-driven development and testing best practices",
            )
            .with_prompt(include_str!("builtin_prompts/testing.txt"))
            .with_trigger(SkillTrigger::TaskType(TaskType::Testing))
            .with_trigger(SkillTrigger::Keyword("test".to_string()))
            .with_trigger(SkillTrigger::Keyword("spec".to_string()))
            .with_priority(8)
            .with_source(SkillSource::Builtin),
        );

        // Debugging skill
        self.register(
            Skill::new("systematic-debugging", "Systematic debugging methodology")
                .with_prompt(include_str!("builtin_prompts/debugging.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Debugging))
                .with_trigger(SkillTrigger::Keyword("bug".to_string()))
                .with_trigger(SkillTrigger::Keyword("fix".to_string()))
                .with_trigger(SkillTrigger::Keyword("error".to_string()))
                .with_priority(8)
                .with_source(SkillSource::Builtin),
        );

        // Code review skill
        self.register(
            Skill::new("code-review", "Thorough code review methodology")
                .with_prompt(include_str!("builtin_prompts/code_review.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Review))
                .with_trigger(SkillTrigger::Keyword("review".to_string()))
                .with_trigger(SkillTrigger::Keyword("pr".to_string()))
                .with_priority(7)
                .with_source(SkillSource::Builtin),
        );

        // Architecture skill
        self.register(
            Skill::new("architecture", "Software architecture and design patterns")
                .with_prompt(include_str!("builtin_prompts/architecture.txt"))
                .with_trigger(SkillTrigger::TaskType(TaskType::Architecture))
                .with_trigger(SkillTrigger::Keyword("architect".to_string()))
                .with_trigger(SkillTrigger::Keyword("design".to_string()))
                .with_trigger(SkillTrigger::Keyword("pattern".to_string()))
                .with_priority(7)
                .with_source(SkillSource::Builtin),
        );

        // Security skill
        self.register(
            Skill::new(
                "security-analysis",
                "Security analysis and vulnerability detection",
            )
            .with_prompt(include_str!("builtin_prompts/security.txt"))
            .with_trigger(SkillTrigger::TaskType(TaskType::Security))
            .with_trigger(SkillTrigger::Keyword("security".to_string()))
            .with_trigger(SkillTrigger::Keyword("vulnerability".to_string()))
            .with_trigger(SkillTrigger::Keyword("cve".to_string()))
            .with_priority(9)
            .with_source(SkillSource::Builtin),
        );

        // Git/Commit skill
        self.register(
            Skill::new("git-commit", "Git commit message best practices")
                .with_prompt(include_str!("builtin_prompts/git_commit.txt"))
                .with_trigger(SkillTrigger::Keyword("commit".to_string()))
                .with_trigger(SkillTrigger::ToolUsage("Bash".to_string()))
                .with_priority(5)
                .with_source(SkillSource::Builtin),
        );
    }

    /// Discover skills from file system
    pub async fn discover(&mut self) -> SageResult<usize> {
        let mut count = 0;

        // Discover project skills
        let project_dir = self.project_root.join(".sage").join("skills");
        count += self.discover_from_dir(&project_dir, true).await?;

        // Discover user skills
        let user_dir = self.user_config_dir.join("skills");
        count += self.discover_from_dir(&user_dir, false).await?;

        Ok(count)
    }

    /// Discover skills from a directory
    async fn discover_from_dir(&mut self, dir: &Path, is_project: bool) -> SageResult<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read skills directory: {}", e)))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SageError::storage(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();

            // Process .md files
            if path.extension().map_or(false, |ext| ext == "md") {
                if let Some(skill) = self.load_skill_from_file(&path, is_project).await? {
                    // Don't override builtins
                    let is_builtin = self
                        .skills
                        .get(&skill.name)
                        .map_or(false, |s| s.source == SkillSource::Builtin);

                    if !is_builtin {
                        self.register(skill);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a skill from a markdown file
    async fn load_skill_from_file(
        &self,
        path: &Path,
        is_project: bool,
    ) -> SageResult<Option<Skill>> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| SageError::invalid_input("Invalid skill file name".to_string()))?
            .to_string();

        let mut file = fs::File::open(path)
            .await
            .map_err(|e| SageError::storage(format!("Failed to open skill file: {}", e)))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| SageError::storage(format!("Failed to read skill file: {}", e)))?;

        let (metadata, prompt) = self.parse_skill_file(&content);

        let source = if is_project {
            SkillSource::Project(path.to_path_buf())
        } else {
            SkillSource::User(path.to_path_buf())
        };

        let mut skill = Skill::new(
            name,
            metadata.get("description").cloned().unwrap_or_default(),
        )
        .with_prompt(prompt)
        .with_source(source);

        // Parse triggers from metadata
        if let Some(triggers) = metadata.get("triggers") {
            for trigger in triggers.split(',') {
                let trigger = trigger.trim();
                if trigger.starts_with("keyword:") {
                    skill = skill.with_trigger(SkillTrigger::Keyword(
                        trigger.strip_prefix("keyword:").unwrap().to_string(),
                    ));
                } else if trigger.starts_with("extension:") {
                    skill = skill.with_trigger(SkillTrigger::FileExtension(
                        trigger.strip_prefix("extension:").unwrap().to_string(),
                    ));
                } else if trigger.starts_with("regex:") {
                    skill = skill.with_trigger(SkillTrigger::Regex(
                        trigger.strip_prefix("regex:").unwrap().to_string(),
                    ));
                }
            }
        }

        // Parse priority
        if let Some(priority) = metadata.get("priority") {
            if let Ok(p) = priority.parse() {
                skill = skill.with_priority(p);
            }
        }

        // Parse model
        if let Some(model) = metadata.get("model") {
            skill = skill.with_model(model.clone());
        }

        // Parse tools
        if let Some(tools) = metadata.get("tools") {
            let tool_access = if tools == "all" {
                ToolAccess::All
            } else if tools == "readonly" {
                ToolAccess::ReadOnly
            } else {
                ToolAccess::Only(tools.split(',').map(|s| s.trim().to_string()).collect())
            };
            skill = skill.with_tools(tool_access);
        }

        Ok(Some(skill))
    }

    /// Parse skill file with YAML frontmatter
    fn parse_skill_file(&self, content: &str) -> (HashMap<String, String>, String) {
        let mut metadata = HashMap::new();

        if let Some(after_prefix) = content.strip_prefix("---") {
            if let Some(end) = after_prefix.find("---") {
                let frontmatter = &after_prefix[..end];
                let prompt = after_prefix[end + 3..].trim().to_string();

                for line in frontmatter.lines() {
                    if let Some(colon) = line.find(':') {
                        let key = line[..colon].trim().to_string();
                        let value = line[colon + 1..].trim().to_string();
                        metadata.insert(key, value);
                    }
                }

                return (metadata, prompt);
            }
        }

        (metadata, content.to_string())
    }

    /// Get skill count
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Get builtin skill count
    pub fn builtin_count(&self) -> usize {
        self.skills
            .values()
            .filter(|s| s.source == SkillSource::Builtin)
            .count()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new(".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = SkillRegistry::new("/project");
        assert_eq!(registry.count(), 0);
    }

    #[tokio::test]
    async fn test_register_skill() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(Skill::new("test", "Test skill"));

        assert!(registry.contains("test"));
        assert_eq!(registry.count(), 1);
    }

    #[tokio::test]
    async fn test_get_skill() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(Skill::new("test", "Test description"));

        let skill = registry.get("test").unwrap();
        assert_eq!(skill.name, "test");
        assert_eq!(skill.description, "Test description");
    }

    #[tokio::test]
    async fn test_register_builtins() {
        let mut registry = SkillRegistry::new("/project");
        registry.register_builtins();

        assert!(registry.contains("rust-expert"));
        assert!(registry.contains("comprehensive-testing"));
        assert!(registry.builtin_count() > 0);
    }

    #[tokio::test]
    async fn test_find_matching() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(
            Skill::new("rust", "Rust skill")
                .with_trigger(SkillTrigger::Keyword("rust".to_string()))
                .with_priority(10),
        );
        registry.register(
            Skill::new("test", "Testing")
                .with_trigger(SkillTrigger::Keyword("test".to_string()))
                .with_priority(5),
        );

        let context = SkillContext::new("Help with rust test");
        let matching = registry.find_matching(&context);

        assert_eq!(matching.len(), 2);
        assert_eq!(matching[0].name, "rust"); // Higher priority first
    }

    #[tokio::test]
    async fn test_find_best_match() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(
            Skill::new("high", "High priority")
                .with_trigger(SkillTrigger::Always)
                .with_priority(100),
        );
        registry.register(
            Skill::new("low", "Low priority")
                .with_trigger(SkillTrigger::Always)
                .with_priority(1),
        );

        let context = SkillContext::new("Any message");
        let best = registry.find_best_match(&context).unwrap();

        assert_eq!(best.name, "high");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(Skill::new("test", "Test"));

        assert!(registry.get("test").unwrap().enabled);

        registry.disable("test");
        assert!(!registry.get("test").unwrap().enabled);

        registry.enable("test");
        assert!(registry.get("test").unwrap().enabled);
    }

    #[tokio::test]
    async fn test_discover_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".sage").join("skills");
        fs::create_dir_all(&skills_dir).await.unwrap();

        let skill_file = skills_dir.join("custom.md");
        let mut file = File::create(&skill_file).await.unwrap();
        file.write_all(
            b"---\ndescription: Custom skill\ntriggers: keyword:custom\n---\nCustom skill prompt",
        )
        .await
        .unwrap();

        let mut registry = SkillRegistry::new(temp_dir.path());
        let count = registry.discover().await.unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("custom"));
    }

    #[tokio::test]
    async fn test_builtin_not_overridden() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".sage").join("skills");
        fs::create_dir_all(&skills_dir).await.unwrap();

        // Try to override builtin
        let skill_file = skills_dir.join("rust-expert.md");
        let mut file = File::create(&skill_file).await.unwrap();
        file.write_all(b"Overridden rust skill").await.unwrap();

        let mut registry = SkillRegistry::new(temp_dir.path());
        registry.register_builtins();
        registry.discover().await.unwrap();

        let skill = registry.get("rust-expert").unwrap();
        assert_eq!(skill.source, SkillSource::Builtin);
    }

    #[test]
    fn test_parse_skill_file() {
        let registry = SkillRegistry::new("/project");

        let content =
            "---\ndescription: Test\ntriggers: keyword:test\npriority: 5\n---\nPrompt here";
        let (metadata, prompt) = registry.parse_skill_file(content);

        assert_eq!(metadata.get("description"), Some(&"Test".to_string()));
        assert_eq!(metadata.get("priority"), Some(&"5".to_string()));
        assert_eq!(prompt, "Prompt here");
    }

    #[tokio::test]
    async fn test_list_enabled() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(Skill::new("enabled1", "Enabled"));
        registry.register(Skill::new("enabled2", "Enabled"));
        registry.register(Skill::new("disabled", "Disabled").disabled());

        let enabled = registry.list_enabled();
        assert_eq!(enabled.len(), 2);
    }
}
