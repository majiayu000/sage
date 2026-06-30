mod integers;
mod migrations;
mod payload;
mod queries;
mod thread_rows;

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use super::backfill::backfill_legacy_path;
use super::error::{ThreadStoreError, ThreadStoreResult};
use super::recovery::detect_startup_issues;
use super::traits::ThreadStore;
use super::types::{
    AppendResult, BackfillOptions, BackfillReport, DeleteMode, DeleteResult, Page, RecoveryReport,
    SearchHit, SearchQuery, ThreadItemInput, ThreadLineage, ThreadListQuery, ThreadRecord,
    ThreadSnapshot, ThreadStatus,
};
use payload::{delete_payload_refs, validate_relative_payload_path};
use queries::{
    existing_item_identity, list_threads, payload_refs, row_to_lineage, row_to_thread,
    rows_to_items, rows_to_turns, search_threads,
};
use thread_rows::{insert_thread, upsert_thread};

#[derive(Clone)]
pub struct SqliteThreadStore {
    conn: Arc<Mutex<Connection>>,
    payload_root: Option<PathBuf>,
}

impl SqliteThreadStore {
    pub fn open(path: impl AsRef<Path>) -> ThreadStoreResult<Self> {
        let path = path.as_ref();
        if path != Path::new(":memory:") {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let payload_root = if path != Path::new(":memory:") {
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            let root = parent.join("thread_store_payloads");
            std::fs::create_dir_all(&root)?;
            Some(root)
        } else {
            None
        };
        let conn =
            Connection::open(path).map_err(|err| ThreadStoreError::NotWritable(err.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        migrations::migrate(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            payload_root,
        })
    }

    pub fn in_memory() -> ThreadStoreResult<Self> {
        Self::open(":memory:")
    }

    pub fn payload_root(&self) -> Option<&Path> {
        self.payload_root.as_deref()
    }

    pub fn payload_file_ref(&self, path: impl AsRef<Path>) -> ThreadStoreResult<String> {
        let root = self.payload_root().ok_or_else(|| {
            ThreadStoreError::InvalidInput("payload root is not configured".into())
        })?;
        let canonical_root = root.canonicalize()?;
        let path = path.as_ref();
        if path.symlink_metadata()?.file_type().is_symlink() {
            return Err(ThreadStoreError::InvalidInput(format!(
                "payload path is a symlink: {}",
                path.display()
            )));
        }
        let canonical_path = path.canonicalize()?;
        let relative = canonical_path.strip_prefix(&canonical_root).map_err(|_| {
            ThreadStoreError::InvalidInput(format!(
                "payload path is outside store root: {}",
                canonical_path.display()
            ))
        })?;
        validate_relative_payload_path(relative).map_err(|err| {
            ThreadStoreError::InvalidInput(format!("invalid payload path: {err}"))
        })?;
        Ok(format!("store_payload:{}", relative.to_string_lossy()))
    }

    pub(crate) fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> ThreadStoreResult<T>,
    ) -> ThreadStoreResult<T> {
        let conn = self.conn.lock();
        f(&conn)
    }

    pub(crate) fn upsert_thread_record(
        &self,
        record: &ThreadRecord,
    ) -> ThreadStoreResult<ThreadRecord> {
        self.with_conn(|conn| {
            upsert_thread(conn, record)?;
            row_to_thread(conn, &record.thread_id)
        })
    }

    pub(crate) fn upsert_legacy_source(
        &self,
        source_id: &str,
        path: &Path,
        source_kind: &str,
        checksum: &str,
        import_status: &str,
    ) -> ThreadStoreResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT INTO legacy_sources(source_id, path, source_kind, imported_at, checksum, import_status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(source_id) DO UPDATE SET
                    imported_at = excluded.imported_at,
                    checksum = excluded.checksum,
                    import_status = excluded.import_status
                "#,
                params![
                    source_id,
                    path.to_string_lossy().as_ref(),
                    source_kind,
                    Utc::now().to_rfc3339(),
                    checksum,
                    import_status
                ],
            )?;
            Ok(())
        })
    }

    pub(crate) fn record_store_error(
        &self,
        error: &super::types::StoreErrorRecord,
    ) -> ThreadStoreResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                r#"
                INSERT OR REPLACE INTO store_errors(
                    error_id, thread_id, source_id, code, message, details, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    error.error_id,
                    error.thread_id,
                    error.source_id,
                    error.code,
                    error.message,
                    error.details,
                    error.created_at.to_rfc3339()
                ],
            )?;
            Ok(())
        })
    }

    pub(crate) fn set_thread_status(
        &self,
        thread_id: &str,
        status: ThreadStatus,
    ) -> ThreadStoreResult<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE threads SET status = ?1, updated_at = ?2 WHERE thread_id = ?3",
                params![status.as_str(), Utc::now().to_rfc3339(), thread_id],
            )?;
            Ok(())
        })
    }
}

