//! Sub-agent executor for running agents with filtered tools

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::registry::AgentRegistry;
use super::types::{AgentDefinition, AgentType, SubAgentResult};
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::{LlmMessage, MessageRole};
use crate::tools::base::Tool;
use crate::tools::types::{ToolCall, ToolResult};
use crate::types::LlmUsage;

/// Configuration for sub-agent execution
#[derive(Debug, Clone)]
pub struct SubAgentConfig {
    /// Agent type to use
    pub agent_type: AgentType,
    /// Task description
    pub task: String,
    /// Additional context
    pub context: Option<String>,
    /// Override maximum steps
    pub max_steps: Option<usize>,
    /// Override temperature
    pub temperature: Option<f64>,
}

impl SubAgentConfig {
    /// Create a new sub-agent configuration
    pub fn new(agent_type: AgentType, task: impl Into<String>) -> Self {
        Self {
            agent_type,
            task: task.into(),
            context: None,
            max_steps: None,
            temperature: None,
        }
    }

    /// Set context
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Set max steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = Some(max_steps);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Progress update from executor
#[derive(Debug, Clone)]
pub struct AgentProgress {
    /// Current step
    pub step: usize,
    /// Max steps
    pub max_steps: usize,
    /// Current action
    pub action: String,
    /// Progress percentage
    pub percentage: u8,
}

impl AgentProgress {
    /// Create progress update
    pub fn new(step: usize, max_steps: usize, action: impl Into<String>) -> Self {
        let percentage = if max_steps > 0 {
            ((step as f64 / max_steps as f64) * 100.0).min(100.0) as u8
        } else {
            0
        };

        Self {
            step,
            max_steps,
            action: action.into(),
            percentage,
        }
    }
}

/// Message from executor to monitor progress
#[derive(Debug, Clone)]
pub enum ExecutorMessage {
    /// Progress update
    Progress(AgentProgress),
    /// Tool call started
    ToolCall { name: String, id: String },
    /// Tool result received
    ToolResult { id: String, success: bool },
    /// Execution completed
    Completed(SubAgentResult),
    /// Execution failed
    Failed(String),
}

/// Result of a single step
enum StepResult {
    /// Continue to next step
    Continue,
    /// Task completed with final message
    Completed(String),
    /// Need more steps but hit limit
    NeedsMoreSteps,
}

/// Sub-agent executor
pub struct SubAgentExecutor {
    registry: Arc<AgentRegistry>,
    llm_client: Arc<LlmClient>,
    all_tools: Vec<Arc<dyn Tool>>,
    max_steps: usize,
}

impl SubAgentExecutor {
    /// Create a new sub-agent executor
    pub fn new(
        registry: Arc<AgentRegistry>,
        llm_client: Arc<LlmClient>,
        tools: Vec<Arc<dyn Tool>>,
    ) -> Self {
        Self {
            registry,
            llm_client,
            all_tools: tools,
            max_steps: usize::MAX, // No limit by default
        }
    }

    /// Set maximum steps
    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Execute a sub-agent with the given configuration
    pub async fn execute(
        &self,
        config: SubAgentConfig,
        cancel: CancellationToken,
    ) -> SageResult<SubAgentResult> {
        let start_time = Instant::now();

        // Get agent definition
        let definition = self
            .registry
            .get(&config.agent_type)
            .ok_or_else(|| SageError::agent(format!("Agent type {:?} not found", config.agent_type)))?;

        // Filter tools based on agent definition
        let tools = self.filter_tools(&definition);

        // Build initial messages
        let mut messages = Vec::new();

        // Add system prompt
        if !definition.system_prompt.is_empty() {
            messages.push(LlmMessage::system(&definition.system_prompt));
        }

        // Add context if provided
        let user_message = if let Some(context) = &config.context {
            format!("{}\n\nContext:\n{}\n\nTask: {}", definition.description, context, config.task)
        } else {
            format!("{}\n\nTask: {}", definition.description, config.task)
        };
        messages.push(LlmMessage::user(user_message));

        // Determine max steps
        let max_steps = config.max_steps.unwrap_or(self.max_steps);

        // Track execution
        let mut steps_taken = 0;
        let mut tool_calls_count = 0;
        let mut total_usage = LlmUsage::default();

        // Execute steps
        loop {
            // Check cancellation
            if cancel.is_cancelled() {
                return Err(SageError::Cancelled);
            }

            // Check step limit
            if steps_taken >= max_steps {
                let duration_secs = start_time.elapsed().as_secs_f64();

                return Ok(SubAgentResult::failure(
                    definition.id(),
                    "Task incomplete: maximum steps reached".to_string(),
                    duration_secs,
                    steps_taken,
                ));
            }

            steps_taken += 1;

            // Execute step
            let step_result = self.execute_step(&mut messages, &tools, &cancel).await?;

            // Update usage if available
            if let Some(usage) = messages
                .last()
                .and_then(|m| m.metadata.get("usage"))
                .and_then(|v| serde_json::from_value::<LlmUsage>(v.clone()).ok())
            {
                total_usage.add(&usage);
            }

            match step_result {
                StepResult::Continue => {
                    // Continue to next iteration
                    continue;
                }
                StepResult::Completed(output) => {
                    // Task completed successfully
                    let duration_secs = start_time.elapsed().as_secs_f64();
                    let result_data = serde_json::json!({
                        "output": output,
                        "token_usage": total_usage,
                        "tool_calls_count": tool_calls_count,
                    });

                    return Ok(SubAgentResult::success(
                        definition.id(),
                        result_data,
                        duration_secs,
                        steps_taken,
                    ));
                }
                StepResult::NeedsMoreSteps => {
                    // Continue to next iteration
                    continue;
                }
            }
        }
    }

