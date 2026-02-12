//! Session replayer for JSONL trajectory files

use crate::error::SageResult;
use crate::trajectory::entry::SessionEntry;
use crate::trajectory::session::{SessionInfo, SessionRecorder};
use crate::types::TokenUsage;
use crate::utils::truncate_with_ellipsis;
use std::path::Path;

/// Session summary statistics
#[derive(Debug, Clone, Default)]
pub struct TrajectorySessionSummary {
    /// Session ID
    pub session_id: Option<uuid::Uuid>,
    /// Task description
    pub task: Option<String>,
    /// Provider used
    pub provider: Option<String>,
    /// Model used
    pub model: Option<String>,
    /// Working directory
    pub cwd: Option<String>,
    /// Git branch
    pub git_branch: Option<String>,
    /// Total LLM requests
    pub llm_request_count: u32,
    /// Total LLM responses
    pub llm_response_count: u32,
    /// Total tool calls
    pub tool_call_count: u32,
    /// Successful tool calls
    pub successful_tool_calls: u32,
    /// Failed tool calls
    pub failed_tool_calls: u32,
    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total execution time in seconds
    pub execution_time_secs: Option<f64>,
    /// Whether session completed successfully
    pub success: Option<bool>,
    /// Final result
    pub final_result: Option<String>,
    /// Error messages
    pub errors: Vec<String>,
    /// Start timestamp
    pub start_time: Option<String>,
    /// End timestamp
    pub end_time: Option<String>,
}

/// Session replayer for loading and analyzing JSONL session files
pub struct SessionReplayer;

impl SessionReplayer {
    /// Load all entries from a session file
    pub async fn load(path: impl AsRef<Path>) -> SageResult<Vec<SessionEntry>> {
        SessionRecorder::load_entries(path).await
    }

    /// List all sessions for a project
    pub async fn list_sessions(working_dir: impl AsRef<Path>) -> SageResult<Vec<SessionInfo>> {
        SessionRecorder::list_sessions(working_dir).await
    }

    /// Get session summary from entries
    pub fn summarize(entries: &[SessionEntry]) -> TrajectorySessionSummary {
        let mut summary = TrajectorySessionSummary::default();

        for entry in entries {
            match entry {
                SessionEntry::SessionStart {
                    session_id,
                    task,
                    provider,
                    model,
                    cwd,
                    git_branch,
                    timestamp,
                } => {
                    summary.session_id = Some(*session_id);
                    summary.task = Some(task.clone());
                    summary.provider = Some(provider.clone());
                    summary.model = Some(model.clone());
                    summary.cwd = Some(cwd.clone());
                    summary.git_branch = git_branch.clone();
                    summary.start_time = Some(timestamp.clone());
                }
                SessionEntry::LlmRequest { .. } => {
                    summary.llm_request_count += 1;
                }
                SessionEntry::LlmResponse { usage, .. } => {
                    summary.llm_response_count += 1;
                    if let Some(usage) = usage {
                        summary.total_input_tokens += usage.input_tokens;
                        summary.total_output_tokens += usage.output_tokens;
                    }
                }
                SessionEntry::ToolCall { .. } => {
                    summary.tool_call_count += 1;
                }
                SessionEntry::ToolResult { success, .. } => {
                    if *success {
                        summary.successful_tool_calls += 1;
                    } else {
                        summary.failed_tool_calls += 1;
                    }
                }
                SessionEntry::Error { message, .. } => {
                    summary.errors.push(message.clone());
                }
                SessionEntry::SessionEnd {
                    success,
                    final_result,
                    execution_time_secs,
                    timestamp,
                    ..
                } => {
                    summary.success = Some(*success);
                    summary.final_result = final_result.clone();
                    summary.execution_time_secs = Some(*execution_time_secs);
                    summary.end_time = Some(timestamp.clone());
                }
                SessionEntry::User { .. } => {}
            }
        }

        summary
    }

    /// Get a brief description of a session from its entries
    pub fn get_session_preview(entries: &[SessionEntry]) -> Option<String> {
        for entry in entries {
            if let SessionEntry::SessionStart { task, .. } = entry {
                let preview = truncate_with_ellipsis(task, 100);
                return Some(preview);
            }
        }
        None
    }

    /// Extract all tool names used in a session
    pub fn get_tools_used(entries: &[SessionEntry]) -> Vec<String> {
        let mut tools: Vec<String> = entries
            .iter()
            .filter_map(|entry| {
                if let SessionEntry::ToolCall { tool_name, .. } = entry {
                    Some(tool_name.clone())
                } else {
                    None
                }
            })
            .collect();

        tools.sort();
        tools.dedup();
        tools
    }

