//! Branch creation, deletion, and modification operations

use super::super::types::{BranchId, BranchSnapshot, SerializedMessage, SerializedToolCall};
use super::core::BranchManager;

impl BranchManager {
    async fn next_branch_name(&self, name: Option<&str>) -> String {
        let mut counter = self.branch_counter.write().await;
        *counter += 1;
        name.map(|s| s.to_string())
            .unwrap_or_else(|| format!("branch-{}", *counter))
    }

    /// Create a new branch at current state
    pub async fn create_branch(
        &self,
        name: Option<&str>,
        messages: Vec<SerializedMessage>,
        tool_history: Vec<SerializedToolCall>,
    ) -> BranchId {
        let branch_name = self.next_branch_name(name).await;

        let mut current_branch = self.current_branch.write().await;

        let mut snapshot = BranchSnapshot::new(&branch_name, messages.len());
        snapshot.messages = messages;
        snapshot.tool_history = tool_history;

        if let Some(parent) = current_branch.clone() {
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
        *current_branch = Some(branch_id.clone());

        branch_id
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

    /// Delete a branch
    pub async fn delete(&self, branch_id: &BranchId) -> Option<BranchSnapshot> {
        // First, remove from branches and drop the lock
        let removed = {
            let mut branches = self.branches.write().await;
            branches.remove(branch_id)
        };

        // Then, update current branch with a single lock acquisition
        let mut current = self.current_branch.write().await;
        if current.as_ref() == Some(branch_id) {
            *current = None;
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

    async fn clear_current_branch(&self) {
        let mut current_branch = self.current_branch.write().await;
        *current_branch = None;
    }

    async fn reset_branch_counter(&self) {
        let mut counter = self.branch_counter.write().await;
        *counter = 0;
    }

    /// Clear all branches
    pub async fn clear(&self) {
        {
            let mut branches = self.branches.write().await;
            branches.clear();
        }
        self.clear_current_branch().await;
        self.reset_branch_counter().await;
    }
}
