//! Base agent implementation

use crate::agent::{AgentExecution, AgentState, AgentStep, ExecutionError, ExecutionOutcome};
use crate::config::model::Config;
use crate::error::{SageError, SageResult};
use crate::interrupt::{global_interrupt_manager, reset_global_interrupt_manager};
use crate::llm::client::LLMClient;
use crate::llm::messages::LLMMessage;
use crate::llm::providers::LLMProvider;
use crate::tools::executor::ToolExecutor;
use crate::tools::types::ToolSchema;
use crate::trajectory::recorder::TrajectoryRecorder;
use crate::types::{Id, TaskMetadata};
use crate::ui::animation::AnimationState;
use crate::ui::{AnimationManager, DisplayManager};
use async_trait::async_trait;
use colored::*;
use std::sync::Arc;
use tokio::select;
use tokio::sync::Mutex;

/// Model identity information for system prompt
#[derive(Debug, Clone)]
struct ModelIdentity {
    base_model_info: String,
    model_name: String,
}

/// Base agent trait
#[async_trait]
pub trait Agent: Send + Sync {
    /// Execute a task and return an explicit outcome
    ///
    /// Returns `ExecutionOutcome` which clearly indicates success, failure,
    /// interruption, or max steps reached, while preserving the full execution trace.
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome>;

    /// Continue an existing execution with new user message
    async fn continue_execution(
        &mut self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()>;

    /// Get the agent's configuration
    fn config(&self) -> &Config;

    /// Get the agent's ID
    fn id(&self) -> Id;
}

/// Base agent implementation
pub struct BaseAgent {
    id: Id,
    config: Config,
    llm_client: LLMClient,
    tool_executor: ToolExecutor,
    trajectory_recorder: Option<Arc<Mutex<TrajectoryRecorder>>>,
    max_steps: u32,
    animation_manager: AnimationManager,
}

impl BaseAgent {
    /// Check if content contains markdown formatting
    pub fn is_markdown_content(content: &str) -> bool {
        // Simple heuristics to detect markdown content
        content.contains("# ") ||           // Headers
        content.contains("## ") ||          // Headers
        content.contains("### ") ||         // Headers
        content.contains("* ") ||           // Lists
        content.contains("- ") ||           // Lists
        content.contains("```") ||          // Code blocks
        content.contains("`") ||            // Inline code
        content.contains("**") ||           // Bold
        content.contains("*") ||            // Italic
        content.contains("[") && content.contains("](") || // Links
        content.contains("> ") ||           // Blockquotes
        content.lines().count() > 3 // Multi-line content is likely markdown
    }

