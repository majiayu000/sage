//! System prompt generation for reactive agent

use crate::llm::messages::LlmMessage;
use crate::prompts::SystemPromptBuilder;
use crate::tools::batch_executor::BatchToolExecutor;
use crate::types::TaskMetadata;

/// Create system message for Claude Code style interaction using modular prompt system
pub(super) fn create_system_message(
    batch_executor: &BatchToolExecutor,
    context: Option<&TaskMetadata>,
) -> LlmMessage {
    // Get tool schemas
    let tool_schemas = batch_executor.get_tool_schemas();

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

    LlmMessage::system(system_prompt)
}
