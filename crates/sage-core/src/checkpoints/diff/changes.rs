//! File change types

use std::path::{Path, PathBuf};

use super::super::types::FileSnapshot;

/// Represents a file change
#[derive(Debug, Clone)]
pub enum FileChange {
    /// File was created
    Created {
        path: PathBuf,
        snapshot: FileSnapshot,
    },
    /// File was modified
    Modified {
        path: PathBuf,
        before: FileSnapshot,
        after: FileSnapshot,
    },
    /// File was deleted
    Deleted {
        path: PathBuf,
        snapshot: FileSnapshot,
    },
}

impl FileChange {
    /// Get the path of the changed file
    pub fn path(&self) -> &Path {
        match self {
            Self::Created { path, .. } => path,
            Self::Modified { path, .. } => path,
            Self::Deleted { path, .. } => path,
        }
    }

    /// Check if this is a creation
    pub fn is_created(&self) -> bool {
        matches!(self, Self::Created { .. })
    }

    /// Check if this is a modification
    pub fn is_modified(&self) -> bool {
        matches!(self, Self::Modified { .. })
    }

    /// Check if this is a deletion
    pub fn is_deleted(&self) -> bool {
        matches!(self, Self::Deleted { .. })
    }
}
