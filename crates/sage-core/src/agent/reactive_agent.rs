//! Reactive Agent - Claude Code style execution model
//!
//! This module implements a lightweight, response-driven execution model
//! inspired by Claude Code's design philosophy.

use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::providers::LLMProvider;
use crate::prompts::SystemPromptBuilder;
use crate::tools::batch_executor::BatchToolExecutor;
use crate::tools::types::{ToolCall, ToolResult};
use crate::types::{Id, TaskMetadata};
use anyhow::Context;
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
    async fn process_request(
        &mut self,
        request: &str,
        context: Option<TaskMetadata>,
    ) -> SageResult<ReactiveResponse>;

    /// Continue a conversation with additional context
    async fn continue_conversation(
        &mut self,
        previous: &ReactiveResponse,
        additional_input: &str,
    ) -> SageResult<ReactiveResponse>;

    /// Get agent configuration
    fn config(&self) -> &Config;
}

/// Tracks file operations for task completion verification
#[derive(Debug, Default, Clone)]
struct FileOperationTracker {
    /// Files created via Write tool
    pub created_files: Vec<String>,
    /// Files modified via Edit tool
    pub modified_files: Vec<String>,
}

impl FileOperationTracker {
    fn new() -> Self {
        Self::default()
    }

    fn track_tool_call(&mut self, tool_name: &str, result: &ToolResult) {
        if !result.success {
            return;
        }

        match tool_name {
            "Write" => {
                if let Some(file_path) = result.metadata.get("file_path") {
                    if let Some(path) = file_path.as_str() {
                        self.created_files.push(path.to_string());
                    }
                }
            }
            "Edit" => {
                if let Some(file_path) = result.metadata.get("file_path") {
                    if let Some(path) = file_path.as_str() {
                        self.modified_files.push(path.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn has_file_operations(&self) -> bool {
        !self.created_files.is_empty() || !self.modified_files.is_empty()
    }

    fn reset(&mut self) {
        self.created_files.clear();
        self.modified_files.clear();
    }
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
    /// File operation tracker for completion verification
    file_tracker: FileOperationTracker,
}

impl ClaudeStyleAgent {
    /// Create a new Claude-style agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Initialize LLM client
        let default_params = config.default_model_parameters()
            .context("Failed to retrieve default model parameters from configuration")?;
        let provider_name = config.get_default_provider();

        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))
            .context(format!("Failed to parse provider name '{}' into a valid LLM provider", provider_name))?;

        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeout(60)
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        let model_params = default_params.to_llm_parameters();
        let llm_client = LLMClient::new(provider, provider_config, model_params)
            .context(format!("Failed to create LLM client for provider: {}", provider_name))?;

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
            file_tracker: FileOperationTracker::new(),
        })
    }

    /// Create system message for Claude Code style interaction using modular prompt system
    fn create_system_message(&self, context: Option<&TaskMetadata>) -> LLMMessage {
        // Get tool schemas
        let tool_schemas = self.batch_executor.get_tool_schemas();

        // Extract context info
        let (task_desc, working_dir) = if let Some(ctx) = context {
            (ctx.description.clone(), ctx.working_dir.clone())
        } else {
            ("General assistance".to_string(), ".".to_string())
        };

        // Check if working directory is a git repo
        let is_git_repo = std::path::Path::new(&working_dir).join(".git").exists();

        // Get current git branch if in a git repo
        let (git_branch, main_branch) = if is_git_repo {
            let branch = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&working_dir)
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "main".to_string());
            (branch, "main".to_string())
        } else {
            ("main".to_string(), "main".to_string())
        };

        // Get platform info
        let platform = std::env::consts::OS.to_string();
        let os_version = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Build system prompt using the new modular system
        let system_prompt = SystemPromptBuilder::new()
            .with_agent_name("Sage Agent")
            .with_agent_version(env!("CARGO_PKG_VERSION"))
            .with_task(&task_desc)
            .with_working_dir(&working_dir)
            .with_git_info(is_git_repo, &git_branch, &main_branch)
            .with_platform(&platform, &os_version)
            .with_tools(tool_schemas)
            .with_git_instructions(is_git_repo)
            .with_security_policy(true)
            .build();

        LLMMessage::system(system_prompt)
    }

    /// Get tool schemas from the batch executor
    pub fn get_tool_schemas(&self) -> Vec<crate::tools::types::ToolSchema> {
        self.batch_executor.get_tool_schemas()
    }

    /// Check if we can continue execution (budget and step limits)
    fn can_continue(&self) -> Result<(), SageError> {
        // Check step limit (None = unlimited)
        if let Some(max_steps) = self.config.max_steps {
            if self.current_step >= max_steps {
                return Err(SageError::agent(format!(
                    "Max steps ({}) reached. Total tokens used: {} (input: {}, output: {})",
                    max_steps,
                    self.token_usage.total(),
                    self.token_usage.input(),
                    self.token_usage.output()
                )));
            }
        }

        // Check token budget
        if self
            .token_usage
            .is_budget_exceeded(self.config.total_token_budget)
        {
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
    async fn execute_single_turn(
        &mut self,
        request: &str,
        context: Option<&TaskMetadata>,
    ) -> SageResult<ReactiveResponse> {
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
            self.token_usage
                .add(usage.prompt_tokens as u64, usage.completion_tokens as u64);
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
            self.batch_executor
                .execute_batch(&llm_response.tool_calls)
                .await
        } else {
            Vec::new()
        };

        // Add tool results to conversation history and track file operations
        if !tool_results.is_empty() {
            for result in &tool_results {
                // Track file operations for completion verification
                self.file_tracker.track_tool_call(&result.tool_name, result);

                let content = if result.success {
                    result.output.as_deref().unwrap_or("")
                } else {
                    &format!(
                        "Error: {}",
                        result.error.as_deref().unwrap_or("Unknown error")
                    )
                };
                self.conversation_history.push(LLMMessage::user(content));
            }
        }

        // Determine if task is completed
        // Check if task_done was called
        let task_done_called = llm_response.indicates_completion()
            || tool_results.iter().any(|r| r.tool_name == "task_done");

        // If task_done is called but no file operations were performed,
        // check if this looks like a documentation-only completion
        let completed = if task_done_called {
            // Allow completion if there were file operations OR
            // if the task explicitly doesn't require code (research/analysis tasks)
            // For now, we log a warning but allow completion
            // TODO: Add stricter validation based on task type
            if !self.file_tracker.has_file_operations() {
                tracing::warn!(
                    "Task marked as complete but no file operations were performed. \
                     Created files: {:?}, Modified files: {:?}",
                    self.file_tracker.created_files,
                    self.file_tracker.modified_files
                );
            }
            true
        } else {
            false
        };

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
    async fn process_request(
        &mut self,
        request: &str,
        context: Option<TaskMetadata>,
    ) -> SageResult<ReactiveResponse> {
        // Clear history for new request if context indicates new task
        if context.is_some() {
            self.conversation_history.clear();
        }

        self.execute_single_turn(request, context.as_ref()).await
    }

    async fn continue_conversation(
        &mut self,
        _previous: &ReactiveResponse,
        additional_input: &str,
    ) -> SageResult<ReactiveResponse> {
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
        let response = self
            .agent
            .process_request(&current_request, context.take())
            .await?;
        let completed = response.completed;
        responses.push(response);

        // Continue if not completed and there's a continuation prompt
        if !completed {
            if let Some(continuation) = &responses.last().unwrap().continuation_prompt {
                let follow_up = self
                    .agent
                    .continue_conversation(responses.last().unwrap(), continuation)
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
}
