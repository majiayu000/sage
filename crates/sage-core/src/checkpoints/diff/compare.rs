//! Snapshot comparison logic

use std::collections::{HashMap, HashSet};

use super::super::types::{FileSnapshot, FileState};
use super::changes::FileChange;

/// Compare two sets of file snapshots and generate changes
pub fn compare_snapshots(before: &[FileSnapshot], after: &[FileSnapshot]) -> Vec<FileChange> {
    let before_map: HashMap<_, _> = before.iter().map(|f| (&f.path, f)).collect();
    let after_map: HashMap<_, _> = after.iter().map(|f| (&f.path, f)).collect();

    let all_paths: HashSet<_> = before_map.keys().chain(after_map.keys()).collect();
    let mut changes = Vec::new();

    for path in all_paths {
        match (before_map.get(path), after_map.get(path)) {
            (Some(before_file), Some(after_file)) => {
                // Check if content changed
                if before_file.content_hash != after_file.content_hash {
                    changes.push(FileChange::Modified {
                        path: (*path).clone(),
                        before: (*before_file).clone(),
                        after: (*after_file).clone(),
                    });
                }
            }
            (Some(before_file), None) => {
                changes.push(FileChange::Deleted {
                    path: (*path).clone(),
                    snapshot: (*before_file).clone(),
                });
            }
            (None, Some(after_file)) => {
                changes.push(FileChange::Created {
                    path: (*path).clone(),
                    snapshot: (*after_file).clone(),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    changes
}

/// Create file snapshots from changes
pub fn changes_to_snapshots(changes: &[FileChange]) -> Vec<FileSnapshot> {
    changes
        .iter()
        .map(|change| match change {
            FileChange::Created { snapshot, .. } => FileSnapshot::new(
                snapshot.path.clone(),
                FileState::Created {
                    content: extract_content(&snapshot.state),
                    content_ref: None,
                },
            )
            .with_size(snapshot.size)
            .with_hash(snapshot.content_hash.clone().unwrap_or_default()),

            FileChange::Modified { before, after, .. } => FileSnapshot::new(
                after.path.clone(),
                FileState::Modified {
                    original_content: extract_content(&before.state),
                    original_content_ref: None,
                    new_content: extract_content(&after.state),
                    new_content_ref: None,
                },
            )
            .with_size(after.size)
            .with_hash(after.content_hash.clone().unwrap_or_default()),

            FileChange::Deleted { path, snapshot } => {
                FileSnapshot::new(path.clone(), FileState::Deleted).with_size(snapshot.size)
            }
        })
        .collect()
}

/// Extract content from FileState
fn extract_content(state: &FileState) -> Option<String> {
    match state {
        FileState::Exists { content, .. } => content.clone(),
        FileState::Created { content, .. } => content.clone(),
        FileState::Modified { new_content, .. } => new_content.clone(),
        FileState::Deleted => None,
    }
}