    /// Create a new base agent
    pub fn new(config: Config) -> SageResult<Self> {
        // Get default provider configuration
        let default_params = config.default_model_parameters()?;
        let provider_name = config.get_default_provider();

        // Debug logging
        tracing::info!("Creating agent with provider: {}", provider_name);
        tracing::info!("Model: {}", default_params.model);
        tracing::info!("API key set: {}", default_params.api_key.is_some());

        // Parse provider
        let provider: LLMProvider = provider_name
            .parse()
            .map_err(|_| SageError::config(format!("Invalid provider: {}", provider_name)))?;

        tracing::info!("Parsed provider: {:?}", provider);

        // Create provider config
        let mut provider_config = crate::config::provider::ProviderConfig::new(provider_name)
            .with_api_key(default_params.get_api_key().unwrap_or_default())
            .with_timeout(60)
            .with_max_retries(3);

        // Apply custom base_url if configured (for OpenRouter, etc.)
        if let Some(base_url) = &default_params.base_url {
            provider_config = provider_config.with_base_url(base_url.clone());
        }

        // Create model parameters
        let model_params = default_params.to_llm_parameters();

        // Create LLM client
        let llm_client = LLMClient::new(provider, provider_config, model_params)?;

        // Create tool executor
        let tool_executor = ToolExecutor::new();

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            config,
            llm_client,
            tool_executor,
            trajectory_recorder: None,
            max_steps: 20,
            animation_manager: AnimationManager::new(),
        })
    }

    /// Set trajectory recorder
    pub fn set_trajectory_recorder(&mut self, recorder: Arc<Mutex<TrajectoryRecorder>>) {
        self.trajectory_recorder = Some(recorder);
    }

    /// Set tool executor
    pub fn set_tool_executor(&mut self, executor: ToolExecutor) {
        self.tool_executor = executor;
    }

    /// Set max steps
    pub fn set_max_steps(&mut self, max_steps: u32) {
        self.max_steps = max_steps;
    }

    // Note: Display methods moved to CLI layer for better separation of concerns

    /// Get model identity information based on current configuration
    fn get_model_identity(&self) -> ModelIdentity {
        let default_provider = self.config.get_default_provider();
        let default_params = crate::config::model::ModelParameters::default();
        let model_params = self
            .config
            .default_model_parameters()
            .unwrap_or(&default_params);

        match default_provider {
            "anthropic" => {
                let base_model_info = match model_params.model.as_str() {
                    "claude-3-sonnet-20240229" => "The base model is Claude 3 Sonnet by Anthropic.",
                    "claude-3-opus-20240229" => "The base model is Claude 3 Opus by Anthropic.",
                    "claude-3-haiku-20240307" => "The base model is Claude 3 Haiku by Anthropic.",
                    "claude-sonnet-4-20250514" => "The base model is Claude Sonnet 4 by Anthropic.",
                    _ => "The base model is Claude by Anthropic.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by Anthropic", model_params.model),
                }
            }
            "openai" => {
                let base_model_info = match model_params.model.as_str() {
                    "gpt-4" => "The base model is GPT-4 by OpenAI.",
                    "gpt-4-turbo" => "The base model is GPT-4 Turbo by OpenAI.",
                    "gpt-3.5-turbo" => "The base model is GPT-3.5 Turbo by OpenAI.",
                    _ => "The base model is GPT by OpenAI.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by OpenAI", model_params.model),
                }
            }
            "google" => {
                let base_model_info = match model_params.model.as_str() {
                    "gemini-2.5-pro" => "The base model is Gemini 2.5 Pro by Google.",
                    "gemini-1.5-pro" => "The base model is Gemini 1.5 Pro by Google.",
                    "gemini-1.0-pro" => "The base model is Gemini 1.0 Pro by Google.",
                    _ => "The base model is Gemini by Google.",
                };
                ModelIdentity {
                    base_model_info: base_model_info.to_string(),
                    model_name: format!("{} by Google", model_params.model),
                }
            }
            _ => ModelIdentity {
                base_model_info: "The base model information is not available.".to_string(),
                model_name: model_params.model.clone(),
            },
        }
    }

    /// Create initial system message
    fn create_system_message(&self, task: &TaskMetadata) -> LLMMessage {
        // Get current model info for the identity section
        let model_info = self.get_model_identity();

        let system_prompt = format!(
            r#"# Role
You are Sage Agent developed by Sage Code, an agentic coding AI assistant with access to the developer's codebase through Sage's world-leading context engine and integrations.
You can read from and write to the codebase using the provided tools.
The current date is 2025-07-14.

# Identity
Here is some information about Sage Agent in case the person asks:
{}
You are Sage Agent developed by Sage Code, an agentic coding AI assistant based on the {} model, with access to the developer's codebase through Sage's world-leading context engine and integrations.

# Current Task
{}

# Working Directory
{}

# Available Tools
{}

# Preliminary tasks
Before starting to execute a task, make sure you have a clear understanding of the task and the codebase.
Use information-gathering tools efficiently and progressively:

## Efficient Information Gathering Strategy
1. **For understanding a project**: Start with directory structure, then key documentation
   - Use `str_replace_based_edit_tool` with `view` action to see directory structure
   - Read README.md, package.json, Cargo.toml, or similar configuration files
   - Avoid commands like `ls -R` that produce excessive output

2. **For finding specific code**: Use targeted searches
   - Use `codebase-retrieval` tool for semantic code search
   - Use `str_replace_based_edit_tool` with specific file paths
   - Use `bash` tool with focused commands like `find`, `grep` with limits

3. **Avoid information overload**:
   - Never use `ls -R` or similar recursive commands without filters
   - Limit bash command outputs (use `head`, `tail`, `grep` with line limits)
   - Read files progressively rather than dumping entire codebases

# Planning and Task Management
You have access to task management tools that can help organize complex work. Consider using these tools when:
- The user explicitly requests planning, task breakdown, or project organization
- You're working on complex multi-step tasks that would benefit from structured planning
- The user mentions wanting to track progress or see next steps
- You need to coordinate multiple related changes across the codebase

When task management would be helpful:
1. Once you have performed preliminary rounds of information-gathering, extremely detailed plan for the actions you want to take.
   - Be sure to be careful and exhaustive.
   - Feel free to think about in a chain of thought first.
   - If you need more information during planning, feel free to perform more information-gathering steps
   - Ensure each sub task represents a meaningful unit of work that would take a professional developer approximately 20 minutes to complete. Avoid overly granular tasks that represent single actions
2. If the request requires breaking down work or organizing tasks, use the appropriate task management tools:
   - Use `add_tasks` to create individual new tasks or subtasks
   - Use `update_tasks` to modify existing task properties (state, name, description):
     * For single task updates: {{"task_id": "abc", "state": "COMPLETE"}}
     * For multiple task updates: {{"tasks": [{{"task_id": "abc", "state": "COMPLETE"}}, {{"task_id": "def", "state": "IN_PROGRESS"}}]}}
     * **Always use batch updates when updating multiple tasks** (e.g., marking current task complete and next task in progress)
   - Use `reorganize_tasklist` only for complex restructuring that affects many tasks at once
3. When using task management, update task states efficiently:
   - When starting work on a new task, use a single `update_tasks` call to mark the previous task complete and the new task in progress
   - Use batch updates: {{"tasks": [{{"task_id": "previous-task", "state": "COMPLETE"}}, {{"task_id": "current-task", "state": "IN_PROGRESS"}}]}}
   - If user feedback indicates issues with a previously completed solution, update that task back to IN_PROGRESS and work on addressing the feedback
   - Here are the task states and their meanings:
       - `[ ]` = Not started (for tasks you haven't begun working on yet)
       - `[/]` = In progress (for tasks you're currently working on)
       - `[-]` = Cancelled (for tasks that are no longer relevant)
       - `[x]` = Completed (for tasks the user has confirmed are complete)

# Making edits
When making edits, use the str_replace_based_edit_tool - do NOT just write a new file.
Before calling the str_replace_based_edit_tool, ALWAYS first call the codebase-retrieval tool
asking for highly detailed information about the code you want to edit.
Ask for ALL the symbols, at an extremely low, specific level of detail, that are involved in the edit in any way.
Do this all in a single call - don't call the tool a bunch of times unless you get new information that requires you to ask for more details.
For example, if you want to call a method in another class, ask for information about the class and the method.
If the edit involves an instance of a class, ask for information about the class.
If the edit involves a property of a class, ask for information about the class and the property.
If several of the above apply, ask for all of them in a single call.
When in any doubt, include the symbol or object.
When making changes, be very conservative and respect the codebase.

# Following instructions
Focus on doing what the user asks you to do.
Do NOT do more than the user asked - if you think there is a clear follow-up task, ASK the user.
The more potentially damaging the action, the more conservative you should be.
For example, do NOT perform any of these actions without explicit permission from the user:
- Committing or pushing code
- Changing the status of a ticket
- Merging a branch
- Installing dependencies
- Deploying code

# Testing
You are very good at writing unit tests and making them work. If you write
code, suggest to the user to test the code by writing tests and running them.
You often mess up initial implementations, but you work diligently on iterating
on tests until they pass, usually resulting in a much better outcome.
Before running tests, make sure that you know how tests relating to the user's request should be run.

# Displaying code
When showing the user code from existing file, don't wrap it in normal markdown ```.
Instead, ALWAYS wrap code you want to show the user in `<sage_code_snippet>` and  `</sage_code_snippet>`  XML tags.
Provide both `path=` and `mode="EXCERPT"` attributes to the tag.
Use four backticks (````) instead of three.

Example:
<sage_code_snippet path="foo/bar.py" mode="EXCERPT">
````python
class AbstractTokenizer():
    def __init__(self, name):
        self.name = name
    ...
````
</sage_code_snippet>

If you fail to wrap code in this way, it will not be visible to the user.
BE VERY BRIEF BY ONLY PROVIDING <10 LINES OF THE CODE. If you give correct XML structure, it will be parsed into a clickable code block, and the user can always click it to see the part in the full file.

# Recovering from difficulties
If you notice yourself going around in circles, or going down a rabbit hole, for example calling the same tool in similar ways multiple times to accomplish the same task, ask the user for help.

# Final
If you've been using task management during this conversation:
1. Reason about the overall progress and whether the original goal is met or if further steps are needed.
2. Consider reviewing the Current Task List using `view_tasklist` to check status.
3. If further changes, new tasks, or follow-up actions are identified, you may use `update_tasks` to reflect these in the task list.
4. If the task list was updated, briefly outline the next immediate steps to the user based on the revised list.
If you have made code edits, always suggest writing or updating tests and executing those tests to make sure the changes are correct.

## CRITICAL: Task Completion Rules

**ALWAYS call `task_done` when you have completed the user's request!**

Remember: Respond appropriately to the type of request. Simple questions don't need complex workflows!"#,
            model_info.base_model_info,
            model_info.model_name,
            task.description,
            task.working_dir,
            self.get_tools_description()
        );

        LLMMessage::system(system_prompt)
    }

    /// Get description of available tools
    fn get_tools_description(&self) -> String {
        let schemas = self.tool_executor.get_tool_schemas();
        schemas
            .iter()
            .map(|schema| format!("- {}: {}", schema.name, schema.description))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Execute a single step
    async fn execute_step(
        &mut self,
        step_number: u32,
        messages: &[LLMMessage],
        tools: &[ToolSchema],
    ) -> SageResult<AgentStep> {
        // Print step separator
        DisplayManager::print_separator(&format!("Step {} - AI Thinking", step_number), "blue");

        let mut step = AgentStep::new(step_number, AgentState::Thinking);

        // Get LLM response with timing and animation
        let start_time = std::time::Instant::now();

        // Start thinking animation
        self.animation_manager
            .start_animation(AnimationState::Thinking, "🤖 Thinking", "blue")
            .await;

        // Get cancellation token for interrupt handling
        let cancellation_token = global_interrupt_manager()
            .lock()
            .map_err(|_| SageError::agent("Failed to acquire interrupt manager lock"))?
            .cancellation_token();

        // Execute LLM call with interrupt support
        let llm_response = select! {
            response = self.llm_client.chat(messages, Some(tools)) => {
                response?
            }
            _ = cancellation_token.cancelled() => {
                // Stop animation on interruption
                self.animation_manager.stop_animation().await;
                return Err(SageError::agent("Task interrupted during LLM call"));
            }
        };

        let api_duration = start_time.elapsed();

        // Stop animation and show timing
        self.animation_manager.stop_animation().await;
        DisplayManager::print_timing("🤖 AI Response", api_duration);

        step = step.with_llm_response(llm_response.clone());

        // Record LLM interaction in trajectory
        if let Some(recorder) = &self.trajectory_recorder {
            let input_messages: Vec<serde_json::Value> = messages
                .iter()
                .map(|msg| serde_json::to_value(msg).unwrap_or_default())
                .collect();

            let response_record = crate::trajectory::recorder::LLMResponseRecord {
                content: llm_response.content.clone(),
                model: llm_response.model.clone(),
                finish_reason: llm_response.finish_reason.clone(),
                usage: llm_response.usage.as_ref().map(|u| {
                    crate::trajectory::recorder::TokenUsageRecord {
                        input_tokens: u.prompt_tokens,
                        output_tokens: u.completion_tokens,
                        cache_creation_input_tokens: None,
                        cache_read_input_tokens: None,
                        reasoning_tokens: None,
                    }
                }),
                tool_calls: if llm_response.tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        llm_response
                            .tool_calls
                            .iter()
                            .map(|tc| serde_json::to_value(tc).unwrap_or_default())
                            .collect(),
                    )
                },
            };

            let tools_available: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();

            // Clone the recorder to avoid holding the reference across await
            let recorder_clone = recorder.clone();
            recorder_clone
                .lock()
                .await
                .record_llm_interaction(
                    provider,
                    model,
                    input_messages,
                    response_record,
                    Some(tools_available),
                )
                .await?;
        }

        // Show AI response with markdown rendering
        if !llm_response.content.is_empty() {
            if Self::is_markdown_content(&llm_response.content) {
                println!("\n🤖 AI Response:");
                DisplayManager::print_markdown(&llm_response.content);
            } else {
                println!("\n🤖 {}", llm_response.content.trim());
            }
        }

        // Check if there are tool calls
        if !llm_response.tool_calls.is_empty() {
            step.state = AgentState::ToolExecution;

            // Print tool execution separator
            DisplayManager::print_separator("Tool Execution", "cyan");

            // Show and execute tools
            for tool_call in &llm_response.tool_calls {
                // Show what the tool is doing
                let action = match tool_call.name.as_str() {
                    "bash" => {
                        if let Some(command) = tool_call.arguments.get("command") {
                            let cmd_str = command.as_str().unwrap_or("");
                            if cmd_str.chars().count() > 60 {
                                let truncated: String = cmd_str.chars().take(57).collect();
                                format!("🖥️  {}...", truncated)
                            } else {
                                format!("🖥️  {}", cmd_str)
                            }
                        } else {
                            "🖥️  Running command".to_string()
                        }
                    }
                    "str_replace_based_edit_tool" => {
                        if let Some(action) = tool_call.arguments.get("action") {
                            match action.as_str().unwrap_or("") {
                                "view" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("📖 Reading: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "📖 Reading file".to_string()
                                    }
                                }
                                "create" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        let content_preview = if let Some(content) =
                                            tool_call.arguments.get("file_text")
                                        {
                                            let content_str = content.as_str().unwrap_or("");
                                            if content_str.len() > 50 {
                                                format!(" ({}...)", &content_str[..47])
                                            } else if !content_str.is_empty() {
                                                format!(" ({})", content_str)
                                            } else {
                                                "".to_string()
                                            }
                                        } else {
                                            "".to_string()
                                        };
                                        format!(
                                            "📝 Creating: {}{}",
                                            path.as_str().unwrap_or(""),
                                            content_preview
                                        )
                                    } else {
                                        "📝 Creating file".to_string()
                                    }
                                }
                                "str_replace" => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("✏️ Editing: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "✏️ Editing file".to_string()
                                    }
                                }
                                _ => {
                                    if let Some(path) = tool_call.arguments.get("path") {
                                        format!("📄 File op: {}", path.as_str().unwrap_or(""))
                                    } else {
                                        "📄 File operation".to_string()
                                    }
                                }
                            }
                        } else {
                            "📄 File operation".to_string()
                        }
                    }
                    "task_done" => {
                        if let Some(summary) = tool_call.arguments.get("summary") {
                            let summary_str = summary.as_str().unwrap_or("");
                            if summary_str.chars().count() > 50 {
                                let truncated: String = summary_str.chars().take(47).collect();
                                format!("✅ Done: {}...", truncated)
                            } else {
                                format!("✅ Done: {}", summary_str)
                            }
                        } else {
                            "✅ Task completed".to_string()
                        }
                    }
                    "sequentialthinking" => {
                        if let Some(thought) = tool_call.arguments.get("thought") {
                            let thought_str = thought.as_str().unwrap_or("");
                            if thought_str.chars().count() > 50 {
                                let truncated: String = thought_str.chars().take(47).collect();
                                format!("🧠 Thinking: {}...", truncated)
                            } else {
                                format!("🧠 Thinking: {}", thought_str)
                            }
                        } else {
                            "🧠 Thinking step by step".to_string()
                        }
                    }
                    "json_edit_tool" => {
                        if let Some(path) = tool_call.arguments.get("path") {
                            format!("📝 JSON edit: {}", path.as_str().unwrap_or(""))
                        } else {
                            "📝 JSON operation".to_string()
                        }
                    }
                    _ => format!("🔧 Using {}", tool_call.name),
                };
                println!("{}", action.blue());
            }

            // Execute tools with timing and animation
            let tool_start_time = std::time::Instant::now();

            // Start tool execution animation
            self.animation_manager
                .start_animation(AnimationState::ExecutingTools, "⚡ Executing tools", "cyan")
                .await;

            // Execute tools with interrupt support
            let tool_results = select! {
                results = self.tool_executor.execute_tools(&llm_response.tool_calls) => {
                    results
                }
                _ = cancellation_token.cancelled() => {
                    // Stop animation on interruption
                    self.animation_manager.stop_animation().await;
                    return Err(SageError::agent("Task interrupted during tool execution"));
                }
            };

            let tool_duration = tool_start_time.elapsed();

            // Stop animation and show timing if significant
            self.animation_manager.stop_animation().await;
            if tool_duration.as_millis() > 1000 {
                DisplayManager::print_timing("⚡ Tools", tool_duration);
            }

            // Show tool results briefly
            for result in &tool_results {
                if !result.success {
                    println!(
                        "❌ Error: {}",
                        result.error.as_deref().unwrap_or("Unknown error")
                    );
                } else if let Some(output) = &result.output {
                    // Only show output for certain tools or if it's short
                    if result.tool_name == "task_done" || output.len() < 100 {
                        println!("✅ {}", output.trim());
                    }
                }
            }

            step = step.with_tool_results(tool_results);
        }

        // Check if task is completed
        if llm_response.indicates_completion() {
            step.state = AgentState::Completed;
            DisplayManager::print_separator("Task Completed", "green");
        }

        step.complete();
        Ok(step)
    }

    /// Build conversation messages from execution history
    fn build_messages(
        &self,
        execution: &AgentExecution,
        system_message: &LLMMessage,
    ) -> Vec<LLMMessage> {
        let mut messages = vec![system_message.clone()];

        // If this is the first step (no previous steps), add an initial user message
        if execution.steps.is_empty() {
            let initial_user_message = LLMMessage::user(
                "Please start working on the task described in the system message.",
            );
            messages.push(initial_user_message);
        }

        for step in &execution.steps {
            // Add LLM response as assistant message
            if let Some(response) = &step.llm_response {
                let mut assistant_msg = LLMMessage::assistant(&response.content);
                if !response.tool_calls.is_empty() {
                    assistant_msg.tool_calls = Some(response.tool_calls.clone());
                }
                messages.push(assistant_msg);

                // Add tool results as user messages (like Python version)
                for result in &step.tool_results {
                    let content = if result.success {
                        result.output.as_deref().unwrap_or("")
                    } else {
                        &format!(
                            "Error: {}",
                            result.error.as_deref().unwrap_or("Unknown error")
                        )
                    };
                    let user_msg = LLMMessage::user(content);
                    messages.push(user_msg);
                }
            }
        }

        messages
    }
}

