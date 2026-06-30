use chrono::Utc;
use tempfile::TempDir;
use uuid::Uuid;

use crate::session::enhanced::context::SessionContext;
use crate::session::jsonl_storage::SessionMetadata;
use crate::session::types::unified::SessionMessage;
use crate::thread_store::error::ThreadStoreError;
use crate::thread_store::sqlite::SqliteThreadStore;
use crate::thread_store::traits::ThreadStore;
use crate::thread_store::types::{BackfillOptions, LegacySourceKind, RecoveryIssueCode};
use crate::trajectory::SessionEntry;

#[tokio::test]
async fn thread_store_backfill_trajectory_records_partial_corrupt_jsonl() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("legacy.jsonl");
    let session_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tool_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let lines = [
        serde_json::to_string(&SessionEntry::SessionStart {
            session_id,
            task: "Migrate legacy".to_string(),
            provider: "openai".to_string(),
            model: "gpt-test".to_string(),
            cwd: temp.path().display().to_string(),
            git_branch: None,
            timestamp: now.clone(),
        })
        .unwrap(),
        serde_json::to_string(&SessionEntry::User {
            uuid: user_id,
            parent_uuid: None,
            content: serde_json::json!("hello"),
            timestamp: now.clone(),
        })
        .unwrap(),
        "{broken".to_string(),
        serde_json::to_string(&SessionEntry::ToolCall {
            uuid: tool_id,
            parent_uuid: None,
            tool_name: "bash".to_string(),
            tool_input: serde_json::json!({"cmd":"true"}),
            timestamp: now,
        })
        .unwrap(),
    ];
    std::fs::write(&path, lines.join("\n")).unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    let report = store
        .backfill_legacy(&path, BackfillOptions::trajectory_jsonl())
        .await
        .unwrap();
    assert!(report.partial);
    assert_eq!(report.imported_threads, 1);
    assert_eq!(report.imported_items, 2);
    assert_eq!(report.errors[0].code, "corrupt_jsonl");

    let snapshot = store.read_thread(&session_id.to_string()).await.unwrap();
    assert_eq!(snapshot.items.len(), 2);
    assert!(snapshot.items.iter().any(|item| item.partial_lineage));

    let recovery = store.detect_recovery().await.unwrap();
    assert!(
        recovery
            .issues
            .iter()
            .any(|issue| issue.code == RecoveryIssueCode::CorruptLegacySource)
    );
}

#[tokio::test]
async fn thread_store_backfill_trajectory_bad_timestamp_is_partial() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("legacy-bad-time.jsonl");
    let session_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let lines = [
        serde_json::to_string(&SessionEntry::SessionStart {
            session_id,
            task: "Bad timestamp".to_string(),
            provider: "openai".to_string(),
            model: "gpt-test".to_string(),
            cwd: temp.path().display().to_string(),
            git_branch: None,
            timestamp: now,
        })
        .unwrap(),
        serde_json::to_string(&SessionEntry::User {
            uuid: user_id,
            parent_uuid: None,
            content: serde_json::json!("bad time"),
            timestamp: "not-a-timestamp".to_string(),
        })
        .unwrap(),
    ];
    std::fs::write(&path, lines.join("\n")).unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    let report = store
        .backfill_legacy(&path, BackfillOptions::trajectory_jsonl())
        .await
        .unwrap();
    assert!(report.partial);
    assert_eq!(report.imported_threads, 1);
    assert_eq!(report.imported_items, 0);
    assert_eq!(report.errors[0].code, "corrupt_jsonl");

    let snapshot = store.read_thread(&session_id.to_string()).await.unwrap();
    assert_eq!(snapshot.items.len(), 0);
}

#[tokio::test]
async fn thread_store_backfill_session_directory_reports_missing_metadata() {
    let temp = TempDir::new().unwrap();
    let session_dir = temp.path().join("session-dir");
    std::fs::create_dir(&session_dir).unwrap();
    let message = SessionMessage::user(
        "hello from session",
        "session-dir",
        SessionContext::new(temp.path().to_path_buf()),
    );
    std::fs::write(
        session_dir.join("messages.jsonl"),
        format!("{}\n", serde_json::to_string(&message).unwrap()),
    )
    .unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    let report = store
        .backfill_legacy(
            &session_dir,
            BackfillOptions {
                source_kind: LegacySourceKind::SessionDirectory,
            },
        )
        .await
        .unwrap();
    assert!(report.partial);
    assert_eq!(report.imported_threads, 1);
    assert_eq!(report.imported_items, 1);
    assert_eq!(report.errors[0].code, "missing_metadata");

    let recovery = store.detect_recovery().await.unwrap();
    assert!(
        recovery
            .issues
            .iter()
            .any(|issue| issue.code == RecoveryIssueCode::MissingMetadata)
    );
}

