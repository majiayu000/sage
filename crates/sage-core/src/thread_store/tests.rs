use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;

use super::error::ThreadStoreError;
use super::sqlite::SqliteThreadStore;
use super::traits::ThreadStore;
use super::types::{
    DeleteMode, RecoveryIssueCode, SearchQuery, ThreadItemInput, ThreadLineage, ThreadListQuery,
    ThreadRecord,
};

mod backfill;

#[tokio::test]
async fn thread_store_migration_creates_schema_and_rejects_newer_versions() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("threads.sqlite");
    let store = SqliteThreadStore::open(&db_path).unwrap();

    let version: i64 = store
        .with_conn(|conn| {
            conn.query_row("SELECT MAX(version) FROM thread_store_schema", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(version, 1);

    let table_count: i64 = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('threads', 'turns', 'items', 'thread_lineage', 'legacy_sources', 'store_errors')",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(table_count, 6);

    let future_path = temp.path().join("future.sqlite");
    {
        let conn = rusqlite::Connection::open(&future_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE thread_store_schema(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO thread_store_schema(version, applied_at) VALUES (999, '2026-01-01T00:00:00Z');",
        )
        .unwrap();
    }

    let err = match SqliteThreadStore::open(&future_path) {
        Ok(_) => panic!("future schema version should be rejected"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        ThreadStoreError::SchemaVersionMismatch { .. }
    ));
}

#[tokio::test]
async fn thread_store_create_thread_rejects_duplicate_without_overwrite() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-duplicate").with_title("Original"))
        .await
        .unwrap();

    let err = store
        .create_thread(ThreadRecord::new("thread-duplicate").with_title("Replacement"))
        .await
        .unwrap_err();
    assert!(matches!(err, ThreadStoreError::ThreadAlreadyExists(_)));

    let snapshot = store.read_thread("thread-duplicate").await.unwrap();
    assert_eq!(snapshot.thread.title.as_deref(), Some("Original"));
}

#[tokio::test]
async fn thread_store_list_search_archive_and_delete_roundtrip() {
    let temp = TempDir::new().unwrap();
    let store = SqliteThreadStore::open(temp.path().join("threads.sqlite")).unwrap();
    let payload_path = store.payload_root().unwrap().join("payload.txt");
    std::fs::write(&payload_path, "payload").unwrap();

    store
        .create_thread(ThreadRecord::new("thread-a").with_title("Alpha"))
        .await
        .unwrap();
    store
        .create_thread(ThreadRecord::new("thread-b").with_title("Beta"))
        .await
        .unwrap();

    let mut item = ThreadItemInput::new("message");
    item.search_text = Some("needle text".to_string());
    item.payload_ref = Some(store.payload_file_ref(&payload_path).unwrap());
    let append = store
        .append_event("thread-a", Some("turn-a"), item)
        .await
        .unwrap();
    assert_eq!(append.sequence, 0);

    let page = store
        .list_threads(ThreadListQuery::default())
        .await
        .unwrap();
    assert_eq!(page.total, 2);

    let search = store
        .search_threads(SearchQuery::new("needle"))
        .await
        .unwrap();
    assert_eq!(search.total, 1);
    assert_eq!(search.items[0].thread.thread_id, "thread-a");
    assert_eq!(
        search.items[0].matched_item_id.as_deref(),
        Some(append.item_id.as_str())
    );

    let archived = store
        .archive_thread("thread-a", Some("done".to_string()))
        .await
        .unwrap();
    assert!(archived.archived_at.is_some());
    assert_eq!(
        store
            .list_threads(ThreadListQuery::default())
            .await
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        store
            .search_threads(SearchQuery::new("needle"))
            .await
            .unwrap()
            .total,
        0
    );

    store.unarchive_thread("thread-a").await.unwrap();
    assert_eq!(
        store
            .search_threads(SearchQuery::new("needle"))
            .await
            .unwrap()
            .total,
        1
    );

    let deleted = store
        .delete_thread("thread-a", DeleteMode::MetadataAndPayloadFiles)
        .await
        .unwrap();
    assert!(deleted.metadata_deleted);
    assert_eq!(deleted.payload_files_deleted, 1);
    assert!(!payload_path.exists());
    assert!(matches!(
        store.read_thread("thread-a").await,
        Err(ThreadStoreError::ThreadNotFound(_))
    ));
}

#[tokio::test]
async fn thread_store_search_hit_uses_matching_item_only() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-search").with_title("Needle title"))
        .await
        .unwrap();

    let mut non_matching = ThreadItemInput::new("message");
    non_matching.item_id = Some("item-a".to_string());
    non_matching.search_text = Some("ordinary text".to_string());
    store
        .append_event("thread-search", None, non_matching)
        .await
        .unwrap();

    let title_match = store
        .search_threads(SearchQuery::new("needle"))
        .await
        .unwrap();
    assert_eq!(title_match.total, 1);
    assert_eq!(title_match.items[0].matched_item_id, None);
    assert_eq!(title_match.items[0].snippet, None);

    let mut matching = ThreadItemInput::new("message");
    matching.item_id = Some("item-b".to_string());
    matching.search_text = Some("exact needle payload".to_string());
    store
        .append_event("thread-search", None, matching)
        .await
        .unwrap();

    let item_match = store
        .search_threads(SearchQuery::new("payload"))
        .await
        .unwrap();
    assert_eq!(item_match.total, 1);
    assert_eq!(
        item_match.items[0].matched_item_id.as_deref(),
        Some("item-b")
    );
    assert_eq!(
        item_match.items[0].snippet.as_deref(),
        Some("exact needle payload")
    );
}

#[tokio::test]
async fn thread_store_payload_delete_errors_do_not_mark_payload_deleted() {
    let temp = TempDir::new().unwrap();
    let payload_path = temp.path().join("outside-payload.txt");
    std::fs::write(&payload_path, "outside").unwrap();
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-delete-error"))
        .await
        .unwrap();

    let mut item = ThreadItemInput::new("message");
    item.payload_ref = Some(format!("file:{}", payload_path.display()));
    store
        .append_event("thread-delete-error", None, item)
        .await
        .unwrap();
    let mut legacy_item = ThreadItemInput::new("message");
    legacy_item.payload_ref = Some("legacy_jsonl:/tmp/legacy.jsonl:1".to_string());
    store
        .append_event("thread-delete-error", None, legacy_item)
        .await
        .unwrap();

    let deleted = store
        .delete_thread("thread-delete-error", DeleteMode::MetadataAndPayloadFiles)
        .await
        .unwrap();
    assert!(!deleted.metadata_deleted);
    assert_eq!(deleted.payload_files_deleted, 0);
    assert_eq!(deleted.payload_delete_errors.len(), 2);
    assert!(payload_path.exists());
    assert!(store.read_thread("thread-delete-error").await.is_ok());

    let deleted_state: (Option<String>, Option<String>) = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT deleted_at, payload_deleted_at FROM threads WHERE thread_id = ?1",
                ["thread-delete-error"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(deleted_state, (None, None));
}

#[tokio::test]
async fn thread_store_duplicate_item_id_does_not_overwrite_existing_thread() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-one"))
        .await
        .unwrap();
    store
        .create_thread(ThreadRecord::new("thread-two"))
        .await
        .unwrap();

    let mut first = ThreadItemInput::new("message");
    first.item_id = Some("shared-item".to_string());
    first.search_text = Some("original".to_string());
    let first_append = store.append_event("thread-one", None, first).await.unwrap();

    let mut same_thread = ThreadItemInput::new("message");
    same_thread.item_id = Some("shared-item".to_string());
    same_thread.search_text = Some("ignored duplicate".to_string());
    let duplicate_same_thread = store
        .append_event("thread-one", None, same_thread)
        .await
        .unwrap();
    assert_eq!(duplicate_same_thread.sequence, first_append.sequence);

    let mut cross_thread = ThreadItemInput::new("message");
    cross_thread.item_id = Some("shared-item".to_string());
    cross_thread.search_text = Some("overwrite attempt".to_string());
    assert!(matches!(
        store.append_event("thread-two", None, cross_thread).await,
        Err(ThreadStoreError::InvalidInput(_))
    ));

    let one = store.read_thread("thread-one").await.unwrap();
    let two = store.read_thread("thread-two").await.unwrap();
    assert_eq!(one.items.len(), 1);
    assert_eq!(one.items[0].search_text.as_deref(), Some("original"));
    assert!(two.items.is_empty());
}

#[tokio::test]
async fn thread_store_turn_id_cannot_be_reused_across_threads() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-one"))
        .await
        .unwrap();
    store
        .create_thread(ThreadRecord::new("thread-two"))
        .await
        .unwrap();

    store
        .append_event(
            "thread-one",
            Some("shared-turn"),
            ThreadItemInput::new("message"),
        )
        .await
        .unwrap();
    assert!(matches!(
        store
            .append_event(
                "thread-two",
                Some("shared-turn"),
                ThreadItemInput::new("message"),
            )
            .await,
        Err(ThreadStoreError::InvalidInput(_))
    ));

    let one = store.read_thread("thread-one").await.unwrap();
    let two = store.read_thread("thread-two").await.unwrap();
    assert_eq!(one.turns.len(), 1);
    assert_eq!(one.items.len(), 1);
    assert!(two.turns.is_empty());
    assert!(two.items.is_empty());
}

