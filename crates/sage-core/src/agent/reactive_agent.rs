//! Reactive Agent - Claude Code style execution model
//!
//! This module implements a lightweight, response-driven execution model
//! inspired by Claude Code's design philosophy.

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::providers::LLMProvider;
use crate::tools::batch_executor::BatchToolExecutor;
use crate::tools::types::{ToolCall, ToolResult};
use crate::types::{Id, TaskMetadata};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use uuid::Uuid;

/// Token usage tracking across all steps
#[derive(Debug, Default)]
pub struct TokenUsage {
    /// Total input tokens consumed
    pub input_tokens: AtomicU64,
    /// Total output tokens consumed
    pub output_tokens: AtomicU64,
}

impl TokenUsage {
    /// Create a new token usage tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Add token usage from a single step
    pub fn add(&self, input: u64, output: u64) {
        self.input_tokens.fetch_add(input, Ordering::Relaxed);
        self.output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    /// Get total tokens (input + output)
    pub fn total(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed) + self.output_tokens.load(Ordering::Relaxed)
    }

    /// Get input tokens
    pub fn input(&self) -> u64 {
        self.input_tokens.load(Ordering::Relaxed)
    }

    /// Get output tokens
    pub fn output(&self) -> u64 {
        self.output_tokens.load(Ordering::Relaxed)
    }

    /// Check if budget is exceeded
    pub fn is_budget_exceeded(&self, budget: Option<u64>) -> bool {
        if let Some(limit) = budget {
            self.total() >= limit
        } else {
            false
        }
    }

    /// Get remaining budget
    pub fn remaining(&self, budget: Option<u64>) -> Option<u64> {
        budget.map(|limit| limit.saturating_sub(self.total()))
    }
}

/// Response-driven agent execution result
#[derive(Debug, Clone)]
pub struct ReactiveResponse {
    /// Unique response ID
    pub id: Id,
    /// User's original request
    pub request: String,
    /// AI's text response
    pub content: String,
    /// Tool calls executed (if any)
    pub tool_calls: Vec<ToolCall>,
    /// Tool execution results
    pub tool_results: Vec<ToolResult>,
    /// Execution duration
    pub duration: std::time::Duration,
    /// Whether the task is completed
    pub completed: bool,
    /// Optional continuation prompt for multi-turn interactions
    pub continuation_prompt: Option<String>,
}

/// Reactive agent trait - simplified Claude Code style interface
#[async_trait]
pub trait ReactiveAgent: Send + Sync {
    /// Process a user request and return a response
    async fn process_request(&mut self, request: &str, context: Option<TaskMetadata>) -> SageResult<ReactiveResponse>;
    
    /// Continue a conversation with additional context
    async fn continue_conversation(&mut self, previous: &ReactiveResponse, additional_input: &str) -> SageResult<ReactiveResponse>;
    
    /// Get agent configuration
    fn config(&self) -> &Config;
}

/// Claude Code style reactive agent implementation
pub struct ClaudeStyleAgent {
    #[allow(dead_code)]
    id: Id,
    config: Config,
    llm_client: LLMClient,
    batch_executor: BatchToolExecutor,
    conversation_history: Vec<LLMMessage>,
    /// Token usage tracking
    token_usage: TokenUsage,
    /// Current step count
    current_step: u32,
}

impl ClaudeStyleAgent {
    /// Create a new Claude-style agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Initialize LLM client
        let default_params = config.default_model_parameters()?;
        let provider_name = config.get_default_provider();
        
