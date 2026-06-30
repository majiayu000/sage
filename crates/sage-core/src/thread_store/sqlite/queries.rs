use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::integers::{stored_i64_to_u64, stored_optional_i64_to_u64, u64_to_i64};
use crate::thread_store::error::{ThreadStoreError, ThreadStoreResult};
use crate::thread_store::types::{
    Page, SearchHit, SearchQuery, ThreadItemRecord, ThreadLineage, ThreadListQuery, ThreadRecord,
    ThreadStatus, TurnRecord, TurnStatus,
};

struct StoredThread {
    thread_id: String,
    legacy_session_id: Option<String>,
    title: Option<String>,
    cwd: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    status: String,
    archived_at: Option<String>,
    deleted_at: Option<String>,
    payload_deleted_at: Option<String>,
    created_at: String,
    updated_at: String,
    metadata_json: String,
}

struct StoredTurn {
    turn_id: String,
    thread_id: String,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    sequence_start: Option<i64>,
    sequence_end: Option<i64>,
}

struct StoredItem {
    item_id: String,
    thread_id: String,
    turn_id: Option<String>,
    item_type: String,
    role: Option<String>,
    status: Option<String>,
    source: String,
    created_at: String,
    sequence: i64,
    legacy_uuid: Option<String>,
    payload_ref: Option<String>,
    payload_json: Option<String>,
    search_text: Option<String>,
    partial_lineage: i64,
}