#[async_trait]
impl ThreadStore for SqliteThreadStore {
    async fn create_thread(&self, record: ThreadRecord) -> ThreadStoreResult<ThreadRecord> {
        self.with_conn(|conn| {
            insert_thread(conn, &record)?;
            row_to_thread(conn, &record.thread_id)
        })
    }

    async fn set_lineage(&self, lineage: ThreadLineage) -> ThreadStoreResult<ThreadLineage> {
        self.with_conn(|conn| {
            let thread_id = lineage.thread_id.clone();
            ensure_thread_exists(conn, &thread_id)?;
            if let Some(parent_thread_id) = &lineage.parent_thread_id {
                ensure_thread_exists(conn, parent_thread_id)?;
            }
            conn.execute(
                r#"
                INSERT OR REPLACE INTO thread_lineage(
                    thread_id, parent_thread_id, parent_turn_id, parent_item_id, fork_mode
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    lineage.thread_id,
                    lineage.parent_thread_id,
                    lineage.parent_turn_id,
                    lineage.parent_item_id,
                    lineage.fork_mode
                ],
            )?;
            row_to_lineage(conn, &lineage.thread_id)?
                .ok_or_else(|| ThreadStoreError::ThreadNotFound(thread_id))
        })
    }

    async fn resume_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadSnapshot> {
        self.read_thread(thread_id).await
    }

    async fn append_event(
        &self,
        thread_id: &str,
        turn_id: Option<&str>,
        mut item: ThreadItemInput,
    ) -> ThreadStoreResult<AppendResult> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ensure_thread_exists(&tx, thread_id)?;
        let sequence = item.sequence.unwrap_or(next_sequence(&tx, thread_id)?);
        let item_id = item
            .item_id
            .take()
            .unwrap_or_else(|| format!("item_{thread_id}_{sequence:06}"));
        let sequence_i64 = integers::u64_to_i64("sequence", sequence)?;
        let turn_id = turn_id.map(str::to_string).or(item.turn_id.take());
        if let Some((existing_thread_id, existing_turn_id, existing_sequence)) =
            existing_item_identity(&tx, &item_id)?
        {
            if existing_thread_id == thread_id {
                tx.commit()?;
                return Ok(AppendResult {
                    thread_id: thread_id.to_string(),
                    turn_id: existing_turn_id,
                    item_id,
                    sequence: existing_sequence,
                });
            }
            return Err(ThreadStoreError::InvalidInput(format!(
                "item id already exists in another thread: {item_id}"
            )));
        }
        if let Some(turn_id) = &turn_id {
            upsert_turn(&tx, thread_id, turn_id, &item, sequence_i64)?;
        }

        tx.execute(
            r#"
                INSERT INTO items(
                    item_id, thread_id, turn_id, item_type, role, status, source, created_at,
                    sequence, legacy_uuid, payload_ref, payload_json, search_text, partial_lineage
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
            params![
                item_id,
                thread_id,
                turn_id,
                item.item_type,
                item.role,
                item.status,
                item.source,
                item.created_at.to_rfc3339(),
                sequence_i64,
                item.legacy_uuid,
                item.payload_ref,
                item.payload_json.map(|value| value.to_string()),
                item.search_text,
                if item.partial_lineage { 1 } else { 0 }
            ],
        )?;
        tx.execute(
            "UPDATE threads SET updated_at = ?1 WHERE thread_id = ?2",
            params![Utc::now().to_rfc3339(), thread_id],
        )?;
        tx.commit()?;
        Ok(AppendResult {
            thread_id: thread_id.to_string(),
            turn_id,
            item_id,
            sequence,
        })
    }

    async fn flush(&self, thread_id: &str) -> ThreadStoreResult<()> {
        self.with_conn(|conn| {
            ensure_thread_exists(conn, thread_id)?;
            Ok(())
        })
    }

    async fn read_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadSnapshot> {
        self.with_conn(|conn| {
            let thread = row_to_thread(conn, thread_id)?;
            if thread.deleted_at.is_some() {
                return Err(ThreadStoreError::ThreadNotFound(thread_id.to_string()));
            }
            Ok(ThreadSnapshot {
                lineage: row_to_lineage(conn, thread_id)?,
                turns: rows_to_turns(conn, thread_id)?,
                items: rows_to_items(conn, thread_id)?,
                thread,
            })
        })
    }

    async fn list_threads(&self, query: ThreadListQuery) -> ThreadStoreResult<Page<ThreadRecord>> {
        self.with_conn(|conn| list_threads(conn, query))
    }

    async fn search_threads(&self, query: SearchQuery) -> ThreadStoreResult<Page<SearchHit>> {
        self.with_conn(|conn| search_threads(conn, query))
    }

