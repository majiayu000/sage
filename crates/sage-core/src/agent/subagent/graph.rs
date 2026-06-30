//! Persistent parent-child graph for sub-agent threads.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

use crate::thread_store::{
    Page, ThreadId, ThreadItemRecord, ThreadLineage, ThreadListQuery, ThreadSnapshot, ThreadStatus,
    ThreadStore, ThreadStoreError,
};
use crate::types::MessageRole;

use super::types::ForkContextMessage;

const AGENT_PATH_PREFIX: &str = "agent://";
const GRAPH_FORK_MODE: &str = "subagent";

#[derive(Debug, Error)]
pub enum SubAgentGraphError {
    #[error("invalid agent path: {0}")]
    InvalidAgentPath(String),
    #[error("parent thread is archived: {0}")]
    ParentArchived(String),
    #[error("child thread is already linked to a different graph edge: {0}")]
    ConflictingChildEdge(String),
    #[error("agent {agent_path} is in invalid state {status} for {operation}")]
    InvalidAgentState {
        agent_path: String,
        status: String,
        operation: String,
    },
    #[error("agent message cannot be empty")]
    EmptyAgentMessage,
    #[error(transparent)]
    ThreadStore(#[from] ThreadStoreError),
}

pub type SubAgentGraphResult<T> = Result<T, SubAgentGraphError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentPath(String);

impl AgentPath {
    pub fn for_child_thread(thread_id: impl AsRef<str>) -> Self {
        Self(format!("{AGENT_PATH_PREFIX}{}", thread_id.as_ref()))
    }

    pub fn try_for_child_thread(thread_id: impl AsRef<str>) -> SubAgentGraphResult<Self> {
        let path = Self::for_child_thread(thread_id);
        path.child_thread_id()?;
        Ok(path)
    }

    pub fn from_raw_path(raw: impl Into<String>) -> SubAgentGraphResult<Self> {
        let path = Self(raw.into());
        path.child_thread_id()?;
        Ok(path)
    }

    pub fn child_thread_id(&self) -> SubAgentGraphResult<&str> {
        let Some(thread_id) = self.0.strip_prefix(AGENT_PATH_PREFIX) else {
            return Err(SubAgentGraphError::InvalidAgentPath(self.0.clone()));
        };
        if thread_id.trim().is_empty() || thread_id.chars().any(char::is_whitespace) {
            return Err(SubAgentGraphError::InvalidAgentPath(self.0.clone()));
        }
        Ok(thread_id)
    }

