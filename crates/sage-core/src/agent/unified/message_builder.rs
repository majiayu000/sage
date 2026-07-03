//! Message building utilities for the unified executor

use crate::error::SageResult;
use crate::memory::{RecallQuery, recall_agent_context};
use crate::prompts::SystemPromptBuilder;
use crate::types::{MessageRole, TaskMetadata};
use std::path::PathBuf;
use tracing::instrument;

use super::UnifiedExecutor;
use super::context_builder::ContextBuilder;

impl UnifiedExecutor {
    /// Build the system prompt
    #[instrument(skip(self))]
    pub(super) async fn build_system_prompt(
        &self,
        task: Option<&TaskMetadata>,
    ) -> SageResult<String> {
        let model_name = self
            .config
            .default_model_parameters()
            .map(|p| p.model.clone())
            .unwrap_or_else(|_| "unknown".to_string());

        let working_dir_path = self
            .options
            .working_directory
            .clone()
            .or_else(|| self.config.working_directory.clone())
            .unwrap_or_else(|| PathBuf::from("."));

        let working_dir = working_dir_path.to_string_lossy().to_string();

        // Load project context (CLAUDE.md, instructions, git info)
        // Use spawn_blocking because ContextBuilder runs sync git commands
        let wd = working_dir_path.clone();
        let project_context =
            tokio::task::spawn_blocking(move || ContextBuilder::new(&wd).build_context())
                .await
                .unwrap_or_default();

        // Get tool schemas to include in prompt - CRITICAL for AI to know what tools are available
        let tool_schemas = self.tool_orchestrator.tool_executor().get_tool_schemas();

        // Get skills prompt for AI auto-invocation (Claude Code compatible)
        let skills_prompt = {
            let registry = self.skill_registry.read().await;
            registry.generate_skill_tool_prompt()
        };

        // Build system prompt with context
        let mut builder = SystemPromptBuilder::new()
            .with_model_name(&model_name)
            .with_working_dir(&working_dir)
            .with_tools(tool_schemas)
            .with_skills_prompt(skills_prompt);

        // Add git info if available
        if let Some(ref git_info) = project_context.git_info {
            builder = builder.with_git_info(
                git_info.is_repo,
                git_info.branch.as_deref().unwrap_or("unknown"),
                git_info.main_branch.as_deref().unwrap_or("main"),
            );
        }

        // Add CLAUDE.md as custom section (if exists)
        if let Some(ref claude_md) = project_context.claude_md {
            builder = builder.with_custom_section("Project Instructions (CLAUDE.md)", claude_md);
        }

        // Add .sage/instructions.md as custom section (if exists)
        if let Some(ref instructions) = project_context.project_instructions {
            builder = builder.with_custom_section("Project Instructions", instructions);
        }

        if let Some(task) = task {
            let recent_messages = self
                .conversation_history
                .iter()
                .rev()
                .filter(|message| message.role != MessageRole::System)
                .take(8)
                .map(|message| message.content.clone())
                .collect::<Vec<_>>();
            let query = RecallQuery {
                task_text: task.description.clone(),
                recent_messages,
                touched_paths: Vec::new(),
                limit: self.config.memory.max_recall_items,
            };
            match recall_agent_context(&self.config.memory, &working_dir_path, &query).await {
                Ok(Some(recalled)) => {
                    if let Some(section) = recalled.render_prompt_section() {
                        builder =
                            builder.with_custom_section("Recalled Memory And Learning", section);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        "Agent memory recall failed; continuing without memory injection"
                    );
                }
            }
        }

        let prompt = builder.build();
        Ok(prompt)
    }
}

#[cfg(test)]
mod tests {
    use crate::agent::ExecutionOptions;
    use crate::agent::UnifiedExecutor;
    use crate::config::{AgentMemoryConfig, Config};
    use crate::memory::runtime::clear_runtime_registry_for_tests;
    use crate::types::TaskMetadata;
    use tempfile::tempdir;

    #[tokio::test]
    async fn build_system_prompt_injects_redacted_recall_when_enabled() {
        clear_runtime_registry_for_tests().await;
        let dir = tempdir().unwrap();
        let mut config = Config::default();
        config.default_provider = "ollama".to_string();
        config.memory = AgentMemoryConfig {
            enabled: true,
            enabled_set: true,
            storage_path: Some(dir.path().join("memory.json")),
            ..AgentMemoryConfig::default()
        };
        let options = ExecutionOptions {
            working_directory: Some(dir.path().to_path_buf()),
            ..ExecutionOptions::default()
        };
        let runtime = crate::memory::init_agent_memory_runtime(&config.memory, dir.path())
            .await
            .unwrap()
            .unwrap();
        runtime
            .memory_manager()
            .remember_lesson("Run cargo check before completion. OPENAI_API_KEY=sk-secret12345")
            .await
            .unwrap();
        let executor = UnifiedExecutor::with_options(config, options).unwrap();
        let task = TaskMetadata::new("Run cargo check", dir.path().to_string_lossy().as_ref());

        let prompt = executor.build_system_prompt(Some(&task)).await.unwrap();

        assert!(prompt.contains("Recalled Memory And Learning"));
        assert!(prompt.contains("cargo check"));
        assert!(!prompt.contains("sk-secret12345"));
    }
}
