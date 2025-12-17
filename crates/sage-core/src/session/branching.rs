//! Conversation branching system
//!
//! Allows saving conversation state at key points and exploring
//! different approaches while being able to restore to previous states.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unique identifier for a branch
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(pub String);

impl BranchId {
    /// Create a new branch ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string()[..8].to_string())
    }

    /// Create from string
    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for BranchId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A snapshot of conversation state at a branch point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSnapshot {
    /// Branch identifier
    pub id: BranchId,
    /// Branch name (user-provided or auto-generated)
    pub name: String,
    /// Description of the branch
    pub description: Option<String>,
    /// Parent branch (if any)
    pub parent_id: Option<BranchId>,
    /// When this branch was created
    pub created_at: DateTime<Utc>,
    /// Message index at branch point
    pub message_index: usize,
    /// Serialized messages up to this point
    pub messages: Vec<SerializedMessage>,
    /// Tool call history
    pub tool_history: Vec<SerializedToolCall>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for organization
    pub tags: Vec<String>,
}

impl BranchSnapshot {
    /// Create a new branch snapshot
    pub fn new(name: impl Into<String>, message_index: usize) -> Self {
        Self {
            id: BranchId::new(),
            name: name.into(),
            description: None,
            parent_id: None,
            created_at: Utc::now(),
            message_index,
            messages: Vec::new(),
            tool_history: Vec::new(),
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set parent branch
    pub fn with_parent(mut self, parent: BranchId) -> Self {
        self.parent_id = Some(parent);
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Get age of the branch
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Check if this is a root branch (no parent)
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Serialized message for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Content
    pub content: String,
    /// Optional name
    pub name: Option<String>,
    /// Tool call ID (for tool results)
    pub tool_call_id: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Serialized tool call for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedToolCall {
    /// Tool name
    pub tool_name: String,
    /// Arguments (JSON)
    pub arguments: serde_json::Value,
    /// Result
    pub result: Option<String>,
    /// Success status
    pub success: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Branch tree node for visualization
#[derive(Debug, Clone)]
pub struct BranchNode {
    /// The branch snapshot
    pub branch: BranchSnapshot,
    /// Child branches
    pub children: Vec<BranchId>,
    /// Depth in tree
    pub depth: usize,
}

/// Manager for conversation branches
#[derive(Debug)]
pub struct BranchManager {
    /// All branches by ID
    branches: Arc<RwLock<HashMap<BranchId, BranchSnapshot>>>,
    /// Current active branch
    current_branch: Arc<RwLock<Option<BranchId>>>,
    /// Branch creation counter (for auto-naming)
    branch_counter: Arc<RwLock<usize>>,
    /// Maximum branches to keep
    max_branches: usize,
}

impl BranchManager {
    /// Create a new branch manager
    pub fn new() -> Self {
        Self {
            branches: Arc::new(RwLock::new(HashMap::new())),
            current_branch: Arc::new(RwLock::new(None)),
            branch_counter: Arc::new(RwLock::new(0)),
            max_branches: 100,
        }
    }

    /// Set maximum branches
    pub fn with_max_branches(mut self, max: usize) -> Self {
        self.max_branches = max;
        self
    }

    /// Create a new branch at current state
    pub async fn create_branch(
        &self,
        name: Option<&str>,
        messages: Vec<SerializedMessage>,
        tool_history: Vec<SerializedToolCall>,
    ) -> BranchId {
        let mut counter = self.branch_counter.write().await;
        *counter += 1;

        let branch_name = name
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("branch-{}", *counter));

        let current = self.current_branch.read().await.clone();

        let mut snapshot = BranchSnapshot::new(&branch_name, messages.len());
        snapshot.messages = messages;
        snapshot.tool_history = tool_history;

        if let Some(parent) = current {
            snapshot = snapshot.with_parent(parent);
        }

        let branch_id = snapshot.id.clone();

        let mut branches = self.branches.write().await;

        // Enforce max branches (remove oldest)
        while branches.len() >= self.max_branches {
            if let Some(oldest) = self.find_oldest_branch(&branches) {
                branches.remove(&oldest);
            } else {
                break;
            }
        }

        branches.insert(branch_id.clone(), snapshot);

        // Update current branch
        *self.current_branch.write().await = Some(branch_id.clone());

        branch_id
    }

    /// Find the oldest branch
    fn find_oldest_branch(&self, branches: &HashMap<BranchId, BranchSnapshot>) -> Option<BranchId> {
        branches
            .iter()
            .min_by_key(|(_, b)| b.created_at)
            .map(|(id, _)| id.clone())
    }

    /// Switch to a different branch
    pub async fn switch_to(&self, branch_id: &BranchId) -> Option<BranchSnapshot> {
        let branches = self.branches.read().await;

        if let Some(branch) = branches.get(branch_id) {
            *self.current_branch.write().await = Some(branch_id.clone());
            Some(branch.clone())
        } else {
            None
        }
    }

    /// Get current branch
    pub async fn current(&self) -> Option<BranchSnapshot> {
        let current_id = self.current_branch.read().await.clone()?;
        let branches = self.branches.read().await;
        branches.get(&current_id).cloned()
    }

    /// Get a branch by ID
    pub async fn get(&self, branch_id: &BranchId) -> Option<BranchSnapshot> {
        self.branches.read().await.get(branch_id).cloned()
    }

    /// List all branches
    pub async fn list(&self) -> Vec<BranchSnapshot> {
        self.branches.read().await.values().cloned().collect()
    }

    /// List branches sorted by creation time
    pub async fn list_sorted(&self) -> Vec<BranchSnapshot> {
        let mut branches: Vec<_> = self.branches.read().await.values().cloned().collect();
        branches.sort_by_key(|b| b.created_at);
        branches
    }

    /// List branches by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<BranchSnapshot> {
        self.branches
            .read()
            .await
            .values()
            .filter(|b| b.tags.iter().any(|t| t == tag))
            .cloned()
            .collect()
    }

    /// Delete a branch
    pub async fn delete(&self, branch_id: &BranchId) -> Option<BranchSnapshot> {
        // First, remove from branches and drop the lock
        let removed = {
            let mut branches = self.branches.write().await;
            branches.remove(branch_id)
        };

        // Then, check and update current_branch (separate lock acquisition)
        // Clone the current value to avoid holding the read lock while acquiring write lock
        let should_clear = {
            let current = self.current_branch.read().await;
            current.as_ref() == Some(branch_id)
        };

        if should_clear {
            *self.current_branch.write().await = None;
        }

        removed
    }

    /// Rename a branch
    pub async fn rename(&self, branch_id: &BranchId, new_name: impl Into<String>) -> bool {
        let mut branches = self.branches.write().await;

        if let Some(branch) = branches.get_mut(branch_id) {
            branch.name = new_name.into();
            true
        } else {
            false
        }
    }

    /// Add tag to branch
    pub async fn add_tag(&self, branch_id: &BranchId, tag: impl Into<String>) -> bool {
        let mut branches = self.branches.write().await;

        if let Some(branch) = branches.get_mut(branch_id) {
            branch.tags.push(tag.into());
            true
        } else {
            false
        }
    }

    /// Get branch tree structure
    pub async fn get_tree(&self) -> Vec<BranchNode> {
        let branches = self.branches.read().await;
        let mut nodes = Vec::new();

        // Find root branches (no parent)
        let roots: Vec<_> = branches
            .values()
            .filter(|b| b.parent_id.is_none())
            .collect();

        for root in roots {
            self.build_tree_recursive(&branches, root, 0, &mut nodes);
        }

        nodes
    }

    /// Recursively build tree nodes
    fn build_tree_recursive(
        &self,
        branches: &HashMap<BranchId, BranchSnapshot>,
        branch: &BranchSnapshot,
        depth: usize,
        nodes: &mut Vec<BranchNode>,
    ) {
        // Find children
        let children: Vec<_> = branches
            .values()
            .filter(|b| b.parent_id.as_ref() == Some(&branch.id))
            .map(|b| b.id.clone())
            .collect();

        nodes.push(BranchNode {
            branch: branch.clone(),
            children: children.clone(),
            depth,
        });

        // Recurse into children
        for child_id in children {
            if let Some(child) = branches.get(&child_id) {
                self.build_tree_recursive(branches, child, depth + 1, nodes);
            }
        }
    }

    /// Get branch ancestry (path from root to branch)
    pub async fn get_ancestry(&self, branch_id: &BranchId) -> Vec<BranchSnapshot> {
        let branches = self.branches.read().await;
        let mut ancestry = Vec::new();
        let mut current = branches.get(branch_id);

        while let Some(branch) = current {
            ancestry.push(branch.clone());
            current = branch.parent_id.as_ref().and_then(|id| branches.get(id));
        }

        ancestry.reverse();
        ancestry
    }

    /// Merge two branches (combine their messages)
    pub async fn merge(
        &self,
        source_id: &BranchId,
        target_id: &BranchId,
    ) -> Option<BranchId> {
        let (merged_messages, merged_history, merge_name) = {
            let branches = self.branches.read().await;

            let source = branches.get(source_id)?;
            let target = branches.get(target_id)?;

            // Create merged messages
            let mut merged_messages = target.messages.clone();
            merged_messages.extend(source.messages.clone());

            let mut merged_history = target.tool_history.clone();
            merged_history.extend(source.tool_history.clone());

            let merge_name = format!("merge-{}-{}", source.name, target.name);

            (merged_messages, merged_history, merge_name)
        };

        // Create new branch for merge result
        let branch_id = self
            .create_branch(Some(&merge_name), merged_messages, merged_history)
            .await;

        // Tag as merge
        self.add_tag(&branch_id, "merge").await;

        Some(branch_id)
    }

    /// Get branch count
    pub async fn count(&self) -> usize {
        self.branches.read().await.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.branches.read().await.is_empty()
    }

    /// Clear all branches
    pub async fn clear(&self) {
        self.branches.write().await.clear();
        *self.current_branch.write().await = None;
        *self.branch_counter.write().await = 0;
    }

    /// Export branches to JSON
    pub async fn export(&self) -> serde_json::Value {
        let branches = self.branches.read().await;
        serde_json::to_value(branches.values().collect::<Vec<_>>()).unwrap_or_default()
    }

    /// Import branches from JSON
    pub async fn import(&self, data: &serde_json::Value) -> Result<usize, String> {
        let imported: Vec<BranchSnapshot> = serde_json::from_value(data.clone())
            .map_err(|e| format!("Failed to parse branches: {}", e))?;

        let count = imported.len();
        let mut branches = self.branches.write().await;

        for branch in imported {
            branches.insert(branch.id.clone(), branch);
        }

        Ok(count)
    }
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe shared branch manager
pub type SharedBranchManager = Arc<BranchManager>;

/// Create a shared branch manager
pub fn create_branch_manager() -> SharedBranchManager {
    Arc::new(BranchManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_id() {
        let id1 = BranchId::new();
        let id2 = BranchId::new();
        assert_ne!(id1, id2);
        assert!(!id1.0.is_empty());
    }

    #[test]
    fn test_branch_snapshot_creation() {
        let snapshot = BranchSnapshot::new("test-branch", 5)
            .with_description("Test description")
            .with_tag("important");

        assert_eq!(snapshot.name, "test-branch");
        assert_eq!(snapshot.message_index, 5);
        assert_eq!(snapshot.description, Some("Test description".to_string()));
        assert!(snapshot.tags.contains(&"important".to_string()));
        assert!(snapshot.is_root());
    }

    #[test]
    fn test_branch_snapshot_with_parent() {
        let parent_id = BranchId::new();
        let snapshot = BranchSnapshot::new("child", 10).with_parent(parent_id.clone());

        assert!(!snapshot.is_root());
        assert_eq!(snapshot.parent_id, Some(parent_id));
    }

    #[tokio::test]
    async fn test_branch_manager_create() {
        let manager = BranchManager::new();

        let messages = vec![SerializedMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            name: None,
            tool_call_id: None,
            timestamp: Utc::now(),
        }];

        let branch_id = manager.create_branch(Some("test"), messages, vec![]).await;

        assert_eq!(manager.count().await, 1);
        assert!(manager.get(&branch_id).await.is_some());
    }

    #[tokio::test]
    async fn test_branch_manager_auto_name() {
        let manager = BranchManager::new();

        let id1 = manager.create_branch(None, vec![], vec![]).await;
        let id2 = manager.create_branch(None, vec![], vec![]).await;

        let branch1 = manager.get(&id1).await.unwrap();
        let branch2 = manager.get(&id2).await.unwrap();

        assert!(branch1.name.contains("branch-"));
        assert!(branch2.name.contains("branch-"));
        assert_ne!(branch1.name, branch2.name);
    }

    #[tokio::test]
    async fn test_branch_manager_switch() {
        let manager = BranchManager::new();

        let id1 = manager.create_branch(Some("first"), vec![], vec![]).await;
        let id2 = manager.create_branch(Some("second"), vec![], vec![]).await;

        let current = manager.current().await.unwrap();
        assert_eq!(current.id, id2);

        manager.switch_to(&id1).await;
        let current = manager.current().await.unwrap();
        assert_eq!(current.id, id1);
    }

    #[tokio::test]
    async fn test_branch_manager_delete() {
        let manager = BranchManager::new();

        let id = manager.create_branch(Some("to-delete"), vec![], vec![]).await;
        assert_eq!(manager.count().await, 1);

        let deleted = manager.delete(&id).await;
        assert!(deleted.is_some());
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_branch_manager_rename() {
        let manager = BranchManager::new();

        let id = manager.create_branch(Some("old-name"), vec![], vec![]).await;
        manager.rename(&id, "new-name").await;

        let branch = manager.get(&id).await.unwrap();
        assert_eq!(branch.name, "new-name");
    }

    #[tokio::test]
    async fn test_branch_manager_tags() {
        let manager = BranchManager::new();

        let id = manager.create_branch(Some("tagged"), vec![], vec![]).await;
        manager.add_tag(&id, "important").await;
        manager.add_tag(&id, "wip").await;

        let tagged = manager.list_by_tag("important").await;
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].id, id);
    }

    #[tokio::test]
    async fn test_branch_manager_ancestry() {
        let manager = BranchManager::new();

        let id1 = manager.create_branch(Some("root"), vec![], vec![]).await;
        let id2 = manager.create_branch(Some("child"), vec![], vec![]).await;
        let id3 = manager.create_branch(Some("grandchild"), vec![], vec![]).await;

        let ancestry = manager.get_ancestry(&id3).await;
        assert_eq!(ancestry.len(), 3);
        assert_eq!(ancestry[0].id, id1);
        assert_eq!(ancestry[1].id, id2);
        assert_eq!(ancestry[2].id, id3);
    }

    #[tokio::test]
    async fn test_branch_manager_tree() {
        let manager = BranchManager::new();

        manager.create_branch(Some("root"), vec![], vec![]).await;
        manager.create_branch(Some("child1"), vec![], vec![]).await;

        // Switch back to root and create another child
        let branches = manager.list().await;
        let root = branches.iter().find(|b| b.name == "root").unwrap();
        manager.switch_to(&root.id).await;
        manager.create_branch(Some("child2"), vec![], vec![]).await;

        let tree = manager.get_tree().await;
        assert!(tree.len() >= 3);
    }

    #[tokio::test]
    async fn test_branch_manager_max_branches() {
        let manager = BranchManager::new().with_max_branches(3);

        for i in 0..5 {
            manager
                .create_branch(Some(&format!("branch-{}", i)), vec![], vec![])
                .await;
        }

        // Should only keep max_branches
        assert!(manager.count().await <= 3);
    }

    #[tokio::test]
    async fn test_branch_manager_clear() {
        let manager = BranchManager::new();

        manager.create_branch(Some("a"), vec![], vec![]).await;
        manager.create_branch(Some("b"), vec![], vec![]).await;

        assert_eq!(manager.count().await, 2);

        manager.clear().await;

        assert!(manager.is_empty().await);
        assert!(manager.current().await.is_none());
    }

    #[tokio::test]
    async fn test_branch_manager_export_import() {
        let manager = BranchManager::new();

        manager.create_branch(Some("test"), vec![], vec![]).await;

        let exported = manager.export().await;

        let manager2 = BranchManager::new();
        let count = manager2.import(&exported).await.unwrap();

        assert_eq!(count, 1);
        assert_eq!(manager2.count().await, 1);
    }

    #[tokio::test]
    async fn test_branch_merge() {
        let manager = BranchManager::new();

        let msg1 = SerializedMessage {
            role: "user".to_string(),
            content: "First".to_string(),
            name: None,
            tool_call_id: None,
            timestamp: Utc::now(),
        };
        let msg2 = SerializedMessage {
            role: "user".to_string(),
            content: "Second".to_string(),
            name: None,
            tool_call_id: None,
            timestamp: Utc::now(),
        };

        let id1 = manager.create_branch(Some("branch1"), vec![msg1], vec![]).await;

        // Create independent branch
        *manager.current_branch.write().await = None;
        let id2 = manager.create_branch(Some("branch2"), vec![msg2], vec![]).await;

        let merged_id = manager.merge(&id1, &id2).await.unwrap();
        let merged = manager.get(&merged_id).await.unwrap();

        assert_eq!(merged.messages.len(), 2);
        assert!(merged.tags.contains(&"merge".to_string()));
    }

    #[test]
    fn test_serialized_message() {
        let msg = SerializedMessage {
            role: "assistant".to_string(),
            content: "Hello!".to_string(),
            name: Some("Claude".to_string()),
            tool_call_id: None,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SerializedMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.role, "assistant");
        assert_eq!(parsed.content, "Hello!");
    }

    #[test]
    fn test_serialized_tool_call() {
        let call = SerializedToolCall {
            tool_name: "Read".to_string(),
            arguments: serde_json::json!({"path": "/test"}),
            result: Some("content".to_string()),
            success: true,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&call).unwrap();
        let parsed: SerializedToolCall = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tool_name, "Read");
        assert!(parsed.success);
    }
}
