//! Core BranchManager struct and basic operations

use super::super::types::{BranchId, BranchSnapshot};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manager for conversation branches
#[derive(Debug)]
pub struct BranchManager {
    /// All branches by ID
    pub(in super::super) branches: Arc<RwLock<HashMap<BranchId, BranchSnapshot>>>,
    /// Current active branch
    pub(crate) current_branch: Arc<RwLock<Option<BranchId>>>,
    /// Branch creation counter (for auto-naming)
    pub(super) branch_counter: Arc<RwLock<usize>>,
    /// Maximum branches to keep
    pub(super) max_branches: usize,
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

    /// Get read access to branches
    pub(in super::super) fn branches(&self) -> &Arc<RwLock<HashMap<BranchId, BranchSnapshot>>> {
        &self.branches
    }

    /// Find the oldest branch
    pub(super) fn find_oldest_branch(
        &self,
        branches: &HashMap<BranchId, BranchSnapshot>,
    ) -> Option<BranchId> {
        branches
            .iter()
            .min_by_key(|(_, b)| b.created_at)
            .map(|(id, _)| id.clone())
    }
}

impl Default for BranchManager {
    fn default() -> Self {
        Self::new()
    }
}
