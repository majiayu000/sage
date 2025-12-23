//! System prompt creation

use crate::llm::messages::LlmMessage;
use crate::prompts::SystemPromptBuilder;
use crate::tools::types::ToolSchema;
use crate::types::TaskMetadata;

use super::model_identity::get_model_identity;
use crate::config::model::Config;

/// Create initial system message using the modular prompt system
pub(super) fn create_system_message(
    task: &TaskMetadata,
    tool_schemas: &[ToolSchema],
    config: &Config,
) -> LlmMessage {
    // Get current model info for the identity section
    let model_info = get_model_identity(config);

    // Check if working directory is a git repo
    let is_git_repo = std::path::Path::new(&task.working_dir)
        .join(".git")
        .exists();

    // Get current git branch if in a git repo
    let (git_branch, main_branch) = if is_git_repo {
        let branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&task.working_dir)
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
        .with_model_name(&model_info.model_name)
        .with_task(&task.description)
        .with_working_dir(&task.working_dir)
        .with_git_info(is_git_repo, &git_branch, &main_branch)
        .with_platform(&platform, &os_version)
        .with_tools(tool_schemas.to_vec())
        .with_git_instructions(is_git_repo)
        .with_security_policy(true)
        .build();

    LlmMessage::system(system_prompt)
}
