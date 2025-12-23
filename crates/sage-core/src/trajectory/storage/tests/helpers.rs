//! Test helpers for trajectory storage tests

use crate::trajectory::recorder::{
    AgentStepRecord, LlmInteractionRecord, LlmResponseRecord, TokenUsageRecord, TrajectoryRecord,
};

/// Helper function to create a sample trajectory record
pub(super) fn create_test_record() -> TrajectoryRecord {
    TrajectoryRecord {
        id: uuid::Uuid::new_v4(),
        task: "Test task".to_string(),
        start_time: "2024-01-01T00:00:00Z".to_string(),
        end_time: "2024-01-01T00:05:00Z".to_string(),
        provider: "test-provider".to_string(),
        model: "test-model".to_string(),
        max_steps: Some(10),
        llm_interactions: vec![LlmInteractionRecord {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            provider: "test-provider".to_string(),
            model: "test-model".to_string(),
            input_messages: vec![serde_json::json!({"role": "user", "content": "test"})],
            response: LlmResponseRecord {
                content: "Test response".to_string(),
                model: Some("test-model".to_string()),
                finish_reason: Some("stop".to_string()),
                usage: Some(TokenUsageRecord {
                    input_tokens: 10,
                    output_tokens: 20,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    reasoning_tokens: None,
                }),
                tool_calls: None,
            },
            tools_available: Some(vec!["tool1".to_string(), "tool2".to_string()]),
        }],
        agent_steps: vec![AgentStepRecord {
            step_number: 1,
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            state: "Running".to_string(),
            llm_messages: Some(vec![serde_json::json!({"role": "user", "content": "test"})]),
            llm_response: Some(LlmResponseRecord {
                content: "Test response".to_string(),
                model: Some("test-model".to_string()),
                finish_reason: Some("stop".to_string()),
                usage: Some(TokenUsageRecord {
                    input_tokens: 10,
                    output_tokens: 20,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                    reasoning_tokens: None,
                }),
                tool_calls: None,
            }),
            tool_calls: None,
            tool_results: None,
            reflection: Some("Test reflection".to_string()),
            error: None,
        }],
        success: true,
        final_result: Some("Test completed".to_string()),
        execution_time: 300.0,
    }
}
