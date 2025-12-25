//! Branch export, import, and merge operations

use super::super::types::{BranchId, BranchSnapshot};
use super::core::BranchManager;

impl BranchManager {
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

    /// Merge two branches (combine their messages)
    pub async fn merge(&self, source_id: &BranchId, target_id: &BranchId) -> Option<BranchId> {
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
}
