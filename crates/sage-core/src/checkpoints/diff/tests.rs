//! Tests for diff module

#[cfg(test)]
mod tests {
    use super::super::super::types::{FileSnapshot, FileState};
    use super::super::capture::ChangeDetector;
    use super::super::changes::FileChange;
    use super::super::compare::{changes_to_snapshots, compare_snapshots};
    use super::super::text_diff::TextDiff;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use tokio::fs::{self, File};
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_capture_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"Hello, World!").await.unwrap();
        file.sync_all().await.unwrap(); // Ensure data is flushed to disk

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshot = detector.capture_file(&file_path).await.unwrap();

        assert!(snapshot.is_some());
        let snapshot = snapshot.unwrap();
        assert_eq!(snapshot.path, PathBuf::from("test.txt"));
        assert_eq!(snapshot.size, 13);
    }

    #[tokio::test]
    async fn test_capture_file_excluded_dir() {
        let temp_dir = TempDir::new().unwrap();
        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir_all(&node_modules).await.unwrap();

        let file_path = node_modules.join("test.js");
        let mut file = File::create(&file_path).await.unwrap();
        file.write_all(b"module.exports = {}").await.unwrap();
        file.sync_all().await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshot = detector.capture_file(&file_path).await.unwrap();

        assert!(snapshot.is_none()); // Should be excluded
    }

    #[tokio::test]
    async fn test_capture_file_extension_filter() {
        let temp_dir = TempDir::new().unwrap();

        let rs_file = temp_dir.path().join("main.rs");
        let txt_file = temp_dir.path().join("notes.txt");

        let mut f1 = File::create(&rs_file).await.unwrap();
        f1.write_all(b"fn main() {}").await.unwrap();
        f1.sync_all().await.unwrap();

        let mut f2 = File::create(&txt_file).await.unwrap();
        f2.write_all(b"Notes").await.unwrap();
        f2.sync_all().await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path()).with_extensions(["rs"]);

        let rs_snapshot = detector.capture_file(&rs_file).await.unwrap();
        let txt_snapshot = detector.capture_file(&txt_file).await.unwrap();

        assert!(rs_snapshot.is_some());
        assert!(txt_snapshot.is_none());
    }

    #[tokio::test]
    async fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).await.unwrap();

        let mut f1 = File::create(src_dir.join("main.rs")).await.unwrap();
        f1.write_all(b"fn main() {}").await.unwrap();
        f1.sync_all().await.unwrap();

        let mut f2 = File::create(src_dir.join("lib.rs")).await.unwrap();
        f2.write_all(b"pub mod test;").await.unwrap();
        f2.sync_all().await.unwrap();

        let detector = ChangeDetector::new(temp_dir.path());
        let snapshots = detector.scan_directory(temp_dir.path()).await.unwrap();

        assert_eq!(snapshots.len(), 2);
    }

    #[test]
    fn test_compare_snapshots_created() {
        let before: Vec<FileSnapshot> = vec![];
        let after = vec![FileSnapshot::new(
            "new.txt",
            FileState::Exists {
                content: Some("New content".to_string()),
                content_ref: None,
            },
        )];

        let changes = compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_created());
    }

    #[test]
    fn test_compare_snapshots_deleted() {
        let before = vec![FileSnapshot::new(
            "old.txt",
            FileState::Exists {
                content: Some("Old content".to_string()),
                content_ref: None,
            },
        )];
        let after: Vec<FileSnapshot> = vec![];

        let changes = compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_deleted());
    }

    #[test]
    fn test_compare_snapshots_modified() {
        let before = vec![
            FileSnapshot::new(
                "file.txt",
                FileState::Exists {
                    content: Some("Before".to_string()),
                    content_ref: None,
                },
            )
            .with_hash("hash1"),
        ];

        let after = vec![
            FileSnapshot::new(
                "file.txt",
                FileState::Exists {
                    content: Some("After".to_string()),
                    content_ref: None,
                },
            )
            .with_hash("hash2"),
        ];

        let changes = compare_snapshots(&before, &after);

        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_modified());
    }

    #[test]
    fn test_text_diff() {
        let old = "line1\nline2\nline3";
        let new = "line1\nmodified\nline3";

        let diff = TextDiff::compute(old, new);

        assert!(diff.has_changes());
        assert!(diff.added_count() > 0);
        assert!(diff.removed_count() > 0);
    }

    #[test]
    fn test_text_diff_no_changes() {
        let text = "line1\nline2\nline3";
        let diff = TextDiff::compute(text, text);

        assert!(!diff.has_changes());
    }

    #[test]
    fn test_text_diff_format_unified() {
        let old = "a\nb\nc";
        let new = "a\nx\nc";

        let diff = TextDiff::compute(old, new);
        let formatted = diff.format_unified();

        assert!(formatted.contains("@@"));
        assert!(formatted.contains("-b"));
        assert!(formatted.contains("+x"));
    }

    #[test]
    fn test_changes_to_snapshots() {
        let created = FileChange::Created {
            path: PathBuf::from("new.txt"),
            snapshot: FileSnapshot::new(
                "new.txt",
                FileState::Exists {
                    content: Some("content".to_string()),
                    content_ref: None,
                },
            ),
        };

        let snapshots = changes_to_snapshots(&[created]);

        assert_eq!(snapshots.len(), 1);
        if let FileState::Created { content, .. } = &snapshots[0].state {
            assert_eq!(content, &Some("content".to_string()));
        } else {
            panic!("Expected Created state");
        }
    }

    #[test]
    fn test_file_change_methods() {
        let change = FileChange::Created {
            path: PathBuf::from("test.txt"),
            snapshot: FileSnapshot::new("test.txt", FileState::Deleted),
        };

        assert_eq!(change.path(), Path::new("test.txt"));
        assert!(change.is_created());
        assert!(!change.is_modified());
        assert!(!change.is_deleted());
    }
}