    /// Execute in background, returning channel for progress updates
    pub async fn execute_background(
        &self,
        config: SubAgentConfig,
    ) -> SageResult<(String, mpsc::Receiver<ExecutorMessage>)> {
        let (tx, rx) = mpsc::channel(100);
        let cancel = CancellationToken::new();
        let executor = Arc::new(self.clone());
        let execution_id = uuid::Uuid::new_v4().to_string();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            let result = executor.execute(config, cancel_clone).await;

            let msg = match result {
                Ok(result) => ExecutorMessage::Completed(result),
                Err(e) => ExecutorMessage::Failed(e.to_string()),
            };

            let _ = tx.send(msg).await;
        });

        Ok((execution_id, rx))
    }

    /// Filter tools based on agent definition
    fn filter_tools(&self, definition: &AgentDefinition) -> Vec<Arc<dyn Tool>> {
        self.all_tools
            .iter()
            .filter(|tool| definition.can_use_tool(tool.name()))
            .cloned()
            .collect()
    }

    /// Build system prompt for agent (unused for now)
    #[allow(dead_code)]
    fn build_system_prompt(&self, definition: &AgentDefinition, tools: &[Arc<dyn Tool>]) -> String {
        let mut prompt = definition.system_prompt.clone();

        if !tools.is_empty() {
            prompt.push_str("\n\nAvailable tools:\n");
            for tool in tools {
                prompt.push_str(&format!("- {}: {}\n", tool.name(), tool.description()));
            }
        }

        prompt
    }

    /// Execute single step
    async fn execute_step(
        &self,
        messages: &mut Vec<LlmMessage>,
        tools: &[Arc<dyn Tool>],
        cancel: &CancellationToken,
    ) -> SageResult<StepResult> {
        // Check cancellation
        if cancel.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Get tool schemas
        let tool_schemas: Vec<_> = tools.iter().map(|t| t.schema()).collect();

        // Call LLM
        let response = self
            .llm_client
            .chat(messages, Some(&tool_schemas))
            .await?;

        // Check if there are tool calls
        if !response.tool_calls.is_empty() {
            // Add assistant message with tool calls
            let assistant_msg = LlmMessage {
                role: MessageRole::Assistant,
                content: response.content.clone(),
                tool_calls: Some(response.tool_calls.clone()),
                tool_call_id: None,
            cache_control: None,
                name: None,
                metadata: Default::default(),
            };
            messages.push(assistant_msg);

            // Execute tool calls
            for call in &response.tool_calls {
                let result = self.execute_tool_call(call, tools, cancel).await?;

                // Add tool result message
                let tool_msg = LlmMessage::tool(
                    result.output.unwrap_or_else(|| result.error.unwrap_or_default()),
                    call.id.clone(),
                    Some(call.name.clone()),
                );
                messages.push(tool_msg);
            }

            Ok(StepResult::Continue)
        } else {
            // No tool calls - this is the final response
            let assistant_msg = LlmMessage::assistant(&response.content);
            messages.push(assistant_msg);

            // Check if this indicates completion
            if response.finish_reason.as_deref() == Some("stop") {
                Ok(StepResult::Completed(response.content))
            } else {
                Ok(StepResult::NeedsMoreSteps)
            }
        }
    }

    /// Execute a tool call
    async fn execute_tool_call(
        &self,
        call: &ToolCall,
        tools: &[Arc<dyn Tool>],
        cancel: &CancellationToken,
    ) -> SageResult<ToolResult> {
        // Check cancellation
        if cancel.is_cancelled() {
            return Err(SageError::Cancelled);
        }

        // Find the tool
        let tool = tools
            .iter()
            .find(|t| t.name() == call.name)
            .ok_or_else(|| SageError::tool(&call.name, "Tool not found"))?;

        // Execute the tool
        let result = tool
            .execute_with_timing(call)
            .await;

        Ok(result)
    }
}

