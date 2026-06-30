use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

use crate::session::types::unified::SessionMessage;

use super::error::ThreadStoreResult;
use super::sqlite::SqliteThreadStore;
use super::types::StoreErrorRecord;

pub(super) fn store_error(
    source_id: Option<String>,
    thread_id: Option<String>,
    code: &str,
    message: String,
    details: Option<String>,
) -> StoreErrorRecord {
    StoreErrorRecord {
        error_id: Uuid::new_v4().to_string(),
        thread_id,
        source_id,
        code: code.to_string(),
        message,
        details,
        created_at: Utc::now(),
    }
}

pub(super) fn parse_legacy_time_at_line(
    store: &SqliteThreadStore,
    path: &Path,
    line_no: usize,
    source_id: &str,
    thread_id: Option<&str>,
    value: &str,
    errors: &mut Vec<StoreErrorRecord>,
) -> ThreadStoreResult<Option<DateTime<Utc>>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(dt) => Ok(Some(dt.with_timezone(&Utc))),
        Err(err) => {
            let error = store_error(
                Some(source_id.to_string()),
                thread_id.map(str::to_string),
                "corrupt_jsonl",
                format!("invalid legacy timestamp at line {line_no}"),
                Some(format!("{}: {err}", path.display())),
            );
            store.record_store_error(&error)?;
            errors.push(error);
            Ok(None)
        }
    }
}

pub(super) fn first_session_id_from_messages(path: &Path) -> ThreadStoreResult<Option<String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(first_session_id_from_content(&content))
}

pub(super) fn first_session_id_from_content(content: &str) -> Option<String> {
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<SessionMessage>(line).ok())
        .map(|message| message.session_id)
        .next()
}

pub(super) fn fallback_thread_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("legacy_thread")
        .to_string()
}

pub(super) fn source_id(path: &Path, checksum: &str) -> String {
    let short = checksum.get(..16).unwrap_or(checksum);
    format!("source_{}_{}", fallback_thread_id(path), short)
}

pub(super) fn legacy_ref(path: &Path, line_no: usize) -> String {
    format!("legacy_jsonl:{}:{line_no}", path.display())
}

pub(super) fn checksum_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn checksum_paths<'a>(paths: impl IntoIterator<Item = &'a Path>) -> String {
    let mut hasher = Sha256::new();
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            hasher.update(bytes);
        }
    }
    format!("{:x}", hasher.finalize())
}