        let provider: LLMProvider = provider_name.parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))?;
            
        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeout(60)
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }
            
        let model_params = default_params.to_llm_parameters();
        let llm_client = LLMClient::new(provider, provider_config, model_params)?;
        
        // Initialize batch tool executor
        let batch_executor = BatchToolExecutor::new();
        
        Ok(Self {
            id: Uuid::new_v4(),
            config,
            llm_client,
            batch_executor,
            conversation_history: Vec::new(),
            token_usage: TokenUsage::new(),
            current_step: 0,
        })
    }
    
    /// Create system message for Claude Code style interaction
    fn create_system_message(&self, context: Option<&TaskMetadata>) -> LLMMessage {
        let context_info = if let Some(ctx) = context {
            format!(
                "\n# Current Task Context\n{}\n# Working Directory\n{}\n",
                ctx.description,
                ctx.working_dir
            )
        } else {
            String::new()
        };
        
        let system_prompt = format!(
            r#"# Role
You are Sage Agent, an agentic coding AI assistant with access to the developer's codebase.
You can read from and write to the codebase using the provided tools.

# Identity  
You are Sage Agent developed by Sage Code, based on advanced language models.
{}
# Response Style
- Be concise and direct
- Provide actionable responses
- Use batch tool calls for efficiency
- Avoid unnecessary explanations unless requested
- Focus on solving the immediate problem

# Tool Usage Strategy
- Use multiple tools concurrently when possible
- Perform speculative searches to gather comprehensive information
- Batch related operations for efficiency
- Prefer reading multiple relevant files simultaneously

# Available Tools
{}

# Execution Philosophy
Execute tools intelligently and concurrently. When you need information, 
gather it comprehensively in a single response rather than making multiple 
sequential requests."#,
            context_info,
            self.get_tools_description()
        );
        
        LLMMessage::system(system_prompt)
    }
    
    /// Get description of available tools
    fn get_tools_description(&self) -> String {
        let schemas = self.batch_executor.get_tool_schemas();
        schemas
            .iter()
            .map(|schema| format!("- {}: {}", schema.name, schema.description))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if we can continue execution (budget and step limits)
    fn can_continue(&self) -> Result<(), SageError> {
        // Check step limit
        if self.current_step >= self.config.max_steps {
            return Err(SageError::agent(format!(
                "Max steps ({}) reached. Total tokens used: {} (input: {}, output: {})",
                self.config.max_steps,
                self.token_usage.total(),
                self.token_usage.input(),
                self.token_usage.output()
            )));
        }

        // Check token budget
        if self.token_usage.is_budget_exceeded(self.config.total_token_budget) {
            return Err(SageError::agent(format!(
                "Token budget ({}) exceeded. Total tokens used: {} (input: {}, output: {})",
                self.config.total_token_budget.unwrap_or(0),
                self.token_usage.total(),
                self.token_usage.input(),
                self.token_usage.output()
            )));
        }

        Ok(())
    }

    /// Get current token usage
    pub fn get_token_usage(&self) -> (u64, u64, u64) {
        (
            self.token_usage.input(),
            self.token_usage.output(),
            self.token_usage.total(),
        )
    }

    /// Get remaining token budget
    pub fn get_remaining_budget(&self) -> Option<u64> {
        self.token_usage.remaining(self.config.total_token_budget)
    }

    /// Get current step count
    pub fn get_current_step(&self) -> u32 {
        self.current_step
    }
    
    /// Execute a single request-response cycle
    async fn execute_single_turn(&mut self, request: &str, context: Option<&TaskMetadata>) -> SageResult<ReactiveResponse> {
        // Check if we can continue (step and budget limits)
        self.can_continue()?;

        let start_time = Instant::now();
        let response_id = Uuid::new_v4();

        // Increment step counter
        self.current_step += 1;

        // Build conversation messages
        let mut messages = vec![self.create_system_message(context)];
        messages.extend(self.conversation_history.clone());
        messages.push(LLMMessage::user(request));

        // Get tool schemas
        let tool_schemas = self.batch_executor.get_tool_schemas();

        // Call LLM with tools
        let llm_response = self.llm_client.chat(&messages, Some(&tool_schemas)).await?;

        // Track token usage from LLM response
        if let Some(usage) = &llm_response.usage {
            self.token_usage.add(
                usage.prompt_tokens as u64,
                usage.completion_tokens as u64,
            );
        }
        
        // Update conversation history
        let mut assistant_msg = LLMMessage::assistant(&llm_response.content);
        if !llm_response.tool_calls.is_empty() {
            assistant_msg.tool_calls = Some(llm_response.tool_calls.clone());
        }
        self.conversation_history.push(LLMMessage::user(request));
        self.conversation_history.push(assistant_msg);
        
        // Execute tools if present (batch execution)
        let tool_results = if !llm_response.tool_calls.is_empty() {
            self.batch_executor.execute_batch(&llm_response.tool_calls).await
        } else {
            Vec::new()
        };
        
        // Add tool results to conversation history
        if !tool_results.is_empty() {
            for result in &tool_results {
                let content = if result.success {
                    result.output.as_deref().unwrap_or("")
                } else {
                    &format!("Error: {}", result.error.as_deref().unwrap_or("Unknown error"))
                };
                self.conversation_history.push(LLMMessage::user(content));
            }
        }
        
        // Determine if task is completed
        let completed = llm_response.indicates_completion() || 
                      tool_results.iter().any(|r| r.tool_name == "task_done");
        
        // Generate continuation prompt if needed
        let continuation_prompt = if !completed && !tool_results.is_empty() {
            Some("Continue with the next step based on the tool results.".to_string())
        } else {
            None
        };
        
        Ok(ReactiveResponse {
            id: response_id,
            request: request.to_string(),
            content: llm_response.content,
            tool_calls: llm_response.tool_calls,
            tool_results,
            duration: start_time.elapsed(),
            completed,
            continuation_prompt,
        })
    }
    
    /// Keep conversation history manageable
    fn trim_conversation_history(&mut self) {
        const MAX_HISTORY_LENGTH: usize = 20; // Keep last 20 messages
        
        if self.conversation_history.len() > MAX_HISTORY_LENGTH {
            let keep_from = self.conversation_history.len() - MAX_HISTORY_LENGTH;
            self.conversation_history = self.conversation_history[keep_from..].to_vec();
        }
    }
}

#[async_trait]
impl ReactiveAgent for ClaudeStyleAgent {
    async fn process_request(&mut self, request: &str, context: Option<TaskMetadata>) -> SageResult<ReactiveResponse> {
        // Clear history for new request if context indicates new task
        if context.is_some() {
            self.conversation_history.clear();
        }
        
        self.execute_single_turn(request, context.as_ref()).await
    }
    
    async fn continue_conversation(&mut self, _previous: &ReactiveResponse, additional_input: &str) -> SageResult<ReactiveResponse> {
        // Trim history to prevent context overflow
        self.trim_conversation_history();
        
        self.execute_single_turn(additional_input, None).await
    }
    
    fn config(&self) -> &Config {
        &self.config
    }
}

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
        let response = self.agent.process_request(&current_request, context.take()).await?;
        let completed = response.completed;
        responses.push(response);
        
        // Continue if not completed and there's a continuation prompt
        if !completed {
            if let Some(continuation) = &responses.last().unwrap().continuation_prompt {
                let follow_up = self.agent.continue_conversation(responses.last().unwrap(), continuation).await?;
                responses.push(follow_up);
            }
        }
        
        Ok(responses)
    }
    
    /// Interactive conversation mode
    pub async fn interactive_mode(&mut self, initial_request: &str) -> SageResult<ReactiveResponse> {
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
        
        self.agent.continue_conversation(&dummy_previous, user_input).await
    }
}