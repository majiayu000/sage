//! Tests for checkpoint types

use super::types::*;

#[test]
fn test_checkpoint_id() {
    let id = CheckpointId::new();
    assert!(!id.as_str().is_empty());

    let id2 = CheckpointId::from_string("test-id");
    assert_eq!(id2.as_str(), "test-id");
}

#[test]
fn test_checkpoint_creation() {
    let checkpoint =
        Checkpoint::new("Test checkpoint", CheckpointType::Manual).with_name("My Checkpoint");

    assert_eq!(checkpoint.description, "Test checkpoint");
    assert_eq!(checkpoint.name, Some("My Checkpoint".to_string()));
    assert_eq!(checkpoint.checkpoint_type, CheckpointType::Manual);
}

#[test]
fn test_checkpoint_with_files() {
    let file = FileSnapshot::new(
        "src/main.rs",
        FileState::Exists {
            content: Some("fn main() {}".to_string()),
            content_ref: None,
        },
    );

    let checkpoint = Checkpoint::new("With files", CheckpointType::Auto).with_file(file);

    assert_eq!(checkpoint.file_count(), 1);
}

#[test]
fn test_restore_options() {
    let opts = RestoreOptions::all();
    assert!(opts.restore_files);
    assert!(opts.restore_conversation);
    assert!(opts.create_backup);
    assert!(!opts.dry_run);

    let opts = RestoreOptions::files_only();
    assert!(opts.restore_files);
    assert!(!opts.restore_conversation);
}

#[test]
fn test_checkpoint_type_display() {
    assert_eq!(CheckpointType::Auto.to_string(), "auto");
    assert_eq!(CheckpointType::Manual.to_string(), "manual");
    assert_eq!(CheckpointType::PreTool.to_string(), "pre-tool");
}