#[async_trait]
impl Agent for BaseAgent {
    async fn execute_task(&mut self, task: TaskMetadata) -> SageResult<ExecutionOutcome> {
        let mut execution = AgentExecution::new(task.clone());

        // Reset the global interrupt manager for this new task
        reset_global_interrupt_manager();

        // Create a task scope for interrupt handling
        let task_scope = global_interrupt_manager()
            .lock()
            .map_err(|_| SageError::agent("Failed to acquire interrupt manager lock"))?
            .create_task_scope();

        // Start trajectory recording if available
        if let Some(recorder) = &self.trajectory_recorder {
            let provider = self.config.get_default_provider().to_string();
            let model = self.config.default_model_parameters()?.model.clone();
            recorder
                .lock()
                .await
                .start_recording(task.clone(), provider, model, self.config.max_steps)
                .await?;
        }

        let system_message = self.create_system_message(&task);
        let tool_schemas = self.tool_executor.get_tool_schemas();
        let provider_name = self.config.get_default_provider().to_string();

        // Main execution loop - returns the final outcome
        let final_outcome = 'execution_loop: {
            for step_number in 1..=self.max_steps {
                // Check for interruption before each step
                if task_scope.is_cancelled() {
                    // Stop animation on interruption
                    self.animation_manager.stop_animation().await;

                    // Print interruption message
                    DisplayManager::print_separator("Task Interrupted", "yellow");
                    println!("{}", "🛑 Task interrupted by user (Ctrl+C)".yellow().bold());
                    println!("{}", "   Task execution stopped gracefully.".dimmed());

                    let interrupt_step = AgentStep::new(step_number, AgentState::Error)
                        .with_error("Task interrupted by user".to_string());

                    // Record interrupt step
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder
                            .lock()
                            .await
                            .record_step(interrupt_step.clone())
                            .await?;
                    }

                    execution.add_step(interrupt_step);
                    execution.complete(false, Some("Task interrupted by user".to_string()));
                    break 'execution_loop ExecutionOutcome::Interrupted { execution };
                }

                let messages = self.build_messages(&execution, &system_message);

                match self
                    .execute_step(step_number, &messages, &tool_schemas)
                    .await
                {
                    Ok(step) => {
                        let is_completed = step.state == AgentState::Completed;

                        // Record step in trajectory
                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder.lock().await.record_step(step.clone()).await?;
                        }

                        execution.add_step(step);

                        if is_completed {
                            execution
                                .complete(true, Some("Task completed successfully".to_string()));
                            break 'execution_loop ExecutionOutcome::Success(execution);
                        }
                    }
                    Err(e) => {
                        // Stop animation on error
                        self.animation_manager.stop_animation().await;

                        let error_step = AgentStep::new(step_number, AgentState::Error)
                            .with_error(e.to_string());

                        // Record error step
                        if let Some(recorder) = &self.trajectory_recorder {
                            recorder
                                .lock()
                                .await
                                .record_step(error_step.clone())
                                .await?;
                        }

                        execution.add_step(error_step);
                        execution.complete(false, Some(format!("Task failed: {}", e)));

                        // Create structured error with classification and suggestions
                        let exec_error =
                            ExecutionError::from_sage_error(&e, Some(provider_name.clone()));
                        break 'execution_loop ExecutionOutcome::Failed {
                            execution,
                            error: exec_error,
                        };
                    }
                }
            }

            // Max steps reached without completion
            execution.complete(
                false,
                Some("Task execution reached maximum steps".to_string()),
            );
            ExecutionOutcome::MaxStepsReached { execution }
        };

        // Finalize trajectory recording
        if let Some(recorder) = &self.trajectory_recorder {
            recorder
                .lock()
                .await
                .finalize_recording(
                    final_outcome.is_success(),
                    final_outcome.execution().final_result.clone(),
                )
                .await?;
        }

        Ok(final_outcome)
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn id(&self) -> Id {
        self.id
    }

    async fn continue_execution(
        &mut self,
        execution: &mut AgentExecution,
        user_message: &str,
    ) -> SageResult<()> {
        // Add user message to the execution context
        let system_message = self.create_system_message(&execution.task);
        let tool_schemas = self.tool_executor.get_tool_schemas();

        // Build messages including the new user message
        let mut messages = self.build_messages(execution, &system_message);
        messages.push(LLMMessage::user(user_message));

        // Continue execution from where we left off
        let start_step = (execution.steps.len() + 1) as u32;
        let max_step = start_step + self.max_steps - 1;

        for step_number in start_step..=max_step {
            match self
                .execute_step(step_number, &messages, &tool_schemas)
                .await
            {
                Ok(step) => {
                    let is_completed = step.state == AgentState::Completed;

                    // Record step in trajectory
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder.lock().await.record_step(step.clone()).await?;
                    }

                    execution.add_step(step);

                    if is_completed {
                        execution.complete(
                            true,
                            Some("Conversation continued successfully".to_string()),
                        );
                        break;
                    }

                    // Update messages for next iteration
                    let updated_messages = self.build_messages(execution, &system_message);
                    messages.clear();
                    messages.extend(updated_messages);
                }
                Err(e) => {
                    // Stop animation on error
                    self.animation_manager.stop_animation().await;

                    let error_step =
                        AgentStep::new(step_number, AgentState::Error).with_error(e.to_string());

                    // Record error step
                    if let Some(recorder) = &self.trajectory_recorder {
                        recorder
                            .lock()
                            .await
                            .record_step(error_step.clone())
                            .await?;
                    }

                    execution.add_step(error_step);
                    execution.complete(
                        false,
                        Some(format!("Conversation continuation failed: {}", e)),
                    );
                    return Err(e);
                }
            }
        }

        // If we reached max steps without completion
        if !execution.is_completed() {
            execution.complete(
                false,
                Some("Conversation continuation reached maximum steps".to_string()),
            );
        }

        Ok(())
    }
}
