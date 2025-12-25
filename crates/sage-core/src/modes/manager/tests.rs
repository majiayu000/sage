//! Tests for mode manager

#[cfg(test)]
mod tests {
    use crate::modes::manager::ModeManager;
    use crate::modes::types::AgentMode;

    #[tokio::test]
    async fn test_mode_manager_creation() {
        let manager = ModeManager::new();
        assert_eq!(manager.current_mode().await, AgentMode::Normal);
    }

    #[tokio::test]
    async fn test_enter_plan_mode() {
        let manager = ModeManager::new();

        let context = manager.enter_plan_mode(Some("test")).await.unwrap();

        assert_eq!(manager.current_mode().await, AgentMode::Plan);
        assert!(context.plan_file.to_string_lossy().contains(".md"));
    }

    #[tokio::test]
    async fn test_exit_plan_mode_without_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.exit_plan_mode(false).await.unwrap();

        assert!(!result.exited);
        assert_eq!(manager.current_mode().await, AgentMode::Plan);
    }

    #[tokio::test]
    async fn test_exit_plan_mode_with_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.exit_plan_mode(true).await.unwrap();

        assert!(result.exited);
        assert_eq!(manager.current_mode().await, AgentMode::Normal);
    }

    #[tokio::test]
    async fn test_is_tool_allowed_normal_mode() {
        let manager = ModeManager::new();

        assert!(manager.is_tool_allowed("Read").await);
        assert!(manager.is_tool_allowed("Write").await);
        assert!(manager.is_tool_allowed("Bash").await);
    }

    #[tokio::test]
    async fn test_is_tool_allowed_plan_mode() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        assert!(manager.is_tool_allowed("Read").await);
        assert!(manager.is_tool_allowed("Glob").await);
        assert!(!manager.is_tool_allowed("Write").await);
        assert!(!manager.is_tool_allowed("Bash").await);
    }

    #[tokio::test]
    async fn test_record_blocked_tool() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        manager.record_blocked_tool("Write").await;

        let state = manager.current_state().await;
        assert_eq!(state.blocked_tool_calls, 1);
    }

    #[tokio::test]
    async fn test_transition_to() {
        let manager = ModeManager::new();

        manager
            .transition_to(AgentMode::Debug, "Testing")
            .await
            .unwrap();
        assert_eq!(manager.current_mode().await, AgentMode::Debug);

        manager
            .transition_to(AgentMode::Review, "Review")
            .await
            .unwrap();
        assert_eq!(manager.current_mode().await, AgentMode::Review);
    }

    #[tokio::test]
    async fn test_transition_requires_approval() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        // Should fail without approval
        let result = manager.transition_to(AgentMode::Normal, "Exit").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_enter_plan_mode_twice_fails() {
        let manager = ModeManager::new();
        manager.enter_plan_mode(None).await.unwrap();

        let result = manager.enter_plan_mode(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transition_history() {
        let manager = ModeManager::new();

        manager.enter_plan_mode(None).await.unwrap();
        manager.exit_plan_mode(true).await.unwrap();

        let transitions = manager.get_transitions().await;
        assert_eq!(transitions.len(), 2);
    }

    #[tokio::test]
    async fn test_save_and_load_plan() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let manager = ModeManager::new().with_plan_dir(temp_dir.path());

        manager.enter_plan_mode(Some("test")).await.unwrap();

        let content = "# Test Plan\n\n1. Step one\n2. Step two";
        manager.save_plan(content).await.unwrap();

        let loaded = manager.load_plan().await.unwrap();
        assert_eq!(loaded, Some(content.to_string()));
    }

    #[tokio::test]
    async fn test_generate_plan_path() {
        let manager = ModeManager::new();
        let path = manager.generate_plan_path(Some("test"));

        assert!(path.to_string_lossy().contains(".md"));
    }

    #[tokio::test]
    async fn test_is_read_only() {
        let manager = ModeManager::new();

        assert!(!manager.is_read_only().await);

        manager.enter_plan_mode(None).await.unwrap();
        assert!(manager.is_read_only().await);
    }
}