#[tokio::test]
async fn thread_store_backfill_session_messages_tracks_source_and_partial_errors() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("messages.jsonl");
    let message = SessionMessage::user(
        "direct session message",
        "session-direct-1",
        SessionContext::new(temp.path().to_path_buf()),
    );
    let mixed = SessionMessage::user(
        "wrong session",
        "session-direct-2",
        SessionContext::new(temp.path().to_path_buf()),
    );
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n{{broken\n",
            serde_json::to_string(&message).unwrap(),
            serde_json::to_string(&mixed).unwrap()
        ),
    )
    .unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    let report = store
        .backfill_legacy(
            &path,
            BackfillOptions {
                source_kind: LegacySourceKind::SessionMessagesJsonl,
            },
        )
        .await
        .unwrap();
    assert!(report.partial);
    assert_eq!(report.imported_threads, 1);
    assert_eq!(report.imported_items, 1);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "mixed_session_id")
    );
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == "corrupt_jsonl")
    );

    let snapshot = store.read_thread("session-direct-1").await.unwrap();
    assert_eq!(
        snapshot.thread.legacy_session_id.as_deref(),
        Some("session-direct-1")
    );
    assert_eq!(snapshot.items.len(), 1);
    assert!(matches!(
        store.read_thread("messages").await,
        Err(ThreadStoreError::ThreadNotFound(_))
    ));

    let import_status: String = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT import_status FROM legacy_sources WHERE source_id = ?1",
                [report.source_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(import_status, "partial");
}

#[tokio::test]
async fn thread_store_backfill_trajectory_is_idempotent_for_synthetic_items() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("legacy-idempotent.jsonl");
    let session_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let end_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let lines = [
        serde_json::to_string(&SessionEntry::SessionStart {
            session_id,
            task: "Idempotent import".to_string(),
            provider: "openai".to_string(),
            model: "gpt-test".to_string(),
            cwd: temp.path().display().to_string(),
            git_branch: None,
            timestamp: now.clone(),
        })
        .unwrap(),
        serde_json::to_string(&SessionEntry::User {
            uuid: user_id,
            parent_uuid: None,
            content: serde_json::json!("hello"),
            timestamp: now.clone(),
        })
        .unwrap(),
        serde_json::to_string(&SessionEntry::SessionEnd {
            uuid: end_id,
            parent_uuid: Some(user_id),
            success: true,
            final_result: Some("done".to_string()),
            total_steps: 1,
            execution_time_secs: 0.1,
            timestamp: now,
        })
        .unwrap(),
    ];
    std::fs::write(&path, lines.join("\n")).unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .backfill_legacy(&path, BackfillOptions::trajectory_jsonl())
        .await
        .unwrap();
    store
        .backfill_legacy(&path, BackfillOptions::trajectory_jsonl())
        .await
        .unwrap();

    let snapshot = store.read_thread(&session_id.to_string()).await.unwrap();
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(snapshot.items[0].item_id, user_id.to_string());
    assert_eq!(snapshot.items[0].sequence, 1);
    assert_eq!(snapshot.items[1].item_id, end_id.to_string());
    assert_eq!(snapshot.items[1].sequence, 2);
}

#[tokio::test]
async fn thread_store_backfill_session_directory_imports_metadata() {
    let temp = TempDir::new().unwrap();
    let session_dir = temp.path().join("session-with-metadata");
    std::fs::create_dir(&session_dir).unwrap();
    let metadata = SessionMetadata::new("session-with-metadata", temp.path().to_path_buf())
        .with_name("Stored session")
        .with_model("gpt-test");
    std::fs::write(
        session_dir.join("metadata.json"),
        serde_json::to_string(&metadata).unwrap(),
    )
    .unwrap();
    let message = SessionMessage::assistant(
        "assistant text",
        "session-with-metadata",
        SessionContext::new(temp.path().to_path_buf()),
        None,
    );
    std::fs::write(
        session_dir.join("messages.jsonl"),
        format!("{}\n", serde_json::to_string(&message).unwrap()),
    )
    .unwrap();

    let store = SqliteThreadStore::in_memory().unwrap();
    let report = store
        .backfill_legacy(&session_dir, BackfillOptions::session_directory())
        .await
        .unwrap();
    assert!(!report.partial);
    assert_eq!(report.imported_threads, 1);
    assert_eq!(report.imported_items, 1);

    let snapshot = store.read_thread("session-with-metadata").await.unwrap();
    assert_eq!(snapshot.thread.title.as_deref(), Some("Stored session"));
    assert_eq!(snapshot.thread.model.as_deref(), Some("gpt-test"));
    assert_eq!(
        snapshot.items[0].search_text.as_deref(),
        Some("assistant text")
    );
}
