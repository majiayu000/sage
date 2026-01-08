//! Tests for skill registry

#[cfg(test)]
mod tests {
    use super::super::super::types::{Skill, SkillContext, SkillSourceType, SkillTrigger};
    use super::super::types::SkillRegistry;
    use tempfile::TempDir;
    use tokio::fs::{self, File};
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
        assert_eq!(skill.name(), "test");
        assert_eq!(skill.description(), "Test description");
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
        assert_eq!(matching[0].name(), "rust"); // Higher priority first
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

        assert_eq!(best.name(), "high");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(Skill::new("test", "Test"));

        assert!(registry.get("test").unwrap().enabled());

        registry.disable("test");
        assert!(!registry.get("test").unwrap().enabled());

        registry.enable("test");
        assert!(registry.get("test").unwrap().enabled());
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
        assert_eq!(*skill.source(), SkillSourceType::Builtin);
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

    // New tests for Claude Code compatible features

    #[test]
    fn test_frontmatter_parsing() {
        use super::super::discovery::SkillFrontmatter;

        let content = r#"---
description: Code review skill
when_to_use: When user asks for code review
allowed_tools:
  - Read
  - Grep
  - Glob
user_invocable: true
argument_hint: "[file path]"
priority: 10
---

Please review the code at: $ARGUMENTS
"#;
        let (frontmatter, prompt) = SkillFrontmatter::parse(content);

        assert_eq!(
            frontmatter.description,
            Some("Code review skill".to_string())
        );
        assert_eq!(
            frontmatter.when_to_use,
            Some("When user asks for code review".to_string())
        );
        assert_eq!(frontmatter.allowed_tools, vec!["Read", "Grep", "Glob"]);
        assert!(frontmatter.user_invocable);
        assert_eq!(frontmatter.argument_hint, Some("[file path]".to_string()));
        assert_eq!(frontmatter.priority, Some(10));
        assert!(prompt.contains("$ARGUMENTS"));
    }

    #[test]
    fn test_frontmatter_inline_array() {
        use super::super::discovery::SkillFrontmatter;

        let content = r#"---
description: Test skill
allowed_tools: [Read, Write, Bash]
---

Test prompt
"#;
        let (frontmatter, _) = SkillFrontmatter::parse(content);

        assert_eq!(frontmatter.allowed_tools, vec!["Read", "Write", "Bash"]);
    }

    #[tokio::test]
    async fn test_discover_skill_md_format() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".sage").join("skills");
        let commit_dir = skills_dir.join("commit");
        fs::create_dir_all(&commit_dir).await.unwrap();

        // Create SKILL.md in subdirectory (Claude Code format)
        let skill_file = commit_dir.join("SKILL.md");
        let mut file = File::create(&skill_file).await.unwrap();
        file.write_all(
            br#"---
description: Smart Git Commit
when_to_use: When user asks to commit
user_invocable: true
allowed_tools:
  - Bash
  - Read
---

Run git commit with smart message
"#,
        )
        .await
        .unwrap();

        let mut registry = SkillRegistry::new(temp_dir.path());
        let count = registry.discover().await.unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("commit"));

        let skill = registry.get("commit").unwrap();
        assert_eq!(
            skill.when_to_use,
            Some("When user asks to commit".to_string())
        );
        assert!(skill.user_invocable());
    }

    #[test]
    fn test_skill_to_xml() {
        let skill = Skill::new("commit", "Smart Git Commit")
            .with_when_to_use("When user asks to commit")
            .with_source(SkillSourceType::User(std::path::PathBuf::from(
                "~/.config/sage/skills/commit",
            )));

        let xml = skill.to_xml();

        assert!(xml.contains("<skill>"));
        assert!(xml.contains("<name>\ncommit\n</name>"));
        assert!(xml.contains("Smart Git Commit - When user asks to commit"));
        assert!(xml.contains("<location>\nuser\n</location>"));
    }

    #[test]
    fn test_list_auto_invocable() {
        let mut registry = SkillRegistry::new("/project");

        // Skill with when_to_use
        registry.register(
            Skill::new("auto1", "Auto invocable").with_when_to_use("When user needs help"),
        );

        // Skill with triggers
        registry.register(
            Skill::new("auto2", "With triggers")
                .with_trigger(SkillTrigger::Keyword("test".to_string())),
        );

        // Skill without when_to_use or triggers
        registry.register(Skill::new("manual", "Manual only"));

        // Disabled skill
        registry.register(
            Skill::new("disabled", "Disabled")
                .with_when_to_use("Always")
                .disabled(),
        );

        let auto_invocable = registry.list_auto_invocable();
        assert_eq!(auto_invocable.len(), 2);
    }

    #[test]
    fn test_generate_skills_xml() {
        let mut registry = SkillRegistry::new("/project");
        registry.register(
            Skill::new("commit", "Smart Git Commit")
                .with_when_to_use("When user asks to commit")
                .with_source(SkillSourceType::User(std::path::PathBuf::from("/path"))),
        );
        registry.register(
            Skill::new("review", "Code Review")
                .with_when_to_use("When user asks for review")
                .with_source(SkillSourceType::Project(std::path::PathBuf::from("/path"))),
        );

        let xml = registry.generate_skills_xml();

        assert!(xml.contains("<available_skills>"));
        assert!(xml.contains("<skill>"));
        assert!(xml.contains("commit"));
        assert!(xml.contains("review"));
        assert!(xml.contains("</available_skills>"));
    }

    #[test]
    fn test_skill_prompt_with_args() {
        let skill = Skill::new("test", "Test").with_prompt("Review the file: $ARGUMENTS");

        let context = SkillContext::new("review").with_working_dir("/project");
        let prompt = skill.get_prompt_with_args(&context, Some("src/main.rs"));

        assert!(prompt.contains("Review the file: src/main.rs"));
    }

    #[test]
    fn test_skill_prompt_args_append() {
        let skill = Skill::new("test", "Test").with_prompt("Do something");

        let context = SkillContext::new("test").with_working_dir("/project");
        let prompt = skill.get_prompt_with_args(&context, Some("extra args"));

        assert!(prompt.contains("Do something"));
        assert!(prompt.contains("ARGUMENTS: extra args"));
    }
}
