//! Core sub-agent executor implementation

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::super::registry::AgentRegistry;
use super::super::types::{AgentDefinition, SubAgentResult};
use super::handlers::StepExecutor;
use super::types::{ExecutorMessage, StepResult, SubAgentConfig};
use crate::error::{SageError, SageResult};
use crate::llm::client::LlmClient;
use crate::llm::messages::LlmMessage;
use crate::tools::base::Tool;
use crate::types::LlmUsage;

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
        let mut messages = self.build_initial_messages(&definition, &config);

        // Determine max steps
        let max_steps = config.max_steps.unwrap_or(self.max_steps);

        // Track execution
        let mut steps_taken = 0;
        let mut tool_calls_count = 0;
        let mut total_usage = LlmUsage::default();

        // Create step executor
        let step_executor = StepExecutor::new(Arc::clone(&self.llm_client));

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
            let step_result = step_executor.execute_step(&mut messages, &tools, &cancel).await?;

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
    pub(super) fn filter_tools(&self, definition: &AgentDefinition) -> Vec<Arc<dyn Tool>> {
        self.all_tools
            .iter()
            .filter(|tool| definition.can_use_tool(tool.name()))
            .cloned()
            .collect()
    }

    /// Build initial messages for agent
    fn build_initial_messages(
        &self,
        definition: &AgentDefinition,
        config: &SubAgentConfig,
    ) -> Vec<LlmMessage> {
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

        messages
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
