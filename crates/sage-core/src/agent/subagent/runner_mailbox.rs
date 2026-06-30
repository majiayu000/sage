use std::sync::Arc;

use super::{SubAgentRunner, get_global_runner};
use crate::agent::subagent::{AgentPath, SubAgentConfig, SubAgentGraph, SubAgentResult};
use crate::error::{SageError, SageResult};
use crate::llm::messages::LlmMessage;
use tokio_util::sync::CancellationToken;

pub(super) struct MailboxRuntime {
    graph: Arc<SubAgentGraph>,
    agent_path: AgentPath,
    last_sequence: Option<u64>,
}

impl MailboxRuntime {
    pub(super) fn new(graph: Arc<SubAgentGraph>, agent_path: AgentPath) -> Self {
        Self {
            graph,
            agent_path,
            last_sequence: None,
        }
    }
}

impl SubAgentRunner {
    pub(super) async fn ingest_mailbox_follow_ups(
        &self,
        messages: &mut Vec<LlmMessage>,
        mailbox: &mut MailboxRuntime,
    ) -> SageResult<bool> {
        let follow_ups = mailbox
            .graph
            .read_follow_ups_after(&mailbox.agent_path, mailbox.last_sequence)
            .await
            .map_err(|err| SageError::agent(format!("failed to read sub-agent mailbox: {err}")))?;
        let received = !follow_ups.is_empty();
        for follow_up in follow_ups {
            mailbox.last_sequence = Some(
                mailbox
                    .last_sequence
                    .map_or(follow_up.sequence, |sequence| {
                        sequence.max(follow_up.sequence)
                    }),
            );
            messages.push(LlmMessage::user(format!(
                "Parent follow-up ({}): {}",
                follow_up.turn_id.as_deref().unwrap_or("mailbox turn"),
                follow_up.message
            )));
        }
        Ok(received)
    }
}

pub async fn execute_subagent_with_mailbox(
    config: SubAgentConfig,
    graph: Arc<SubAgentGraph>,
    agent_path: AgentPath,
) -> SageResult<SubAgentResult> {
    let runner_lock = get_global_runner().ok_or_else(|| {
        SageError::agent(
            "Sub-agent runner not initialized. Call init_global_runner_from_config first.",
        )
    })?;

    let guard = runner_lock.read().await;
    let runner = guard
        .as_ref()
        .ok_or_else(|| SageError::agent("Sub-agent runner not available"))?;

    let cancel = CancellationToken::new();
    runner
        .execute_with_mailbox(config, cancel, graph, agent_path)
        .await
}
