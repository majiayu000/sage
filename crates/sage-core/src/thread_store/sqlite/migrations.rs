use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};

use crate::thread_store::error::{ThreadStoreError, ThreadStoreResult};

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

pub fn migrate(conn: &Connection) -> ThreadStoreResult<()> {
    let version = current_version(conn)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(ThreadStoreError::SchemaVersionMismatch {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if version == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS thread_store_schema (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS threads (
            thread_id TEXT PRIMARY KEY,
            legacy_session_id TEXT,
            title TEXT,
            cwd TEXT,
            provider TEXT,
            model TEXT,
            status TEXT NOT NULL,
            archived_at TEXT,
            deleted_at TEXT,
            payload_deleted_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            metadata_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS turns (
            turn_id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            sequence_start INTEGER,
            sequence_end INTEGER,
            FOREIGN KEY(thread_id) REFERENCES threads(thread_id)
        );

        CREATE TABLE IF NOT EXISTS items (
            item_id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            turn_id TEXT,
            item_type TEXT NOT NULL,
            role TEXT,
            status TEXT,
            source TEXT NOT NULL,
            created_at TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            legacy_uuid TEXT,
            payload_ref TEXT,
            payload_json TEXT,
            search_text TEXT,
            partial_lineage INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(thread_id) REFERENCES threads(thread_id),
            FOREIGN KEY(turn_id) REFERENCES turns(turn_id)
        );

        CREATE TABLE IF NOT EXISTS thread_lineage (
            thread_id TEXT PRIMARY KEY,
            parent_thread_id TEXT,
            parent_turn_id TEXT,
            parent_item_id TEXT,
            fork_mode TEXT,
            FOREIGN KEY(thread_id) REFERENCES threads(thread_id)
        );

        CREATE TABLE IF NOT EXISTS legacy_sources (
            source_id TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            imported_at TEXT NOT NULL,
            checksum TEXT NOT NULL,
            import_status TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS store_errors (
            error_id TEXT PRIMARY KEY,
            thread_id TEXT,
            source_id TEXT,
            code TEXT NOT NULL,
            message TEXT NOT NULL,
            details TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(thread_id) REFERENCES threads(thread_id),
            FOREIGN KEY(source_id) REFERENCES legacy_sources(source_id)
        );

        CREATE INDEX IF NOT EXISTS idx_threads_updated
            ON threads(updated_at DESC, thread_id ASC);
        CREATE INDEX IF NOT EXISTS idx_items_thread_sequence
            ON items(thread_id, sequence ASC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_items_thread_sequence_unique
            ON items(thread_id, sequence);
        CREATE INDEX IF NOT EXISTS idx_items_search_text
            ON items(search_text);
        "#,
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO thread_store_schema(version, applied_at) VALUES (?1, ?2)",
        params![CURRENT_SCHEMA_VERSION, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn current_version(conn: &Connection) -> ThreadStoreResult<i64> {
    let table_exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'thread_store_schema'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if table_exists.is_none() {
        return Ok(0);
    }

    let version = conn
        .query_row(
            "SELECT version FROM thread_store_schema ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    Ok(version)
}
