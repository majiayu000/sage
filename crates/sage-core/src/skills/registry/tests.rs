//! Tests for skill registry

#[cfg(test)]
mod tests {
    use super::super::super::types::{Skill, SkillContext, SkillSource, SkillTrigger};
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
