//! Durable mailbox and lifecycle events for graph-backed sub-agents.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{AgentPath, SubAgentGraph, SubAgentGraphError, SubAgentGraphResult};
use crate::thread_store::{AppendResult, ThreadItemInput, ThreadStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMailboxReceipt {
    pub agent_path: AgentPath,
    pub child_thread_id: String,
    pub turn_id: String,
    pub item_id: String,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTerminalState {
    pub status: ThreadStatus,
    pub result: Option<String>,
    pub reason: Option<String>,
    pub item_id: String,
    pub turn_id: Option<String>,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFollowUpMessage {
    pub message: String,
    pub item_id: String,
    pub turn_id: Option<String>,
    pub sequence: u64,
}

impl SubAgentGraph {
    pub async fn send_follow_up(
        &self,
        agent_path: &AgentPath,
        message: &str,
    ) -> SubAgentGraphResult<AgentMailboxReceipt> {
        let message = message.trim();
        if message.is_empty() {
            return Err(SubAgentGraphError::EmptyAgentMessage);
        }
        let summary = self.read_child(agent_path).await?;
        ensure_active(agent_path, summary.status, "follow_up")?;

        let turn_id = generated_id("turn", &summary.child_thread_id, "followup");
        let item_id = generated_id("item", &summary.child_thread_id, "followup");
        let mut item = ThreadItemInput::new("message");
        item.item_id = Some(item_id);
        item.turn_id = Some(turn_id.clone());
        item.role = Some("user".to_string());
        item.status = Some("queued".to_string());
        item.source = "agent_mailbox".to_string();
        item.payload_json = Some(json!({
            "kind": "agent.follow_up",
            "agent_path": agent_path.as_path_str(),
            "message": message,
        }));
        item.search_text = Some(message.to_string());

        let append = self
            .store
            .append_event(&summary.child_thread_id, Some(&turn_id), item)
            .await?;
        Ok(receipt(agent_path.clone(), summary.child_thread_id, append))
    }

    pub async fn interrupt_child(
        &self,
        agent_path: &AgentPath,
        reason: Option<&str>,
    ) -> SubAgentGraphResult<AgentMailboxReceipt> {
        let summary = self.read_child(agent_path).await?;
        ensure_active(agent_path, summary.status, "interrupt")?;

        let turn_id = generated_id("turn", &summary.child_thread_id, "interrupt");
        let item_id = generated_id("item", &summary.child_thread_id, "interrupt");
        let reason = reason
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("interrupted by parent");
        let append = self
            .append_lifecycle_item(
                &summary.child_thread_id,
                Some(turn_id),
                item_id,
                ThreadStatus::Interrupted,
                None,
                Some(reason),
                "agent.interrupt",
            )
            .await?;
        self.store
            .set_thread_status(&summary.child_thread_id, ThreadStatus::Interrupted)
            .await?;
        Ok(receipt(agent_path.clone(), summary.child_thread_id, append))
    }

    pub async fn record_terminal_state(
        &self,
        agent_path: &AgentPath,
        status: ThreadStatus,
        result: Option<&str>,
    ) -> SubAgentGraphResult<AgentMailboxReceipt> {
        let summary = self.read_child(agent_path).await?;
        ensure_active(agent_path, summary.status, "record_terminal_state")?;
        let item_id = generated_id("item", &summary.child_thread_id, "terminal");
        let append = self
            .append_lifecycle_item(
                &summary.child_thread_id,
                None,
                item_id,
                status,
                result,
                None,
                "agent.terminal",
            )
            .await?;
        self.store
            .set_thread_status(&summary.child_thread_id, status)
            .await?;
        Ok(receipt(agent_path.clone(), summary.child_thread_id, append))
    }

    pub async fn read_follow_ups_after(
        &self,
        agent_path: &AgentPath,
        after_sequence: Option<u64>,
    ) -> SubAgentGraphResult<Vec<AgentFollowUpMessage>> {
        let summary = self.read_child(agent_path).await?;
        let snapshot = self.store.read_thread(&summary.child_thread_id).await?;
        let mut messages = snapshot
            .items
            .iter()
            .filter(|item| after_sequence.map_or(true, |sequence| item.sequence > sequence))
            .filter_map(follow_up_from_item)
            .collect::<Vec<_>>();
        messages.sort_by_key(|message| message.sequence);
        Ok(messages)
    }

    pub async fn read_terminal_state(
        &self,
        agent_path: &AgentPath,
    ) -> SubAgentGraphResult<Option<AgentTerminalState>> {
        let summary = self.read_child(agent_path).await?;
        let snapshot = self.store.read_thread(&summary.child_thread_id).await?;
        Ok(snapshot
            .items
            .iter()
            .rev()
            .find_map(|item| terminal_state_from_item(item)))
    }

    async fn append_lifecycle_item(
        &self,
        child_thread_id: &str,
        turn_id: Option<String>,
        item_id: String,
        status: ThreadStatus,
        result: Option<&str>,
        reason: Option<&str>,
        kind: &str,
    ) -> SubAgentGraphResult<AppendResult> {
        let mut item = ThreadItemInput::new("agent_lifecycle");
        item.item_id = Some(item_id);
        item.turn_id = turn_id.clone();
        item.status = Some(status.as_str().to_string());
        item.source = "agent_mailbox".to_string();
        item.created_at = Utc::now();
        item.payload_json = Some(json!({
            "kind": kind,
            "status": status.as_str(),
            "result": result,
            "reason": reason,
        }));
        item.search_text = result.or(reason).map(str::to_string);
        Ok(self
            .store
            .append_event(child_thread_id, turn_id.as_deref(), item)
            .await?)
    }
}

fn ensure_active(
    agent_path: &AgentPath,
    status: ThreadStatus,
    operation: &str,
) -> SubAgentGraphResult<()> {
    if matches!(
        status,
        ThreadStatus::Completed | ThreadStatus::Failed | ThreadStatus::Interrupted
    ) {
        return Err(SubAgentGraphError::InvalidAgentState {
            agent_path: agent_path.as_path_str().to_string(),
            status: status.as_str().to_string(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}

fn generated_id(prefix: &str, child_thread_id: &str, kind: &str) -> String {
    format!(
        "{prefix}_{child_thread_id}_{kind}_{}",
        Uuid::new_v4().as_simple()
    )
}

fn receipt(
    agent_path: AgentPath,
    child_thread_id: String,
    append: AppendResult,
) -> AgentMailboxReceipt {
    AgentMailboxReceipt {
        agent_path,
        child_thread_id,
        turn_id: append.turn_id.unwrap_or_default(),
        item_id: append.item_id,
        sequence: append.sequence,
    }
}

fn terminal_state_from_item(
    item: &crate::thread_store::ThreadItemRecord,
) -> Option<AgentTerminalState> {
    let payload = item.payload_json.as_ref()?;
    let kind = payload.get("kind").and_then(serde_json::Value::as_str)?;
    if kind != "agent.terminal" && kind != "agent.interrupt" {
        return None;
    }
    let status = payload
        .get("status")
        .and_then(serde_json::Value::as_str)
        .and_then(ThreadStatus::from_store)?;
    Some(AgentTerminalState {
        status,
        result: payload
            .get("result")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        reason: payload
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        item_id: item.item_id.clone(),
        turn_id: item.turn_id.clone(),
        sequence: item.sequence,
    })
}

fn follow_up_from_item(
    item: &crate::thread_store::ThreadItemRecord,
) -> Option<AgentFollowUpMessage> {
    let payload = item.payload_json.as_ref()?;
    let kind = payload.get("kind").and_then(serde_json::Value::as_str)?;
    if kind != "agent.follow_up" {
        return None;
    }
    Some(AgentFollowUpMessage {
        message: payload
            .get("message")
            .and_then(serde_json::Value::as_str)?
            .to_string(),
        item_id: item.item_id.clone(),
        turn_id: item.turn_id.clone(),
        sequence: item.sequence,
    })
}
