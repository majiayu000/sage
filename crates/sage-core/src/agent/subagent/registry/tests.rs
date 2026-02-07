//! Tests for AgentRegistry

use super::super::types::{
    AgentDefinition, AgentProgress, AgentStatus, AgentType, ExecutionMetadata, SubAgentConfig,
    SubAgentResult, ToolAccessControl,
};
use super::types::AgentRegistry;

fn create_test_agent(agent_type: AgentType, name: &str) -> AgentDefinition {
    AgentDefinition {
        agent_type,
        name: name.to_string(),
        description: format!("{} agent", name),
        available_tools: ToolAccessControl::All,
        model: None,
        system_prompt: format!("Prompt for {}", name),
    }
}

// ===== Agent Definition Tests =====

#[tokio::test]
async fn test_registry_register_and_get() {
    let registry = AgentRegistry::new();
    let agent = create_test_agent(AgentType::GeneralPurpose, "General");

    registry.register(agent.clone()).await;

    let retrieved = registry.get(&AgentType::GeneralPurpose).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "General");
}

#[tokio::test]
async fn test_registry_get_by_name() {
    let registry = AgentRegistry::new();
    let agent = create_test_agent(AgentType::Plan, "Planner");

    registry.register(agent).await;

    let retrieved = registry.get_by_name("plan").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().agent_type, AgentType::Plan);
}

#[tokio::test]
async fn test_registry_list_definitions() {
    let registry = AgentRegistry::new();
    registry
        .register(create_test_agent(AgentType::GeneralPurpose, "General"))
        .await;
    registry
        .register(create_test_agent(AgentType::Explore, "Explorer"))
        .await;
    registry
        .register(create_test_agent(AgentType::Plan, "Planner"))
        .await;

    let agents = registry.list_definitions().await;
    assert_eq!(agents.len(), 3);
}

#[tokio::test]
async fn test_registry_contains() {
    let registry = AgentRegistry::new();
    registry
        .register(create_test_agent(AgentType::GeneralPurpose, "General"))
        .await;

    assert!(registry.contains(&AgentType::GeneralPurpose).await);
    assert!(!registry.contains(&AgentType::Explore).await);
}

#[tokio::test]
async fn test_registry_len_and_is_empty() {
    let registry = AgentRegistry::new();
    assert_eq!(registry.len().await, 0);
    assert!(registry.is_empty().await);

    registry
        .register(create_test_agent(AgentType::GeneralPurpose, "General"))
        .await;
    assert_eq!(registry.len().await, 1);
    assert!(!registry.is_empty().await);
}

#[tokio::test]
async fn test_registry_clear_definitions() {
    let registry = AgentRegistry::new();
    registry
        .register(create_test_agent(AgentType::GeneralPurpose, "General"))
        .await;
    registry
        .register(create_test_agent(AgentType::Explore, "Explorer"))
        .await;

    assert_eq!(registry.len().await, 2);

    registry.clear_definitions().await;
    assert_eq!(registry.len().await, 0);
    assert!(registry.is_empty().await);
}

// ===== Running Agent Tests =====

#[tokio::test]
async fn test_create_running_agent() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");

    let agent_id = registry.create_running_agent(config).await;

    assert!(!agent_id.is_empty());
    assert_eq!(registry.running_count().await, 1);
}

#[tokio::test]
async fn test_update_and_get_status() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
    let agent_id = registry.create_running_agent(config).await;

    // Initial status should be Pending
    let status = registry.get_status(&agent_id).await;
    assert!(matches!(status, Some(AgentStatus::Pending)));

    // Update to Running with progress
    let mut progress = AgentProgress::default();
    progress.add_tokens(100);
    registry.update_progress(&agent_id, progress).await;

    let status = registry.get_status(&agent_id).await;
    assert!(matches!(status, Some(AgentStatus::Running(_))));

    // Update to Failed
    registry
        .update_status(&agent_id, AgentStatus::Failed("test error".to_string()))
        .await;
    let status = registry.get_status(&agent_id).await;
    assert!(matches!(status, Some(AgentStatus::Failed(_))));
}

