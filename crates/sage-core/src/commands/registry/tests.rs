//! Tests for command registry

#[cfg(test)]
mod tests {
    use crate::commands::registry::CommandRegistry;
    use crate::commands::types::{CommandSource, SlashCommand};
    use tempfile::TempDir;
    use tokio::fs::{self, File};
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = CommandRegistry::new("/project");
        assert_eq!(registry.count(), 0);
    }

    #[tokio::test]
    async fn test_register_command() {
        let mut registry = CommandRegistry::new("/project");
        let cmd = SlashCommand::new("test", "Test command");

        registry.register(cmd, CommandSource::Project);

        assert!(registry.contains("test"));
        assert_eq!(registry.count(), 1);
    }

    #[tokio::test]
    async fn test_get_command() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(
            SlashCommand::new("test", "Test prompt").with_description("Test"),
            CommandSource::Project,
        );

        let cmd = registry.get("test").unwrap();
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.description, Some("Test".to_string()));
    }

    #[tokio::test]
    async fn test_register_builtins() {
        let mut registry = CommandRegistry::new("/project");
        registry.register_builtins();

        assert!(registry.contains("help"));
        assert!(registry.contains("clear"));
        assert!(registry.contains("checkpoint"));
        assert!(registry.builtin_count() > 0);
    }

    #[tokio::test]
    async fn test_list_commands() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("a", "A"), CommandSource::Project);
        registry.register(SlashCommand::new("b", "B"), CommandSource::User);

        let list = registry.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_by_source() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("p1", "P1"), CommandSource::Project);
        registry.register(SlashCommand::new("p2", "P2"), CommandSource::Project);
        registry.register(SlashCommand::new("u1", "U1"), CommandSource::User);

        let project = registry.list_by_source(CommandSource::Project);
        assert_eq!(project.len(), 2);

        let user = registry.list_by_source(CommandSource::User);
        assert_eq!(user.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_command() {
        let mut registry = CommandRegistry::new("/project");
        registry.register(SlashCommand::new("test", "Test"), CommandSource::Project);

        assert!(registry.contains("test"));
        registry.remove("test");
        assert!(!registry.contains("test"));
    }

    #[tokio::test]
    async fn test_discover_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        // Create a command file
        let cmd_file = commands_dir.join("greet.md");
        let mut file = File::create(&cmd_file).await.unwrap();
        file.write_all(b"Say hello to $ARGUMENTS").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        let count = registry.discover().await.unwrap();

        assert_eq!(count, 1);
        assert!(registry.contains("greet"));
    }

    #[tokio::test]
    async fn test_discover_with_frontmatter() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        let cmd_file = commands_dir.join("fancy.md");
        let mut file = File::create(&cmd_file).await.unwrap();
        file.write_all(b"---\ndescription: A fancy command\n---\nDo something fancy")
            .await
            .unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        registry.discover().await.unwrap();

        let cmd = registry.get("fancy").unwrap();
        assert_eq!(cmd.description, Some("A fancy command".to_string()));
    }

    #[tokio::test]
    async fn test_project_overrides_user() {
        let temp_dir = TempDir::new().unwrap();

        // Create user command
        let user_dir = temp_dir.path().join("user").join("commands");
        fs::create_dir_all(&user_dir).await.unwrap();
        let mut f1 = File::create(user_dir.join("test.md")).await.unwrap();
        f1.write_all(b"User version").await.unwrap();

        // Create project command
        let project_dir = temp_dir
            .path()
            .join("project")
            .join(".sage")
            .join("commands");
        fs::create_dir_all(&project_dir).await.unwrap();
        let mut f2 = File::create(project_dir.join("test.md")).await.unwrap();
        f2.write_all(b"Project version").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path().join("project"))
            .with_user_config_dir(temp_dir.path().join("user"));

        registry.discover().await.unwrap();

        let cmd = registry.get("test").unwrap();
        assert_eq!(cmd.prompt_template, "Project version");
    }

    #[tokio::test]
    async fn test_builtin_not_overridden() {
        let temp_dir = TempDir::new().unwrap();
        let commands_dir = temp_dir.path().join(".sage").join("commands");
        fs::create_dir_all(&commands_dir).await.unwrap();

        // Try to override builtin
        let mut file = File::create(commands_dir.join("help.md")).await.unwrap();
        file.write_all(b"Overridden help").await.unwrap();

        let mut registry = CommandRegistry::new(temp_dir.path());
        registry.register_builtins();
        registry.discover().await.unwrap();

        let (cmd, source) = registry.get_with_source("help").unwrap();
        assert_eq!(*source, CommandSource::Builtin);
        assert!(!cmd.prompt_template.contains("Overridden"));
    }

    #[test]
    fn test_parse_frontmatter() {
        let registry = CommandRegistry::new("/project");

        let content = "---\ndescription: Test\nauthor: Me\n---\nPrompt content";
        let (metadata, prompt) = registry.parse_command_file(content);

        assert_eq!(metadata.get("description"), Some(&"Test".to_string()));
        assert_eq!(metadata.get("author"), Some(&"Me".to_string()));
        assert_eq!(prompt, "Prompt content");
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let registry = CommandRegistry::new("/project");

        let content = "Just a prompt";
        let (metadata, prompt) = registry.parse_command_file(content);

        assert!(metadata.is_empty());
        assert_eq!(prompt, "Just a prompt");
    }
}
