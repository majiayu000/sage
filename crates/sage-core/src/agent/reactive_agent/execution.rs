//! Reactive execution manager - orchestrates the Claude Code style workflow

use super::agent::ClaudeStyleAgent;
use super::trait_def::ReactiveAgent;
use super::types::ReactiveResponse;
use crate::config::model::Config;
use crate::error::SageResult;
use crate::types::TaskMetadata;
use uuid::Uuid;

/// Reactive execution manager - orchestrates the Claude Code style workflow
pub struct ReactiveExecutionManager {
    agent: ClaudeStyleAgent,
}

impl ReactiveExecutionManager {
    /// Create a new reactive execution manager
    pub fn new(config: Config) -> SageResult<Self> {
        let agent = ClaudeStyleAgent::new(config)?;
        Ok(Self { agent })
    }

    /// Execute a task using Claude Code style workflow
    pub async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<Vec<ReactiveResponse>> {
        let mut responses = Vec::new();
        let current_request = task.description.clone();
        let mut context = Some(task);

        // Initial request processing
        let response = self
            .agent
            .process_request(&current_request, context.take())
            .await?;
        let completed = response.completed;
        responses.push(response);

        // Continue if not completed and there's a continuation prompt
        // SAFETY: responses is never empty here since we just pushed a response above
        if !completed {
            if let Some(continuation) = responses
                .last()
                .and_then(|r| r.continuation_prompt.as_ref())
            {
                let last_response = responses.last().unwrap();
                let follow_up = self
                    .agent
                    .continue_conversation(last_response, continuation)
                    .await?;
                responses.push(follow_up);
            }
        }

        Ok(responses)
    }

    /// Interactive conversation mode
    pub async fn interactive_mode(
        &mut self,
        initial_request: &str,
    ) -> SageResult<ReactiveResponse> {
        self.agent.process_request(initial_request, None).await
    }

    /// Continue interactive conversation
    pub async fn continue_interactive(&mut self, user_input: &str) -> SageResult<ReactiveResponse> {
        // Create a dummy previous response for the interface
        let dummy_previous = ReactiveResponse {
            id: Uuid::new_v4(),
            request: String::new(),
            content: String::new(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
            duration: std::time::Duration::from_millis(0),
            completed: false,
            continuation_prompt: None,
        };

        self.agent
            .continue_conversation(&dummy_previous, user_input)
            .await
    }

    /// Get tool schemas from the underlying agent
    pub fn get_tool_schemas(&self) -> Vec<crate::tools::types::ToolSchema> {
        self.agent.get_tool_schemas()
    }

    /// Register a tool with the underlying agent
    pub fn register_tool(&mut self, tool: std::sync::Arc<dyn crate::tools::base::Tool>) {
        self.agent.register_tool(tool);
    }

    /// Register multiple tools with the underlying agent
    pub fn register_tools(&mut self, tools: Vec<std::sync::Arc<dyn crate::tools::base::Tool>>) {
        self.agent.register_tools(tools);
    }
}