#[tokio::test]
async fn test_update_and_get_progress() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::Explore, "Search files");
    let agent_id = registry.create_running_agent(config).await;

    // Initially no progress (status is Pending, not Running)
    assert!(registry.get_progress(&agent_id).await.is_none());

    // Update progress
    let mut progress = AgentProgress::default();
    progress.next_step();
    progress.add_tokens(100);
    progress.increment_tool_use();
    registry.update_progress(&agent_id, progress.clone()).await;

    let retrieved_progress = registry.get_progress(&agent_id).await;
    assert!(retrieved_progress.is_some());
    let retrieved_progress = retrieved_progress.unwrap();
    assert_eq!(retrieved_progress.current_step, 1);
    assert_eq!(retrieved_progress.token_count, 100);
    assert_eq!(retrieved_progress.tool_use_count, 1);
}

#[tokio::test]
async fn test_list_running() {
    let registry = AgentRegistry::new();

    let config1 = SubAgentConfig::new(AgentType::GeneralPurpose, "Task 1");
    let id1 = registry.create_running_agent(config1).await;

    let config2 = SubAgentConfig::new(AgentType::Explore, "Task 2");
    let id2 = registry.create_running_agent(config2).await;

    let progress = AgentProgress::default();
    registry
        .update_status(&id1, AgentStatus::Running(progress))
        .await;

    let result = SubAgentResult {
        agent_id: id2.clone(),
        content: "done".to_string(),
        metadata: ExecutionMetadata::default(),
    };
    registry
        .update_status(&id2, AgentStatus::Completed(result))
        .await;

    let running = registry.list_running().await;
    assert_eq!(running.len(), 2);

    // Find our agents in the list
    let agent1 = running.iter().find(|(id, _, _)| id == &id1);
    assert!(agent1.is_some());
    let (_, agent_type, status) = agent1.unwrap();
    assert_eq!(*agent_type, AgentType::GeneralPurpose);
    assert!(matches!(status, AgentStatus::Running(_)));
}

#[tokio::test]
async fn test_kill_agent() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
    let agent_id = registry.create_running_agent(config).await;

    // Get cancel token and verify it's not cancelled
    let token = registry.get_cancel_token(&agent_id).await;
    assert!(token.is_some());
    let token = token.unwrap();
    assert!(!token.is_cancelled());

    // Kill the agent
    let result = registry.kill(&agent_id).await;
    assert!(result.is_ok());

    // Token should now be cancelled
    assert!(token.is_cancelled());

    // Trying to kill non-existent agent should fail
    let result = registry.kill("non-existent-id").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_remove_running_agent() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
    let agent_id = registry.create_running_agent(config).await;

    assert_eq!(registry.running_count().await, 1);

    registry.remove(&agent_id).await;
    assert_eq!(registry.running_count().await, 0);
    assert!(registry.get_status(&agent_id).await.is_none());
}

#[tokio::test]
async fn test_clear_running() {
    let registry = AgentRegistry::new();

    let config1 = SubAgentConfig::new(AgentType::GeneralPurpose, "Task 1");
    registry.create_running_agent(config1).await;

    let config2 = SubAgentConfig::new(AgentType::Explore, "Task 2");
    registry.create_running_agent(config2).await;

    assert_eq!(registry.running_count().await, 2);

    registry.clear_running().await;
    assert_eq!(registry.running_count().await, 0);
}

#[tokio::test]
async fn test_get_cancel_token() {
    let registry = AgentRegistry::new();
    let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test task");
    let agent_id = registry.create_running_agent(config).await;

    let token = registry.get_cancel_token(&agent_id).await;
    assert!(token.is_some());

    let token = registry.get_cancel_token("non-existent-id").await;
    assert!(token.is_none());
}

#[tokio::test]
async fn test_registry_clone() {
    let registry1 = AgentRegistry::new();
    registry1
        .register(create_test_agent(AgentType::GeneralPurpose, "General"))
        .await;

    let config = SubAgentConfig::new(AgentType::Explore, "Task");
    let agent_id = registry1.create_running_agent(config).await;

    // Clone the registry
    let registry2 = registry1.clone();

    // Both registries should share the same data
    assert_eq!(registry1.len().await, registry2.len().await);
    assert_eq!(
        registry1.running_count().await,
        registry2.running_count().await
    );

    // Modifications through one registry should be visible in the other
    let progress = AgentProgress::default();
    registry2
        .update_status(&agent_id, AgentStatus::Running(progress))
        .await;
    assert!(matches!(
        registry1.get_status(&agent_id).await,
        Some(AgentStatus::Running(_))
    ));
}

#[tokio::test]
async fn test_default() {
    let registry = AgentRegistry::default();
    assert!(registry.is_empty().await);
    assert_eq!(registry.running_count().await, 0);
}
