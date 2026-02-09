//! Tests for sub-agent types

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::GeneralPurpose.to_string(), "general_purpose");
        assert_eq!(AgentType::Explore.to_string(), "explore");
        assert_eq!(AgentType::Plan.to_string(), "plan");
        assert_eq!(AgentType::Custom.to_string(), "custom");
    }

    #[test]
    fn test_agent_type_default() {
        assert_eq!(AgentType::default(), AgentType::GeneralPurpose);
    }

    #[test]
    fn test_agent_type_as_str() {
        assert_eq!(AgentType::GeneralPurpose.as_str(), "general_purpose");
        assert_eq!(AgentType::Explore.as_str(), "explore");
        assert_eq!(AgentType::Plan.as_str(), "plan");
        assert_eq!(AgentType::Custom.as_str(), "custom");
    }

    #[test]
    fn test_agent_type_serde() {
        let agent_type = AgentType::Explore;
        let json = serde_json::to_string(&agent_type).unwrap();
        assert_eq!(json, "\"explore\"");

        let deserialized: AgentType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, agent_type);
    }

    #[test]
    fn test_tool_access_control_display() {
        let all = ToolAccessControl::All;
        assert_eq!(all.to_string(), "all_tools");

        let specific = ToolAccessControl::Specific(vec!["Read".to_string(), "Write".to_string()]);
        assert_eq!(specific.to_string(), "tools[Read, Write]");

        let none = ToolAccessControl::None;
        assert_eq!(none.to_string(), "no_tools");
    }

    #[test]
    fn test_tool_access_control_default() {
        assert_eq!(ToolAccessControl::default(), ToolAccessControl::All);
    }

    #[test]
    fn test_tool_access_control_all() {
        let access = ToolAccessControl::All;
        assert!(access.allows_tool("any_tool"));
        assert!(access.allows_tool("another_tool"));
        assert_eq!(access.allowed_tools(), None);
    }

    #[test]
    fn test_tool_access_control_specific() {
        let access = ToolAccessControl::Specific(vec![
            "glob".to_string(),
            "grep".to_string(),
            "read".to_string(),
        ]);
        assert!(access.allows_tool("glob"));
        assert!(access.allows_tool("grep"));
        assert!(access.allows_tool("read"));
        assert!(!access.allows_tool("write"));
        assert!(!access.allows_tool("bash"));

        let tools = access.allowed_tools().unwrap();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn test_tool_access_control_none() {
        let access = ToolAccessControl::None;
        assert!(!access.allows_tool("any_tool"));
        assert_eq!(access.allowed_tools(), None);
    }

    #[test]
    fn test_agent_definition_custom() {
        let agent = AgentDefinition::custom(
            "Test Agent".to_string(),
            "A test agent".to_string(),
            ToolAccessControl::All,
            "Test prompt".to_string(),
        );

        assert_eq!(agent.agent_type, AgentType::Custom);
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.description, "A test agent");
        assert_eq!(agent.system_prompt, "Test prompt");
        assert!(agent.can_use_tool("any_tool"));
    }

    #[test]
    fn test_agent_definition_can_use_tool() {
        let agent = AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Fast explorer".to_string(),
            available_tools: ToolAccessControl::Specific(vec!["glob".to_string()]),
            model: Some("haiku".to_string()),
            system_prompt: "Explore!".to_string(),
        };

        assert!(agent.can_use_tool("glob"));
        assert!(!agent.can_use_tool("write"));
    }

    #[test]
    fn test_agent_definition_display() {
        let def = AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Fast exploration agent".to_string(),
            available_tools: ToolAccessControl::Specific(vec!["Read".to_string()]),
            model: Some("gpt-3.5-turbo".to_string()),
            system_prompt: "You are an explorer".to_string(),
        };

        let display = def.to_string();
        assert!(display.contains("Explorer"));
        assert!(display.contains("explore"));
        assert!(display.contains("Read"));
    }

    #[test]
    fn test_subagent_config_new() {
        let config = SubAgentConfig::new(AgentType::Explore, "Find files");
        assert_eq!(config.agent_type, AgentType::Explore);
        assert_eq!(config.prompt, "Find files");
        assert_eq!(config.resume_id, None);
        assert!(!config.run_in_background);
        assert_eq!(config.model_override, None);
    }

    #[test]
    fn test_subagent_config_builder() {
        let config = SubAgentConfig::new(AgentType::Plan, "Design system")
            .with_resume_id("resume-123".to_string())
            .with_background(true)
            .with_model("gpt-4".to_string());

        assert_eq!(config.agent_type, AgentType::Plan);
        assert_eq!(config.prompt, "Design system");
        assert_eq!(config.resume_id, Some("resume-123".to_string()));
        assert!(config.run_in_background);
        assert_eq!(config.model_override, Some("gpt-4".to_string()));
    }

    #[test]
    fn test_agent_progress_add_activity() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.recent_activities.len(), 0);

        progress.add_activity("Step 1".to_string());
        assert_eq!(progress.recent_activities.len(), 1);

        // Add 11 activities to test the limit
        for i in 2..=12 {
            progress.add_activity(format!("Step {}", i));
        }

        // Should only keep last 10
        assert_eq!(progress.recent_activities.len(), 10);
        assert_eq!(progress.recent_activities[0], "Step 3");
        assert_eq!(progress.recent_activities[9], "Step 12");
    }

    #[test]
    fn test_agent_progress_increment_tool_use() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.tool_use_count, 0);

        progress.increment_tool_use();
        assert_eq!(progress.tool_use_count, 1);

        progress.increment_tool_use();
        assert_eq!(progress.tool_use_count, 2);
    }

    #[test]
    fn test_agent_progress_add_tokens() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.token_count, 0);

        progress.add_tokens(100);
        assert_eq!(progress.token_count, 100);

        progress.add_tokens(50);
        assert_eq!(progress.token_count, 150);
    }

    #[test]
    fn test_agent_progress_next_step() {
        let mut progress = AgentProgress::default();
        assert_eq!(progress.current_step, 0);

        progress.next_step();
        assert_eq!(progress.current_step, 1);

        progress.next_step();
        assert_eq!(progress.current_step, 2);
    }

    #[test]
    fn test_agent_progress_serde() {
        let progress = AgentProgress {
            recent_activities: vec!["Reading files".to_string(), "Analyzing code".to_string()]
                .into(),
            token_count: 500,
            tool_use_count: 3,
            current_step: 2,
        };

        let json = serde_json::to_string(&progress).unwrap();
        let deserialized: AgentProgress = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized, progress);
    }

    #[test]
    fn test_execution_metadata_from_progress() {
        let mut progress = AgentProgress::default();
        progress.token_count = 1000;
        progress.tool_use_count = 5;
        progress.current_step = 3;

        let metadata = ExecutionMetadata::from_progress(&progress, 5000);
        assert_eq!(metadata.total_tokens, 1000);
        assert_eq!(metadata.total_tool_uses, 5);
        assert_eq!(metadata.execution_time_ms, 5000);
        assert_eq!(metadata.tools_used.len(), 0);
    }

    #[test]
    fn test_execution_metadata_add_tool() {
        let mut metadata = ExecutionMetadata::default();
        assert_eq!(metadata.tools_used.len(), 0);

        metadata.add_tool("Read".to_string());
        assert_eq!(metadata.tools_used.len(), 1);
        assert_eq!(metadata.tools_used[0], "Read");

        // Adding same tool should not duplicate
        metadata.add_tool("Read".to_string());
        assert_eq!(metadata.tools_used.len(), 1);

        metadata.add_tool("Write".to_string());
        assert_eq!(metadata.tools_used.len(), 2);
    }

    #[test]
    fn test_agent_status_is_terminal() {
        assert!(!AgentStatus::Pending.is_terminal());
        assert!(!AgentStatus::Running(AgentProgress::default()).is_terminal());
        assert!(
            AgentStatus::Completed(SubAgentResult {
                agent_id: "test".to_string(),
                content: "done".to_string(),
                metadata: ExecutionMetadata::default(),
            })
            .is_terminal()
        );
        assert!(AgentStatus::Failed("error".to_string()).is_terminal());
        assert!(AgentStatus::Killed.is_terminal());
    }

    #[test]
    fn test_agent_status_is_running() {
        assert!(!AgentStatus::Pending.is_running());
        assert!(AgentStatus::Running(AgentProgress::default()).is_running());
        assert!(
            !AgentStatus::Completed(SubAgentResult {
                agent_id: "test".to_string(),
                content: "done".to_string(),
                metadata: ExecutionMetadata::default(),
            })
            .is_running()
        );
        assert!(!AgentStatus::Failed("error".to_string()).is_running());
        assert!(!AgentStatus::Killed.is_running());
    }

    #[test]
    fn test_agent_status_progress() {
        let progress = AgentProgress {
            recent_activities: vec!["test".to_string()].into(),
            token_count: 100,
            tool_use_count: 2,
            current_step: 1,
        };

        let status = AgentStatus::Running(progress.clone());
        assert!(status.progress().is_some());
        assert_eq!(status.progress().unwrap().token_count, 100);

        assert!(AgentStatus::Pending.progress().is_none());
        assert!(AgentStatus::Killed.progress().is_none());
    }

    #[test]
    fn test_agent_status_progress_mut() {
        let mut status = AgentStatus::Running(AgentProgress::default());

        if let Some(progress) = status.progress_mut() {
            progress.add_tokens(500);
        }

        assert_eq!(status.progress().unwrap().token_count, 500);
    }

    #[test]
    fn test_agent_status_result() {
        let result = SubAgentResult {
            agent_id: "test-123".to_string(),
            content: "Success!".to_string(),
            metadata: ExecutionMetadata::default(),
        };

        let status = AgentStatus::Completed(result.clone());
        assert!(status.result().is_some());
        assert_eq!(status.result().unwrap().agent_id, "test-123");

        assert!(AgentStatus::Pending.result().is_none());
        assert!(
            AgentStatus::Running(AgentProgress::default())
                .result()
                .is_none()
        );
        assert!(AgentStatus::Failed("error".to_string()).result().is_none());
    }

    #[test]
    fn test_running_agent_new() {
        let config = SubAgentConfig::new(AgentType::Explore, "Test prompt");

        let agent = RunningAgent::new("agent-123".to_string(), AgentType::Explore, config.clone());

        assert_eq!(agent.id, "agent-123");
        assert_eq!(agent.agent_type, AgentType::Explore);
        assert!(matches!(agent.status, AgentStatus::Pending));
        assert!(!agent.cancel_token.is_cancelled());
    }

    #[test]
    fn test_running_agent_elapsed_ms() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        // Sleep a bit to ensure elapsed time is > 0
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(agent.elapsed_ms() >= 10);
    }

    #[test]
    fn test_running_agent_is_active() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let mut agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        // Initially pending - active
        assert!(agent.is_active());

        // Running - active
        agent.status = AgentStatus::Running(AgentProgress::default());
        assert!(agent.is_active());

        // Completed - not active
        agent.status = AgentStatus::Completed(SubAgentResult {
            agent_id: "test".to_string(),
            content: "done".to_string(),
            metadata: ExecutionMetadata::default(),
        });
        assert!(!agent.is_active());

        // Failed - not active
        agent.status = AgentStatus::Failed("error".to_string());
        assert!(!agent.is_active());

        // Killed - not active
        agent.status = AgentStatus::Killed;
        assert!(!agent.is_active());
    }

    #[test]
    fn test_running_agent_cancel() {
        let config = SubAgentConfig::new(AgentType::GeneralPurpose, "Test");
        let agent = RunningAgent::new("test".to_string(), AgentType::GeneralPurpose, config);

        assert!(!agent.is_cancelled());

        agent.cancel();
        assert!(agent.is_cancelled());
    }

    #[test]
    fn test_subagent_config_serde() {
        let config = SubAgentConfig {
            agent_type: AgentType::Plan,
            prompt: "Design the system".to_string(),
            resume_id: Some("resume-123".to_string()),
            run_in_background: true,
            model_override: Some("gpt-4".to_string()),
            thoroughness: Thoroughness::VeryThorough,
            working_directory: WorkingDirectoryConfig::Inherited,
            tool_access: ToolAccessControl::All,
            parent_cwd: None,
            parent_tools: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: SubAgentConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_type, AgentType::Plan);
        assert_eq!(deserialized.prompt, "Design the system");
        assert_eq!(deserialized.resume_id, Some("resume-123".to_string()));
        assert!(deserialized.run_in_background);
        assert_eq!(deserialized.model_override, Some("gpt-4".to_string()));
        assert_eq!(deserialized.thoroughness, Thoroughness::VeryThorough);
    }

    #[test]
    fn test_thoroughness_levels() {
        assert_eq!(Thoroughness::Quick.suggested_max_steps(), 5);
        assert_eq!(Thoroughness::Medium.suggested_max_steps(), 15);
        assert_eq!(Thoroughness::VeryThorough.suggested_max_steps(), 30);

        assert_eq!(Thoroughness::Quick.as_str(), "quick");
        assert_eq!(Thoroughness::Medium.as_str(), "medium");
        assert_eq!(Thoroughness::VeryThorough.as_str(), "very_thorough");
    }

    #[test]
    fn test_subagent_result_serde() {
        let result = SubAgentResult {
            agent_id: "agent-456".to_string(),
            content: "Task completed successfully".to_string(),
            metadata: ExecutionMetadata {
                total_tokens: 2500,
                total_tool_uses: 8,
                execution_time_ms: 15000,
                tools_used: vec!["Read".to_string(), "Write".to_string()],
            },
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: SubAgentResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.agent_id, "agent-456");
        assert_eq!(deserialized.content, "Task completed successfully");
        assert_eq!(deserialized.metadata.total_tokens, 2500);
        assert_eq!(deserialized.metadata.total_tool_uses, 8);
        assert_eq!(deserialized.metadata.execution_time_ms, 15000);
        assert_eq!(deserialized.metadata.tools_used.len(), 2);
    }
}
