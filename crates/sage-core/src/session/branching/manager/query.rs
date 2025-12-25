//! Branch query and list operations

use super::super::types::{BranchId, BranchSnapshot};
use super::core::BranchManager;

impl BranchManager {
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

    /// Get branch count
    pub async fn count(&self) -> usize {
        self.branches.read().await.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.branches.read().await.is_empty()
    }
}