    /// Get token usage breakdown
    pub fn get_token_usage(entries: &[SessionEntry]) -> TokenUsage {
        let mut usage = TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
            cache_read_tokens: None,
            cache_write_tokens: None,
            cost_estimate: None,
        };

        for entry in entries {
            if let SessionEntry::LlmResponse {
                usage: Some(entry_usage),
                ..
            } = entry
            {
                usage.input_tokens += entry_usage.input_tokens;
                usage.output_tokens += entry_usage.output_tokens;
                if let Some(cache_read) = entry_usage.cache_read_tokens {
                    *usage.cache_read_tokens.get_or_insert(0) += cache_read;
                }
                if let Some(cache_write) = entry_usage.cache_write_tokens {
                    *usage.cache_write_tokens.get_or_insert(0) += cache_write;
                }
            }
        }

        usage
    }

    /// Find entries by type
    pub fn filter_by_type<'a>(
        entries: &'a [SessionEntry],
        entry_type: &str,
    ) -> Vec<&'a SessionEntry> {
        entries
            .iter()
            .filter(|e| e.entry_type() == entry_type)
            .collect()
    }

    /// Get the last error in a session
    pub fn get_last_error(entries: &[SessionEntry]) -> Option<&SessionEntry> {
        entries
            .iter()
            .rev()
            .find(|e| matches!(e, SessionEntry::Error { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_entries() -> Vec<SessionEntry> {
        vec![
            SessionEntry::SessionStart {
                session_id: Uuid::new_v4(),
                task: "Test task".to_string(),
                provider: "glm".to_string(),
                model: "glm-4.7".to_string(),
                cwd: "/test".to_string(),
                git_branch: Some("main".to_string()),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            },
            SessionEntry::LlmRequest {
                uuid: Uuid::new_v4(),
                parent_uuid: None,
                messages: vec![],
                tools: Some(vec!["bash".to_string()]),
                timestamp: "2024-01-01T00:00:01Z".to_string(),
            },
            SessionEntry::LlmResponse {
                uuid: Uuid::new_v4(),
                parent_uuid: None,
                content: "Hello".to_string(),
                model: "glm-4.7".to_string(),
                usage: Some(TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_read_tokens: None,
                    cache_write_tokens: None,
                }),
                tool_calls: None,
                timestamp: "2024-01-01T00:00:02Z".to_string(),
            },
            SessionEntry::ToolCall {
                uuid: Uuid::new_v4(),
                parent_uuid: None,
                tool_name: "bash".to_string(),
                tool_input: serde_json::json!({"command": "ls"}),
                timestamp: "2024-01-01T00:00:03Z".to_string(),
            },
            SessionEntry::ToolResult {
                uuid: Uuid::new_v4(),
                parent_uuid: None,
                tool_name: "bash".to_string(),
                success: true,
                output: Some("file1\nfile2".to_string()),
                error: None,
                execution_time_ms: 100,
                timestamp: "2024-01-01T00:00:04Z".to_string(),
            },
            SessionEntry::SessionEnd {
                uuid: Uuid::new_v4(),
                parent_uuid: None,
                success: true,
                final_result: Some("Done".to_string()),
                total_steps: 1,
                execution_time_secs: 5.0,
                timestamp: "2024-01-01T00:00:05Z".to_string(),
            },
        ]
    }

    #[test]
    fn test_summarize() {
        let entries = create_test_entries();
        let summary = SessionReplayer::summarize(&entries);

        assert!(summary.session_id.is_some());
        assert_eq!(summary.task, Some("Test task".to_string()));
        assert_eq!(summary.provider, Some("glm".to_string()));
        assert_eq!(summary.llm_request_count, 1);
        assert_eq!(summary.llm_response_count, 1);
        assert_eq!(summary.tool_call_count, 1);
        assert_eq!(summary.successful_tool_calls, 1);
        assert_eq!(summary.total_input_tokens, 100);
        assert_eq!(summary.total_output_tokens, 50);
        assert_eq!(summary.success, Some(true));
    }

    #[test]
    fn test_get_tools_used() {
        let entries = create_test_entries();
        let tools = SessionReplayer::get_tools_used(&entries);

        assert_eq!(tools, vec!["bash".to_string()]);
    }

    #[test]
    fn test_get_session_preview() {
        let entries = create_test_entries();
        let preview = SessionReplayer::get_session_preview(&entries);

        assert_eq!(preview, Some("Test task".to_string()));
    }

    #[test]
    fn test_filter_by_type() {
        let entries = create_test_entries();
        let tool_calls = SessionReplayer::filter_by_type(&entries, "tool_call");

        assert_eq!(tool_calls.len(), 1);
    }
}