    pub fn as_path_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AgentPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentGraphDepth {
    Direct,
    Descendants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentGraphListQuery {
    pub depth: AgentGraphDepth,
    pub include_archived: bool,
}

impl AgentGraphListQuery {
    pub fn direct() -> Self {
        Self {
            depth: AgentGraphDepth::Direct,
            include_archived: false,
        }
    }

    pub fn descendants() -> Self {
        Self {
            depth: AgentGraphDepth::Descendants,
            include_archived: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildAgentSpawnRecord {
    pub parent_thread_id: ThreadId,
    pub child_thread_id: ThreadId,
    pub parent_turn_id: Option<String>,
    pub spawn_item_id: String,
    pub title: Option<String>,
    pub status: ThreadStatus,
}

impl ChildAgentSpawnRecord {
    pub fn new(
        parent_thread_id: impl Into<String>,
        child_thread_id: impl Into<String>,
        spawn_item_id: impl Into<String>,
    ) -> Self {
        Self {
            parent_thread_id: parent_thread_id.into(),
            child_thread_id: child_thread_id.into(),
            parent_turn_id: None,
            spawn_item_id: spawn_item_id.into(),
            title: None,
            status: ThreadStatus::Active,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildAgentSummary {
    pub agent_path: AgentPath,
    pub parent_thread_id: ThreadId,
    pub child_thread_id: ThreadId,
    pub parent_turn_id: Option<String>,
    pub spawn_item_id: String,
    pub status: ThreadStatus,
    pub archived: bool,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SubAgentGraph {
    pub(in crate::agent::subagent) store: Arc<dyn ThreadStore>,
}

impl SubAgentGraph {
    pub fn new(store: Arc<dyn ThreadStore>) -> Self {
        Self { store }
    }

    pub async fn record_child(
        &self,
        record: ChildAgentSpawnRecord,
    ) -> SubAgentGraphResult<ChildAgentSummary> {
        let parent = self.store.read_thread(&record.parent_thread_id).await?;
        if parent.thread.archived_at.is_some() {
            return Err(SubAgentGraphError::ParentArchived(
                record.parent_thread_id.clone(),
            ));
        }

        let agent_path = AgentPath::try_for_child_thread(&record.child_thread_id)?;
        let mut child_record = crate::thread_store::ThreadRecord::new(&record.child_thread_id);
        child_record.title = record.title.clone();
        child_record.status = record.status;
        child_record
            .metadata
            .insert("agent_path".to_string(), json!(agent_path.as_path_str()));
        child_record
            .metadata
            .insert("spawn_item_id".to_string(), json!(record.spawn_item_id));
        child_record.metadata.insert(
            "parent_thread_id".to_string(),
            json!(record.parent_thread_id),
        );

        match self.store.create_thread(child_record).await {
            Ok(_) => {}
            Err(ThreadStoreError::ThreadAlreadyExists(_)) => {
                self.ensure_existing_child_edge_matches(&record).await?;
            }
            Err(err) => return Err(err.into()),
        }

        self.store
            .set_lineage(ThreadLineage {
                thread_id: record.child_thread_id.clone(),
                parent_thread_id: Some(record.parent_thread_id.clone()),
                parent_turn_id: record.parent_turn_id.clone(),
                parent_item_id: Some(record.spawn_item_id.clone()),
                fork_mode: Some(GRAPH_FORK_MODE.to_string()),
            })
            .await?;

        self.read_child(&agent_path).await
    }

    pub async fn read_child(
        &self,
        agent_path: &AgentPath,
    ) -> SubAgentGraphResult<ChildAgentSummary> {
        let child_thread_id = agent_path.child_thread_id()?;
        let snapshot = self.store.read_thread(child_thread_id).await?;
        summary_from_snapshot(snapshot).ok_or_else(|| {
            SubAgentGraphError::InvalidAgentPath(agent_path.as_path_str().to_string())
        })
    }

    pub async fn fork_context_messages(
        &self,
        thread_id: &str,
    ) -> SubAgentGraphResult<Vec<ForkContextMessage>> {
        let snapshot = self.store.read_thread(thread_id).await?;
        Ok(fork_context_messages_from_snapshot(&snapshot))
    }

    pub async fn list_children(
        &self,
        parent_thread_id: &str,
        query: AgentGraphListQuery,
    ) -> SubAgentGraphResult<Vec<ChildAgentSummary>> {
        let parent = self.store.read_thread(parent_thread_id).await?;
        if parent.thread.archived_at.is_some() && !query.include_archived {
            return Ok(Vec::new());
        }

        let mut summaries = Vec::new();
        for snapshot in self.all_snapshots(query.include_archived).await? {
            if let Some(summary) = summary_from_snapshot(snapshot) {
                summaries.push(summary);
            }
        }

        let mut by_parent: HashMap<String, Vec<ChildAgentSummary>> = HashMap::new();
        for summary in summaries {
            by_parent
                .entry(summary.parent_thread_id.clone())
                .or_default()
                .push(summary);
        }

        match query.depth {
            AgentGraphDepth::Direct => {
                let mut direct = by_parent.remove(parent_thread_id).unwrap_or_default();
                sort_summaries(&mut direct);
                Ok(direct)
            }
            AgentGraphDepth::Descendants => Ok(descendants(parent_thread_id, by_parent)),
        }
    }

    async fn all_snapshots(
        &self,
        include_archived: bool,
    ) -> SubAgentGraphResult<Vec<ThreadSnapshot>> {
        let mut offset = 0;
        let mut snapshots = Vec::new();

        loop {
            let page: Page<_> = self
                .store
                .list_threads(ThreadListQuery {
                    include_archived,
                    limit: 100,
                    offset,
                })
                .await?;
            let count = page.items.len();
            for thread in page.items {
                snapshots.push(self.store.read_thread(&thread.thread_id).await?);
            }
            if count == 0 || snapshots.len() >= usize::try_from(page.total).unwrap_or(usize::MAX) {
                break;
            }
            offset += u64::try_from(count).unwrap_or(u64::MAX);
        }

        Ok(snapshots)
    }

    async fn ensure_existing_child_edge_matches(
        &self,
        record: &ChildAgentSpawnRecord,
    ) -> SubAgentGraphResult<()> {
        let snapshot = self.store.read_thread(&record.child_thread_id).await?;
        let Some(lineage) = snapshot.lineage else {
            return Err(SubAgentGraphError::ConflictingChildEdge(
                record.child_thread_id.clone(),
            ));
        };

        let matches_existing = lineage.parent_thread_id.as_deref()
            == Some(&record.parent_thread_id)
            && lineage.parent_turn_id == record.parent_turn_id
            && lineage.parent_item_id.as_deref() == Some(&record.spawn_item_id)
            && lineage.fork_mode.as_deref() == Some(GRAPH_FORK_MODE);
        if matches_existing {
            Ok(())
        } else {
            Err(SubAgentGraphError::ConflictingChildEdge(
                record.child_thread_id.clone(),
            ))
        }
    }
}

fn summary_from_snapshot(snapshot: ThreadSnapshot) -> Option<ChildAgentSummary> {
    let lineage = snapshot.lineage?;
    if lineage.fork_mode.as_deref() != Some(GRAPH_FORK_MODE) {
        return None;
    }
    let parent_thread_id = lineage.parent_thread_id?;
    let spawn_item_id = lineage.parent_item_id?;
    let agent_path = AgentPath::try_for_child_thread(&snapshot.thread.thread_id).ok()?;
    Some(ChildAgentSummary {
        agent_path,
        parent_thread_id,
        child_thread_id: snapshot.thread.thread_id,
        parent_turn_id: lineage.parent_turn_id,
        spawn_item_id,
        status: snapshot.thread.status,
        archived: snapshot.thread.archived_at.is_some(),
        title: snapshot.thread.title,
        created_at: snapshot.thread.created_at,
        updated_at: snapshot.thread.updated_at,
    })
}

fn fork_context_messages_from_snapshot(snapshot: &ThreadSnapshot) -> Vec<ForkContextMessage> {
    snapshot
        .items
        .iter()
        .filter_map(fork_context_message_from_item)
        .collect()
}

fn fork_context_message_from_item(item: &ThreadItemRecord) -> Option<ForkContextMessage> {
    let role = item.role.as_deref().and_then(message_role_from_store)?;
    let content = item
        .search_text
        .clone()
        .or_else(|| item.payload_json.as_ref().and_then(payload_content))?;
    ForkContextMessage::new(role, content).ok()
}

fn message_role_from_store(role: &str) -> Option<MessageRole> {
    match role {
        "user" => Some(MessageRole::User),
        "assistant" => Some(MessageRole::Assistant),
        _ => None,
    }
}

fn payload_content(value: &Value) -> Option<String> {
    let payload = value.get("payload").unwrap_or(value);
    for key in ["content", "message", "result"] {
        if let Some(text) = payload.get(key).and_then(Value::as_str) {
            return Some(text.to_string());
        }
    }
    None
}

fn descendants(
    parent_thread_id: &str,
    mut by_parent: HashMap<String, Vec<ChildAgentSummary>>,
) -> Vec<ChildAgentSummary> {
    let mut out = Vec::new();
    let mut queue = VecDeque::from([parent_thread_id.to_string()]);
    let mut visited = HashSet::new();

    while let Some(parent) = queue.pop_front() {
        if !visited.insert(parent.clone()) {
            continue;
        }
        let Some(mut children) = by_parent.remove(&parent) else {
            continue;
        };
        sort_summaries(&mut children);
        for child in children {
            queue.push_back(child.child_thread_id.clone());
            out.push(child);
        }
    }

    out
}

fn sort_summaries(summaries: &mut [ChildAgentSummary]) {
    summaries.sort_by(|left, right| left.child_thread_id.cmp(&right.child_thread_id));
}
