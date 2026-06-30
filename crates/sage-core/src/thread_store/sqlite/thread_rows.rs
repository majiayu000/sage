use rusqlite::{Connection, ErrorCode, params};

use crate::thread_store::error::{ThreadStoreError, ThreadStoreResult};
use crate::thread_store::types::ThreadRecord;

pub(super) fn insert_thread(conn: &Connection, record: &ThreadRecord) -> ThreadStoreResult<()> {
    execute_thread(conn, insert_sql(), record)
        .map(|_| ())
        .map_err(|err| map_insert_error(err, &record.thread_id))
}

pub(super) fn upsert_thread(conn: &Connection, record: &ThreadRecord) -> ThreadStoreResult<()> {
    execute_thread(conn, upsert_sql(), record)?;
    Ok(())
}

fn execute_thread(
    conn: &Connection,
    sql: &str,
    record: &ThreadRecord,
) -> Result<usize, rusqlite::Error> {
    let cwd = record
        .cwd
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    let metadata_json = serde_json::to_string(&record.metadata)
        .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?;
    conn.execute(
        sql,
        params![
            record.thread_id,
            record.legacy_session_id,
            record.title,
            cwd,
            record.provider,
            record.model,
            record.status.as_str(),
            record.archived_at.map(|dt| dt.to_rfc3339()),
            record.deleted_at.map(|dt| dt.to_rfc3339()),
            record.payload_deleted_at.map(|dt| dt.to_rfc3339()),
            record.created_at.to_rfc3339(),
            record.updated_at.to_rfc3339(),
            metadata_json
        ],
    )
}

fn insert_sql() -> &'static str {
    r#"
    INSERT INTO threads(
        thread_id, legacy_session_id, title, cwd, provider, model, status, archived_at,
        deleted_at, payload_deleted_at, created_at, updated_at, metadata_json
    )
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
    "#
}

fn upsert_sql() -> &'static str {
    r#"
    INSERT INTO threads(
        thread_id, legacy_session_id, title, cwd, provider, model, status, archived_at,
        deleted_at, payload_deleted_at, created_at, updated_at, metadata_json
    )
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
    ON CONFLICT(thread_id) DO UPDATE SET
        legacy_session_id = excluded.legacy_session_id,
        title = excluded.title,
        cwd = excluded.cwd,
        provider = excluded.provider,
        model = excluded.model,
        status = excluded.status,
        updated_at = excluded.updated_at,
        metadata_json = excluded.metadata_json
    "#
}

fn map_insert_error(err: rusqlite::Error, thread_id: &str) -> ThreadStoreError {
    match &err {
        rusqlite::Error::SqliteFailure(sqlite_err, _)
            if sqlite_err.code == ErrorCode::ConstraintViolation =>
        {
            ThreadStoreError::ThreadAlreadyExists(thread_id.to_string())
        }
        _ => ThreadStoreError::Sqlite(err),
    }
}