#[tokio::test]
async fn thread_store_sequence_bounds_fail_closed() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-sequence"))
        .await
        .unwrap();

    let too_large = (i64::MAX as u64) + 1;
    let mut oversized = ThreadItemInput::new("message");
    oversized.sequence = Some(too_large);
    assert!(matches!(
        store.append_event("thread-sequence", None, oversized).await,
        Err(ThreadStoreError::InvalidInput(_))
    ));

    store
        .append_event(
            "thread-sequence",
            Some("turn-sequence"),
            ThreadItemInput::new("message"),
        )
        .await
        .unwrap();

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE items SET sequence = -1 WHERE thread_id = ?1",
                ["thread-sequence"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-sequence").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "items",
            field: "sequence",
            ..
        })
    ));

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE items SET sequence = 0 WHERE thread_id = ?1",
                ["thread-sequence"],
            )?;
            conn.execute(
                "UPDATE turns SET sequence_start = -1 WHERE thread_id = ?1",
                ["thread-sequence"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-sequence").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "turns",
            field: "sequence_start",
            ..
        })
    ));
}

#[tokio::test]
async fn thread_store_corrupt_stored_values_fail_closed() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-corrupt"))
        .await
        .unwrap();
    store
        .append_event(
            "thread-corrupt",
            Some("turn-corrupt"),
            ThreadItemInput::new("message"),
        )
        .await
        .unwrap();

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET metadata_json = '{broken' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-corrupt").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "threads",
            field: "metadata_json",
            ..
        })
    ));

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET metadata_json = '{}', created_at = 'not-time' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-corrupt").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "threads",
            field: "created_at",
            ..
        })
    ));

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET created_at = ?1 WHERE thread_id = ?2",
                [Utc::now().to_rfc3339(), "thread-corrupt".to_string()],
            )?;
            conn.execute(
                "UPDATE items SET payload_json = '{broken' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-corrupt").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "items",
            field: "payload_json",
            ..
        })
    ));

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE items SET payload_json = NULL WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            conn.execute(
                "UPDATE threads SET status = 'not-a-status' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-corrupt").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "threads",
            field: "status",
            ..
        })
    ));

    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET status = 'active' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            conn.execute(
                "UPDATE turns SET status = 'not-a-status' WHERE thread_id = ?1",
                ["thread-corrupt"],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        store.read_thread("thread-corrupt").await,
        Err(ThreadStoreError::InvalidStoredData {
            table: "turns",
            field: "status",
            ..
        })
    ));
}