// Implement Clone for SubAgentExecutor
impl Clone for SubAgentExecutor {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            llm_client: Arc::clone(&self.llm_client),
            all_tools: self.all_tools.clone(),
            max_steps: self.max_steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::subagent::types::ToolAccessControl;
    use crate::tools::base::{Tool, ToolError};
    use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
    use async_trait::async_trait;

    /// Mock tool for testing
    struct MockTool {
        name: String,
        description: String,
    }

    impl MockTool {
        fn new(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: self.name.clone(),
                description: self.description.clone(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            }
        }

        async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::success(
                &call.id,
                &self.name,
                format!("Executed {}", self.name),
            ))
        }
    }

    fn create_test_registry() -> Arc<AgentRegistry> {
        let mut registry = AgentRegistry::new();

        registry.register(AgentDefinition {
            agent_type: AgentType::GeneralPurpose,
            name: "General".to_string(),
            description: "General purpose agent".to_string(),
            available_tools: ToolAccessControl::All,
            model: None,
            system_prompt: "You are a helpful assistant.".to_string(),
        });

        registry.register(AgentDefinition {
            agent_type: AgentType::Explore,
            name: "Explorer".to_string(),
            description: "Code exploration agent".to_string(),
            available_tools: ToolAccessControl::Specific(vec![
                "read".to_string(),
                "glob".to_string(),
            ]),
            model: None,
            system_prompt: "You are a code explorer.".to_string(),
        });

        Arc::new(registry)
    }

    #[test]
    fn test_filter_tools() {
        let registry = create_test_registry();
        let tools: Vec<Arc<dyn Tool>> = vec![
            Arc::new(MockTool::new("read", "Read files")),
            Arc::new(MockTool::new("write", "Write files")),
            Arc::new(MockTool::new("glob", "Search files")),
        ];

        // Mock LLM client (won't be used in this test)
        use crate::llm::provider_types::{LlmProvider, ModelParameters};
        use crate::config::provider::ProviderConfig;

        let llm_config = ProviderConfig {
            base_url: Some("http://localhost".to_string()),
            api_key: None,
            ..Default::default()
        };

        let model_params = ModelParameters {
            model: "test-model".to_string(),
            ..Default::default()
        };

        let llm_client = Arc::new(
            LlmClient::new(LlmProvider::OpenAI, llm_config, model_params).unwrap()
        );

        let executor = SubAgentExecutor::new(registry.clone(), llm_client, tools);

        // Test filtering for GeneralPurpose agent (should have all tools)
        let general_def = registry.get(&AgentType::GeneralPurpose).unwrap();
        let filtered = executor.filter_tools(&general_def);
        assert_eq!(filtered.len(), 3);

        // Test filtering for Explore agent (should only have read and glob)
        let explore_def = registry.get(&AgentType::Explore).unwrap();
        let filtered = executor.filter_tools(&explore_def);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name() == "read"));
        assert!(filtered.iter().any(|t| t.name() == "glob"));
        assert!(!filtered.iter().any(|t| t.name() == "write"));
    }

    #[test]
    fn test_sub_agent_config() {
        let config = SubAgentConfig::new(AgentType::Explore, "Find all Rust files")
            .with_context("Working in /src directory")
            .with_max_steps(10)
            .with_temperature(0.5);

        assert_eq!(config.agent_type, AgentType::Explore);
        assert_eq!(config.task, "Find all Rust files");
        assert_eq!(config.context, Some("Working in /src directory".to_string()));
        assert_eq!(config.max_steps, Some(10));
        assert_eq!(config.temperature, Some(0.5));
    }

    #[test]
    fn test_agent_progress() {
        let progress = AgentProgress::new(5, 10, "Processing");
        assert_eq!(progress.step, 5);
        assert_eq!(progress.max_steps, 10);
        assert_eq!(progress.percentage, 50);

        let progress = AgentProgress::new(10, 10, "Complete");
        assert_eq!(progress.percentage, 100);

        let progress = AgentProgress::new(1, 3, "Starting");
        assert_eq!(progress.percentage, 33);
    }
}
