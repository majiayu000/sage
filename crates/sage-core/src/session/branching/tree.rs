//! Branch tree operations

use super::manager::BranchManager;
use super::types::{BranchId, BranchNode, BranchSnapshot};
use std::collections::HashMap;

impl BranchManager {
    /// Get branch tree structure
    pub async fn get_tree(&self) -> Vec<BranchNode> {
        let branches = self.current_branch.read().await;
        drop(branches);

        let branches = self.branches().read().await;
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
    #[allow(clippy::only_used_in_recursion)]
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
        let branches = self.branches().read().await;
        let mut ancestry = Vec::new();
        let mut current = branches.get(branch_id);

        while let Some(branch) = current {
            ancestry.push(branch.clone());
            current = branch.parent_id.as_ref().and_then(|id| branches.get(id));
        }

        ancestry.reverse();
        ancestry
    }
}