#[tokio::test]
async fn thread_store_rejects_pagination_values_outside_sqlite_range() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("thread-pagination"))
        .await
        .unwrap();

    let too_large = (i64::MAX as u64) + 1;
    let mut list_query = ThreadListQuery::default();
    list_query.limit = too_large;
    assert!(matches!(
        store.list_threads(list_query).await,
        Err(ThreadStoreError::InvalidInput(_))
    ));

    let mut search_query = SearchQuery::new("pagination");
    search_query.offset = too_large;
    assert!(matches!(
        store.search_threads(search_query).await,
        Err(ThreadStoreError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn thread_store_lineage_roundtrip_reads_parent_links() {
    let store = SqliteThreadStore::in_memory().unwrap();
    store
        .create_thread(ThreadRecord::new("parent-thread"))
        .await
        .unwrap();
    store
        .create_thread(ThreadRecord::new("child-thread"))
        .await
        .unwrap();

    let lineage = store
        .set_lineage(ThreadLineage {
            thread_id: "child-thread".to_string(),
            parent_thread_id: Some("parent-thread".to_string()),
            parent_turn_id: Some("turn-parent".to_string()),
            parent_item_id: Some("item-parent".to_string()),
            fork_mode: Some("branch".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(lineage.parent_thread_id.as_deref(), Some("parent-thread"));

    let snapshot = store.read_thread("child-thread").await.unwrap();
    assert_eq!(
        snapshot
            .lineage
            .as_ref()
            .and_then(|lineage| lineage.parent_thread_id.as_deref()),
        Some("parent-thread")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn thread_store_concurrent_append_assigns_stable_sequences() {
    let store = Arc::new(SqliteThreadStore::in_memory().unwrap());
    store
        .create_thread(ThreadRecord::new("thread-concurrent"))
        .await
        .unwrap();

    let mut tasks = Vec::new();
    for idx in 0..24 {
        let store = store.clone();
        tasks.push(tokio::spawn(async move {
            let mut item = ThreadItemInput::new("message");
            item.search_text = Some(format!("message {idx}"));
            store
                .append_event("thread-concurrent", Some("turn-concurrent"), item)
                .await
                .unwrap()
        }));
    }

    let mut sequences = Vec::new();
    for task in tasks {
        sequences.push(task.await.unwrap().sequence);
    }
    sequences.sort_unstable();
    assert_eq!(sequences, (0..24).collect::<Vec<_>>());

    let snapshot = store.read_thread("thread-concurrent").await.unwrap();
    assert_eq!(snapshot.items.len(), 24);
    assert_eq!(snapshot.turns.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn thread_store_two_handles_assign_unique_sequences() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("threads.sqlite");
    let store_a = Arc::new(SqliteThreadStore::open(&db_path).unwrap());
    let store_b = Arc::new(SqliteThreadStore::open(&db_path).unwrap());
    store_a
        .create_thread(ThreadRecord::new("thread-two-handles"))
        .await
        .unwrap();

    let mut tasks = Vec::new();
    for idx in 0..32 {
        let store = if idx % 2 == 0 {
            store_a.clone()
        } else {
            store_b.clone()
        };
        tasks.push(tokio::spawn(async move {
            let mut item = ThreadItemInput::new("message");
            item.search_text = Some(format!("message {idx}"));
            store
                .append_event("thread-two-handles", Some("turn-two-handles"), item)
                .await
                .unwrap()
        }));
    }

    let mut sequences = Vec::new();
    for task in tasks {
        sequences.push(task.await.unwrap().sequence);
    }
    sequences.sort_unstable();
    assert_eq!(sequences, (0..32).collect::<Vec<_>>());

    let snapshot = store_a.read_thread("thread-two-handles").await.unwrap();
    assert_eq!(snapshot.items.len(), 32);
    assert_eq!(snapshot.turns.len(), 1);
}

#[tokio::test]
async fn thread_store_restart_recovery_detects_incomplete_turns() {
    let temp = TempDir::new().unwrap();
    let db_path = temp.path().join("threads.sqlite");
    {
        let store = SqliteThreadStore::open(&db_path).unwrap();
        store
            .create_thread(ThreadRecord::new("thread-restart"))
            .await
            .unwrap();
        store
            .append_event(
                "thread-restart",
                Some("turn-open"),
                ThreadItemInput::new("message"),
            )
            .await
            .unwrap();
    }

    let reopened = SqliteThreadStore::open(&db_path).unwrap();
    let snapshot = reopened.read_thread("thread-restart").await.unwrap();
    assert_eq!(snapshot.items.len(), 1);
    let recovery = reopened.detect_recovery().await.unwrap();
    assert!(recovery.issues.iter().any(|issue| {
        issue.code == RecoveryIssueCode::IncompleteTurn
            && issue.thread_id.as_deref() == Some("thread-restart")
            && issue.turn_id.as_deref() == Some("turn-open")
    }));
}
