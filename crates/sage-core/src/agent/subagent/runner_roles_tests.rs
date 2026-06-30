use super::SubAgentRunner;
use crate::agent::subagent::types::{AgentType, SubAgentConfig};
use crate::config::provider::ProviderConfig;
use crate::llm::client::LlmClient;
use crate::llm::provider_types::{LlmProvider, LlmRequestParams};
use crate::tools::base::{Tool, ToolError};
use crate::tools::types::{ToolCall, ToolResult, ToolSchema};
use async_trait::async_trait;
use std::fs;
use std::sync::Arc;

struct NamedTool(&'static str);

#[async_trait]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.0
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(self.name(), self.description(), vec![])
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        Ok(ToolResult::success(&call.id, self.name(), "ok"))
    }
}

fn runner() -> SubAgentRunner {
    runner_for_provider(LlmProvider::OpenAI)
}

fn runner_for_provider(provider: LlmProvider) -> SubAgentRunner {
    let llm_client = LlmClient::new(
        provider,
        ProviderConfig::new("openai").with_api_key("test-key"),
        LlmRequestParams::default(),
    )
    .expect("llm client");
    SubAgentRunner {
        llm_client,
        all_tools: vec![Arc::new(NamedTool("Read")), Arc::new(NamedTool("Write"))],
        max_steps: 1,
        working_directory: std::env::current_dir().expect("cwd"),
    }
}

#[test]
fn subagent_role_resolution_rejects_parent_tool_escalation() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path().join(".sage").join("agents");
    fs::create_dir_all(&root).expect("role root");
    fs::write(
        root.join("writer.toml"),
        r#"
name = "writer"
prompt = "write"
tools = ["Write"]
"#,
    )
    .expect("role file");

    let mut config = SubAgentConfig::new(AgentType::Custom, "task")
        .with_role_path("writer.toml")
        .with_parent_cwd(workspace.path().to_path_buf())
        .with_parent_tools(vec!["Read".to_string()]);

    let error = runner()
        .resolve_agent_role(&mut config)
        .expect_err("custom role cannot exceed parent tools");
    assert!(error.to_string().contains("outside parent tool scope"));
}

#[test]
fn subagent_role_resolution_rejects_profile_tool_escalation() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path().join(".sage").join("agents");
    fs::create_dir_all(&root).expect("role root");
    fs::write(
        root.join("writer.toml"),
        r#"
name = "writer"
prompt = "write"
tools = ["Write"]
profile = "review"
"#,
    )
    .expect("role file");

    let mut config = SubAgentConfig::new(AgentType::Custom, "task")
        .with_role_path("writer.toml")
        .with_parent_cwd(workspace.path().to_path_buf())
        .with_parent_tools(vec!["Read".to_string(), "Write".to_string()]);

    let error = runner()
        .resolve_agent_role(&mut config)
        .expect_err("custom role cannot exceed profile tools");
    assert!(error.to_string().contains("outside profile tool scope"));
}

#[test]
fn subagent_role_resolution_applies_reasoning_to_client_params() {
    let override_client = runner()
        .llm_client_for_role(Some("gpt-5.4"), Some("high"))
        .expect("override client")
        .expect("client changed");
    assert_eq!(
        override_client.model_params().reasoning_effort.as_deref(),
        Some("high")
    );
}

#[test]
fn subagent_role_resolution_rejects_model_from_other_provider() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path().join(".sage").join("agents");
    fs::create_dir_all(&root).expect("role root");
    fs::write(
        root.join("claude.toml"),
        r#"
name = "claude"
prompt = "review"
tools = ["Read"]
model = "claude-opus-4-7"
"#,
    )
    .expect("role file");

    let mut config = SubAgentConfig::new(AgentType::Custom, "task")
        .with_role_path("claude.toml")
        .with_parent_cwd(workspace.path().to_path_buf())
        .with_parent_tools(vec!["Read".to_string()]);

    let error = runner()
        .resolve_agent_role(&mut config)
        .expect_err("cross-provider model must fail");
    assert!(
        error
            .to_string()
            .contains("unsupported for provider 'openai'")
    );
}

#[test]
fn subagent_role_resolution_allows_dynamic_provider_model() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path().join(".sage").join("agents");
    fs::create_dir_all(&root).expect("role root");
    fs::write(
        root.join("dynamic.toml"),
        r#"
name = "dynamic"
prompt = "review"
tools = ["Read"]
model = "openai/gpt-4o"
"#,
    )
    .expect("role file");

    let mut config = SubAgentConfig::new(AgentType::Custom, "task")
        .with_role_path("dynamic.toml")
        .with_parent_cwd(workspace.path().to_path_buf())
        .with_parent_tools(vec!["Read".to_string()]);

    let role = runner_for_provider(LlmProvider::OpenRouter)
        .resolve_agent_role(&mut config)
        .expect("dynamic provider model should be accepted");
    assert_eq!(role.model.as_deref(), Some("openai/gpt-4o"));
}