    async fn archive_thread(
        &self,
        thread_id: &str,
        reason: Option<String>,
    ) -> ThreadStoreResult<ThreadRecord> {
        self.with_conn(|conn| {
            ensure_thread_exists(conn, thread_id)?;
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE threads SET archived_at = ?1, updated_at = ?1 WHERE thread_id = ?2",
                params![now, thread_id],
            )?;
            if let Some(reason) = reason {
                conn.execute(
                    "UPDATE threads SET metadata_json = json_set(metadata_json, '$.archive_reason', ?1) WHERE thread_id = ?2",
                    params![reason, thread_id],
                )?;
            }
            row_to_thread(conn, thread_id)
        })
    }

    async fn unarchive_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadRecord> {
        self.with_conn(|conn| {
            ensure_thread_exists(conn, thread_id)?;
            conn.execute(
                "UPDATE threads SET archived_at = NULL, updated_at = ?1 WHERE thread_id = ?2",
                params![Utc::now().to_rfc3339(), thread_id],
            )?;
            row_to_thread(conn, thread_id)
        })
    }

    async fn delete_thread(
        &self,
        thread_id: &str,
        mode: DeleteMode,
    ) -> ThreadStoreResult<DeleteResult> {
        self.with_conn(|conn| {
            ensure_thread_exists(conn, thread_id)?;
            let mut payload_files_deleted = 0;
            let mut payload_delete_errors = Vec::new();
            if mode == DeleteMode::MetadataAndPayloadFiles {
                let report =
                    delete_payload_refs(payload_refs(conn, thread_id)?, self.payload_root());
                payload_files_deleted = report.files_deleted;
                payload_delete_errors = report.errors;
            }

            let now = Utc::now().to_rfc3339();
            let metadata_deleted =
                mode != DeleteMode::MetadataAndPayloadFiles || payload_delete_errors.is_empty();
            if metadata_deleted {
                let payload_deleted = mode == DeleteMode::MetadataAndPayloadFiles;
                conn.execute(
                    r#"
                    UPDATE threads
                    SET deleted_at = ?1,
                        payload_deleted_at = CASE WHEN ?2 = 1 THEN ?1 ELSE payload_deleted_at END,
                        updated_at = ?1
                    WHERE thread_id = ?3
                    "#,
                    params![now, if payload_deleted { 1 } else { 0 }, thread_id],
                )?;
            }
            Ok(DeleteResult {
                thread_id: thread_id.to_string(),
                metadata_deleted,
                payload_files_deleted,
                payload_delete_errors,
            })
        })
    }

    async fn backfill_legacy(
        &self,
        source_path: &Path,
        options: BackfillOptions,
    ) -> ThreadStoreResult<BackfillReport> {
        backfill_legacy_path(self, source_path, options).await
    }

    async fn detect_recovery(&self) -> ThreadStoreResult<RecoveryReport> {
        detect_startup_issues(self).await
    }
}

fn ensure_thread_exists(conn: &Connection, thread_id: &str) -> ThreadStoreResult<()> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM threads WHERE thread_id = ?1 AND deleted_at IS NULL",
            params![thread_id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(ThreadStoreError::ThreadNotFound(thread_id.to_string()));
    }
    Ok(())
}

fn next_sequence(conn: &Connection, thread_id: &str) -> ThreadStoreResult<u64> {
    let next: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sequence), -1) + 1 FROM items WHERE thread_id = ?1",
        params![thread_id],
        |row| row.get(0),
    )?;
    integers::stored_i64_to_u64("items", "sequence", thread_id, next)
}

fn upsert_turn(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
    item: &ThreadItemInput,
    sequence: i64,
) -> ThreadStoreResult<()> {
    ensure_turn_ownership(conn, thread_id, turn_id)?;
    let status = match item.item_type.as_str() {
        "turn" => item.status.as_deref().unwrap_or("started"),
        _ => "started",
    };
    let completed_at = match status {
        "completed" | "failed" | "interrupted" => Some(item.created_at.to_rfc3339()),
        _ => None,
    };
    conn.execute(
        r#"
        INSERT INTO turns(turn_id, thread_id, status, started_at, completed_at, sequence_start, sequence_end)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(turn_id) DO UPDATE SET
            status = excluded.status,
            completed_at = COALESCE(excluded.completed_at, turns.completed_at),
            sequence_end = MAX(COALESCE(turns.sequence_end, excluded.sequence_end), excluded.sequence_end)
        "#,
        params![
            turn_id,
            thread_id,
            status,
            item.created_at.to_rfc3339(),
            completed_at,
            sequence,
            sequence
        ],
    )?;
    Ok(())
}

fn ensure_turn_ownership(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
) -> ThreadStoreResult<()> {
    let owner: Option<String> = conn
        .query_row(
            "SELECT thread_id FROM turns WHERE turn_id = ?1",
            params![turn_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(owner) = owner {
        if owner != thread_id {
            return Err(ThreadStoreError::InvalidInput(format!(
                "turn id already exists in another thread: {turn_id}"
            )));
        }
    }
    Ok(())
}