pub(super) fn existing_item_identity(
    conn: &Connection,
    item_id: &str,
) -> ThreadStoreResult<Option<(String, Option<String>, u64)>> {
    let row = conn
        .query_row(
            "SELECT thread_id, turn_id, sequence FROM items WHERE item_id = ?1",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?;
    row.map(|(thread_id, turn_id, sequence)| {
        Ok((
            thread_id,
            turn_id,
            stored_i64_to_u64("items", "sequence", item_id, sequence)?,
        ))
    })
    .transpose()
}

pub(super) fn row_to_thread(conn: &Connection, thread_id: &str) -> ThreadStoreResult<ThreadRecord> {
    let stored = conn
        .query_row(
            r#"
            SELECT thread_id, legacy_session_id, title, cwd, provider, model, status, archived_at,
                   deleted_at, payload_deleted_at, created_at, updated_at, metadata_json
            FROM threads WHERE thread_id = ?1
            "#,
            params![thread_id],
            stored_thread_from_row,
        )
        .optional()?
        .ok_or_else(|| ThreadStoreError::ThreadNotFound(thread_id.to_string()))?;
    thread_from_stored(stored)
}

pub(super) fn row_to_lineage(
    conn: &Connection,
    thread_id: &str,
) -> ThreadStoreResult<Option<ThreadLineage>> {
    conn.query_row(
        r#"
        SELECT thread_id, parent_thread_id, parent_turn_id, parent_item_id, fork_mode
        FROM thread_lineage WHERE thread_id = ?1
        "#,
        params![thread_id],
        |row| {
            Ok(ThreadLineage {
                thread_id: row.get(0)?,
                parent_thread_id: row.get(1)?,
                parent_turn_id: row.get(2)?,
                parent_item_id: row.get(3)?,
                fork_mode: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn rows_to_turns(
    conn: &Connection,
    thread_id: &str,
) -> ThreadStoreResult<Vec<TurnRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT turn_id, thread_id, status, started_at, completed_at, sequence_start, sequence_end
        FROM turns WHERE thread_id = ?1 ORDER BY sequence_start ASC, turn_id ASC
        "#,
    )?;
    let rows = stmt.query_map(params![thread_id], stored_turn_from_row)?;
    let mut turns = Vec::new();
    for row in rows {
        turns.push(turn_from_stored(row?)?);
    }
    Ok(turns)
}

pub(super) fn rows_to_items(
    conn: &Connection,
    thread_id: &str,
) -> ThreadStoreResult<Vec<ThreadItemRecord>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT item_id, thread_id, turn_id, item_type, role, status, source, created_at,
               sequence, legacy_uuid, payload_ref, payload_json, search_text, partial_lineage
        FROM items WHERE thread_id = ?1 ORDER BY sequence ASC, item_id ASC
        "#,
    )?;
    let rows = stmt.query_map(params![thread_id], stored_item_from_row)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(item_from_stored(row?)?);
    }
    Ok(items)
}

pub(super) fn list_threads(
    conn: &Connection,
    query: ThreadListQuery,
) -> ThreadStoreResult<Page<ThreadRecord>> {
    let where_clause = if query.include_archived {
        "deleted_at IS NULL"
    } else {
        "deleted_at IS NULL AND archived_at IS NULL"
    };
    let total = count_threads(conn, where_clause)?;
    let sql = format!(
        "SELECT thread_id FROM threads WHERE {where_clause} ORDER BY updated_at DESC, thread_id ASC LIMIT ?1 OFFSET ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let limit = u64_to_i64("pagination limit", query.limit)?;
    let offset = u64_to_i64("pagination offset", query.offset)?;
    let rows = stmt.query_map(params![limit, offset], |row| row.get::<_, String>(0))?;
    let ids = rows.collect::<Result<Vec<_>, _>>()?;
    let items = ids
        .iter()
        .map(|id| row_to_thread(conn, id))
        .collect::<ThreadStoreResult<Vec<_>>>()?;
    Ok(Page {
        items,
        total,
        limit: query.limit,
        offset: query.offset,
    })
}

pub(super) fn search_threads(
    conn: &Connection,
    query: SearchQuery,
) -> ThreadStoreResult<Page<SearchHit>> {
    let archive_filter = if query.include_archived {
        ""
    } else {
        "AND threads.archived_at IS NULL"
    };
    let pattern = format!("%{}%", query.query.to_lowercase());
    let total: i64 = conn.query_row(
        &format!(
            r#"
            SELECT COUNT(DISTINCT threads.thread_id)
            FROM threads
            LEFT JOIN items ON items.thread_id = threads.thread_id
            WHERE threads.deleted_at IS NULL {archive_filter}
              AND (
                lower(COALESCE(threads.title, '')) LIKE ?1 OR
                lower(COALESCE(threads.cwd, '')) LIKE ?1 OR
                lower(COALESCE(threads.provider, '')) LIKE ?1 OR
                lower(COALESCE(threads.model, '')) LIKE ?1 OR
                lower(COALESCE(threads.metadata_json, '')) LIKE ?1 OR
                lower(COALESCE(items.search_text, '')) LIKE ?1
              )
            "#
        ),
        params![pattern],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(&format!(
        r#"
        SELECT threads.thread_id
        FROM threads
        LEFT JOIN items ON items.thread_id = threads.thread_id
        WHERE threads.deleted_at IS NULL {archive_filter}
          AND (
            lower(COALESCE(threads.title, '')) LIKE ?1 OR
            lower(COALESCE(threads.cwd, '')) LIKE ?1 OR
            lower(COALESCE(threads.provider, '')) LIKE ?1 OR
            lower(COALESCE(threads.model, '')) LIKE ?1 OR
            lower(COALESCE(threads.metadata_json, '')) LIKE ?1 OR
            lower(COALESCE(items.search_text, '')) LIKE ?1
          )
        GROUP BY threads.thread_id
        ORDER BY MAX(threads.updated_at) DESC, threads.thread_id ASC
        LIMIT ?2 OFFSET ?3
        "#
    ))?;
    let limit = u64_to_i64("pagination limit", query.limit)?;
    let offset = u64_to_i64("pagination offset", query.offset)?;
    let rows = stmt.query_map(params![pattern, limit, offset], |row| {
        row.get::<_, String>(0)
    })?;
    let mut hits = Vec::new();
    for row in rows {
        let thread_id = row?;
        let (matched_item_id, snippet) = matching_item(conn, &thread_id, &pattern)?;
        hits.push(SearchHit {
            thread: row_to_thread(conn, &thread_id)?,
            matched_item_id,
            snippet,
        });
    }
    Ok(Page {
        items: hits,
        total: total as u64,
        limit: query.limit,
        offset: query.offset,
    })
}

pub(super) fn payload_refs(conn: &Connection, thread_id: &str) -> ThreadStoreResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT payload_ref FROM items WHERE thread_id = ?1")?;
    let rows = stmt.query_map(params![thread_id], |row| row.get::<_, Option<String>>(0))?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn stored_thread_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredThread> {
    Ok(StoredThread {
        thread_id: row.get(0)?,
        legacy_session_id: row.get(1)?,
        title: row.get(2)?,
        cwd: row.get(3)?,
        provider: row.get(4)?,
        model: row.get(5)?,
        status: row.get(6)?,
        archived_at: row.get(7)?,
        deleted_at: row.get(8)?,
        payload_deleted_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        metadata_json: row.get(12)?,
    })
}

fn stored_turn_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredTurn> {
    Ok(StoredTurn {
        turn_id: row.get(0)?,
        thread_id: row.get(1)?,
        status: row.get(2)?,
        started_at: row.get(3)?,
        completed_at: row.get(4)?,
        sequence_start: row.get(5)?,
        sequence_end: row.get(6)?,
    })
}

fn stored_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredItem> {
    Ok(StoredItem {
        item_id: row.get(0)?,
        thread_id: row.get(1)?,
        turn_id: row.get(2)?,
        item_type: row.get(3)?,
        role: row.get(4)?,
        status: row.get(5)?,
        source: row.get(6)?,
        created_at: row.get(7)?,
        sequence: row.get(8)?,
        legacy_uuid: row.get(9)?,
        payload_ref: row.get(10)?,
        payload_json: row.get(11)?,
        search_text: row.get(12)?,
        partial_lineage: row.get(13)?,
    })
}

fn thread_from_stored(stored: StoredThread) -> ThreadStoreResult<ThreadRecord> {
    let id = stored.thread_id.clone();
    let metadata = parse_json_map("threads", "metadata_json", &id, &stored.metadata_json)?;
    Ok(ThreadRecord {
        thread_id: stored.thread_id,
        legacy_session_id: stored.legacy_session_id,
        title: stored.title,
        cwd: stored.cwd.map(PathBuf::from),
        provider: stored.provider,
        model: stored.model,
        status: ThreadStatus::from_store(&stored.status)
            .ok_or_else(|| invalid_stored("threads", "status", &id, "unknown thread status"))?,
        archived_at: parse_optional_time("threads", "archived_at", &id, stored.archived_at)?,
        deleted_at: parse_optional_time("threads", "deleted_at", &id, stored.deleted_at)?,
        payload_deleted_at: parse_optional_time(
            "threads",
            "payload_deleted_at",
            &id,
            stored.payload_deleted_at,
        )?,
        created_at: parse_time("threads", "created_at", &id, &stored.created_at)?,
        updated_at: parse_time("threads", "updated_at", &id, &stored.updated_at)?,
        metadata,
    })
}

fn turn_from_stored(stored: StoredTurn) -> ThreadStoreResult<TurnRecord> {
    let id = stored.turn_id.clone();
    Ok(TurnRecord {
        turn_id: stored.turn_id,
        thread_id: stored.thread_id,
        status: TurnStatus::from_store(&stored.status)
            .ok_or_else(|| invalid_stored("turns", "status", &id, "unknown turn status"))?,
        started_at: parse_time("turns", "started_at", &id, &stored.started_at)?,
        completed_at: parse_optional_time("turns", "completed_at", &id, stored.completed_at)?,
        sequence_start: stored_optional_i64_to_u64(
            "turns",
            "sequence_start",
            &id,
            stored.sequence_start,
        )?,
        sequence_end: stored_optional_i64_to_u64(
            "turns",
            "sequence_end",
            &id,
            stored.sequence_end,
        )?,
    })
}

fn item_from_stored(stored: StoredItem) -> ThreadStoreResult<ThreadItemRecord> {
    let id = stored.item_id.clone();
    let payload_json = match stored.payload_json {
        Some(json) => Some(parse_json_value("items", "payload_json", &id, &json)?),
        None => None,
    };
    Ok(ThreadItemRecord {
        item_id: stored.item_id,
        thread_id: stored.thread_id,
        turn_id: stored.turn_id,
        item_type: stored.item_type,
        role: stored.role,
        status: stored.status,
        source: stored.source,
        created_at: parse_time("items", "created_at", &id, &stored.created_at)?,
        sequence: stored_i64_to_u64("items", "sequence", &id, stored.sequence)?,
        legacy_uuid: stored.legacy_uuid,
        payload_ref: stored.payload_ref,
        payload_json,
        search_text: stored.search_text,
        partial_lineage: stored.partial_lineage != 0,
    })
}

fn matching_item(
    conn: &Connection,
    thread_id: &str,
    pattern: &str,
) -> ThreadStoreResult<(Option<String>, Option<String>)> {
    let row = conn
        .query_row(
            r#"
            SELECT item_id, search_text
            FROM items
            WHERE thread_id = ?1 AND lower(COALESCE(search_text, '')) LIKE ?2
            ORDER BY sequence ASC, item_id ASC
            LIMIT 1
            "#,
            params![thread_id, pattern],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    Ok(match row {
        Some((item_id, snippet)) => (Some(item_id), snippet),
        None => (None, None),
    })
}

fn count_threads(conn: &Connection, where_clause: &str) -> ThreadStoreResult<u64> {
    let sql = format!("SELECT COUNT(*) FROM threads WHERE {where_clause}");
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(count as u64)
}

fn parse_json_map(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: &str,
) -> ThreadStoreResult<BTreeMap<String, Value>> {
    serde_json::from_str(value).map_err(|err| invalid_stored(table, field, id, err))
}

fn parse_json_value(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: &str,
) -> ThreadStoreResult<Value> {
    serde_json::from_str(value).map_err(|err| invalid_stored(table, field, id, err))
}

fn parse_time(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: &str,
) -> ThreadStoreResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| invalid_stored(table, field, id, err))
}

fn parse_optional_time(
    table: &'static str,
    field: &'static str,
    id: &str,
    value: Option<String>,
) -> ThreadStoreResult<Option<DateTime<Utc>>> {
    value
        .as_deref()
        .map(|value| parse_time(table, field, id, value))
        .transpose()
}

fn invalid_stored(
    table: &'static str,
    field: &'static str,
    id: &str,
    err: impl std::fmt::Display,
) -> ThreadStoreError {
    ThreadStoreError::InvalidStoredData {
        table,
        field,
        id: id.to_string(),
        message: err.to_string(),
    }
}
