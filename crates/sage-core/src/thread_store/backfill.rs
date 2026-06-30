use std::path::Path;

use crate::session::jsonl_storage::SessionMetadata;
use crate::session::types::unified::SessionMessage;
use crate::trajectory::SessionEntry;

use super::backfill_support::{
    checksum_bytes, checksum_paths, fallback_thread_id, first_session_id_from_content,
    first_session_id_from_messages, legacy_ref, parse_legacy_time_at_line, source_id, store_error,
};
use super::error::ThreadStoreResult;
use super::sqlite::SqliteThreadStore;
use super::traits::ThreadStore;
use super::types::{
    BackfillOptions, BackfillReport, LegacySourceKind, StoreErrorRecord, ThreadItemInput,
    ThreadRecord, ThreadStatus,
};

pub async fn backfill_legacy_path(
    store: &SqliteThreadStore,
    source_path: &Path,
    options: BackfillOptions,
) -> ThreadStoreResult<BackfillReport> {
    match options.source_kind {
        LegacySourceKind::TrajectoryJsonl => backfill_trajectory_jsonl(store, source_path).await,
        LegacySourceKind::SessionDirectory => backfill_session_directory(store, source_path).await,
        LegacySourceKind::SessionMessagesJsonl => {
            backfill_session_messages(store, source_path).await
        }
    }
}

async fn backfill_trajectory_jsonl(
    store: &SqliteThreadStore,
    path: &Path,
) -> ThreadStoreResult<BackfillReport> {
    let content = std::fs::read_to_string(path)?;
    let checksum = checksum_bytes(content.as_bytes());
    let source_id = source_id(path, &checksum);
    store.upsert_legacy_source(
        &source_id,
        path,
        LegacySourceKind::TrajectoryJsonl.as_str(),
        &checksum,
        "started",
    )?;

    let mut current_thread_id = None;
    let mut imported_threads = 0;
    let mut imported_items = 0;
    let mut errors = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        match serde_json::from_str::<SessionEntry>(line) {
            Ok(entry) => match entry {
                SessionEntry::SessionStart {
                    session_id,
                    task,
                    provider,
                    model,
                    cwd,
                    timestamp,
                    ..
                } => {
                    let Some(created_at) = parse_legacy_time_at_line(
                        store,
                        path,
                        line_no,
                        &source_id,
                        None,
                        &timestamp,
                        &mut errors,
                    )?
                    else {
                        continue;
                    };
                    let mut record = ThreadRecord::new(session_id.to_string()).with_title(task);
                    record.legacy_session_id = Some(session_id.to_string());
                    record.provider = Some(provider);
                    record.model = Some(model);
                    record.cwd = Some(cwd.into());
                    record.created_at = created_at;
                    record.updated_at = record.created_at;
                    store.upsert_thread_record(&record)?;
                    current_thread_id = Some(session_id.to_string());
                    imported_threads += 1;
                }
                SessionEntry::SessionEnd {
                    uuid,
                    success,
                    timestamp,
                    ..
                } => {
                    let thread_id = ensure_thread_for_legacy(
                        store,
                        path,
                        &source_id,
                        &mut current_thread_id,
                        &mut imported_threads,
                        &mut errors,
                    )
                    .await?;
                    let Some(created_at) = parse_legacy_time_at_line(
                        store,
                        path,
                        line_no,
                        &source_id,
                        Some(&thread_id),
                        &timestamp,
                        &mut errors,
                    )?
                    else {
                        continue;
                    };
                    store.set_thread_status(
                        &thread_id,
                        if success {
                            ThreadStatus::Completed
                        } else {
                            ThreadStatus::Failed
                        },
                    )?;
                    let mut item = ThreadItemInput::new("session_end");
                    item.item_id = Some(uuid.to_string());
                    item.created_at = created_at;
                    item.source = "legacy".to_string();
                    item.payload_ref = Some(legacy_ref(path, line_no));
                    item.sequence = Some((line_no - 1) as u64);
                    store.append_event(&thread_id, None, item).await?;
                    imported_items += 1;
                }
                other => {
                    let thread_id = ensure_thread_for_legacy(
                        store,
                        path,
                        &source_id,
                        &mut current_thread_id,
                        &mut imported_threads,
                        &mut errors,
                    )
                    .await?;
                    let Some(created_at) = parse_legacy_time_at_line(
                        store,
                        path,
                        line_no,
                        &source_id,
                        current_thread_id.as_deref(),
                        other.timestamp(),
                        &mut errors,
                    )?
                    else {
                        continue;
                    };
                    let item = item_from_session_entry(path, line_no, &other, created_at);
                    let turn_id = item.turn_id.clone();
                    store
                        .append_event(&thread_id, turn_id.as_deref(), item)
                        .await?;
                    imported_items += 1;
                }
            },
            Err(err) => {
                let error = store_error(
                    Some(source_id.clone()),
                    None,
                    "corrupt_jsonl",
                    format!("invalid legacy JSONL record at line {line_no}"),
                    Some(err.to_string()),
                );
                store.record_store_error(&error)?;
                errors.push(error);
            }
        }
    }

    store.upsert_legacy_source(
        &source_id,
        path,
        LegacySourceKind::TrajectoryJsonl.as_str(),
        &checksum,
        if errors.is_empty() {
            "imported"
        } else {
            "partial"
        },
    )?;
    Ok(BackfillReport {
        source_id,
        imported_threads,
        imported_items,
        partial: !errors.is_empty(),
        errors,
    })
}

async fn backfill_session_directory(
    store: &SqliteThreadStore,
    dir: &Path,
) -> ThreadStoreResult<BackfillReport> {
    let metadata_path = dir.join("metadata.json");
    let messages_path = dir.join("messages.jsonl");
    let checksum = checksum_paths([metadata_path.as_path(), messages_path.as_path()]);
    let source_id = source_id(dir, &checksum);
    store.upsert_legacy_source(
        &source_id,
        dir,
        LegacySourceKind::SessionDirectory.as_str(),
        &checksum,
        "started",
    )?;

    let mut errors = Vec::new();
    let imported_threads;
    let thread_id = if metadata_path.exists() {
        let metadata: SessionMetadata =
            serde_json::from_str(&std::fs::read_to_string(&metadata_path)?)?;
        let mut record = ThreadRecord::new(metadata.id.clone());
        record.legacy_session_id = Some(metadata.id.clone());
        record.title = metadata
            .custom_title
            .or(metadata.name)
            .or(metadata.first_prompt);
        record.cwd = Some(metadata.working_directory);
        record.model = metadata.model;
        record.created_at = metadata.created_at;
        record.updated_at = metadata.updated_at;
        store.upsert_thread_record(&record)?;
        imported_threads = 1;
        metadata.id
    } else {
        let thread_id = if messages_path.exists() {
            first_session_id_from_messages(&messages_path)?
                .unwrap_or_else(|| fallback_thread_id(dir))
        } else {
            fallback_thread_id(dir)
        };
        let mut record = ThreadRecord::new(thread_id.clone());
        record.legacy_session_id = Some(thread_id.clone());
        record.status = ThreadStatus::Unknown;
        store.upsert_thread_record(&record)?;
        let error = store_error(
            Some(source_id.clone()),
            Some(thread_id.clone()),
            "missing_metadata",
            "session directory is missing metadata.json".to_string(),
            Some(metadata_path.display().to_string()),
        );
        store.record_store_error(&error)?;
        errors.push(error);
        imported_threads = 1;
        thread_id
    };

    let mut imported_items = 0;
    if messages_path.exists() {
        imported_items +=
            import_session_messages(store, &thread_id, &source_id, &messages_path, &mut errors)
                .await?;
    }

    store.upsert_legacy_source(
        &source_id,
        dir,
        LegacySourceKind::SessionDirectory.as_str(),
        &checksum,
        if errors.is_empty() {
            "imported"
        } else {
            "partial"
        },
    )?;
    Ok(BackfillReport {
        source_id,
        imported_threads,
        imported_items,
        partial: !errors.is_empty(),
        errors,
    })
}

async fn backfill_session_messages(
    store: &SqliteThreadStore,
    path: &Path,
) -> ThreadStoreResult<BackfillReport> {
    let content = std::fs::read_to_string(path)?;
    let checksum = checksum_bytes(content.as_bytes());
    let source_id = source_id(path, &checksum);
    let thread_id =
        first_session_id_from_content(&content).unwrap_or_else(|| fallback_thread_id(path));
    store.upsert_legacy_source(
        &source_id,
        path,
        LegacySourceKind::SessionMessagesJsonl.as_str(),
        &checksum,
        "started",
    )?;
    let mut record = ThreadRecord::new(thread_id.clone());
    record.legacy_session_id = Some(thread_id.clone());
    store.upsert_thread_record(&record)?;
    let mut errors = Vec::new();
    let imported_items =
        import_session_messages(store, &thread_id, &source_id, path, &mut errors).await?;
    store.upsert_legacy_source(
        &source_id,
        path,
        LegacySourceKind::SessionMessagesJsonl.as_str(),
        &checksum,
        if errors.is_empty() {
            "imported"
        } else {
            "partial"
        },
    )?;
    Ok(BackfillReport {
        source_id,
        imported_threads: 1,
        imported_items,
        partial: !errors.is_empty(),
        errors,
    })
}

async fn import_session_messages(
    store: &SqliteThreadStore,
    thread_id: &str,
    source_id: &str,
    path: &Path,
    errors: &mut Vec<StoreErrorRecord>,
) -> ThreadStoreResult<usize> {
    let content = std::fs::read_to_string(path)?;
    let mut imported = 0;
    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        match serde_json::from_str::<SessionMessage>(line) {
            Ok(message) => {
                if message.session_id != thread_id {
                    let error = store_error(
                        Some(source_id.to_string()),
                        Some(thread_id.to_string()),
                        "mixed_session_id",
                        format!("session message at line {line_no} belongs to a different session"),
                        Some(format!("found session_id {}", message.session_id)),
                    );
                    store.record_store_error(&error)?;
                    errors.push(error);
                    continue;
                }
                let partial_lineage = message.parent_uuid.is_none() && !message.is_user();
                let mut item = ThreadItemInput::new(message.message_type.to_string());
                item.item_id = Some(message.uuid.clone());
                item.turn_id = Some(format!("turn_{thread_id}_{line_no:06}"));
                item.role = Some(message.message.role.to_string());
                item.source = "legacy_session".to_string();
                item.created_at = message.timestamp;
                item.legacy_uuid = Some(message.uuid.clone());
                item.payload_ref = Some(legacy_ref(path, line_no));
                item.sequence = Some((line_no - 1) as u64);
                item.search_text = Some(message.message.content);
                item.partial_lineage = partial_lineage;
                let turn_id = item.turn_id.clone();
                store
                    .append_event(thread_id, turn_id.as_deref(), item)
                    .await?;
                imported += 1;
            }
            Err(err) => {
                let error = store_error(
                    Some(source_id.to_string()),
                    Some(thread_id.to_string()),
                    "corrupt_jsonl",
                    format!("invalid session message at line {line_no}"),
                    Some(err.to_string()),
                );
                store.record_store_error(&error)?;
                errors.push(error);
            }
        }
    }
    Ok(imported)
}

async fn ensure_thread_for_legacy(
    store: &SqliteThreadStore,
    path: &Path,
    source_id: &str,
    current_thread_id: &mut Option<String>,
    imported_threads: &mut usize,
    errors: &mut Vec<StoreErrorRecord>,
) -> ThreadStoreResult<String> {
    if let Some(thread_id) = current_thread_id.clone() {
        return Ok(thread_id);
    }
    let thread_id = fallback_thread_id(path);
    let mut record = ThreadRecord::new(thread_id.clone());
    record.status = ThreadStatus::Unknown;
    store.upsert_thread_record(&record)?;
    let error = store_error(
        Some(source_id.to_string()),
        Some(thread_id.clone()),
        "missing_metadata",
        "legacy JSONL did not start with session_start".to_string(),
        Some(path.display().to_string()),
    );
    store.record_store_error(&error)?;
    errors.push(error);
    *current_thread_id = Some(thread_id.clone());
    *imported_threads += 1;
    Ok(thread_id)
}

fn item_from_session_entry(
    path: &Path,
    line_no: usize,
    entry: &SessionEntry,
    created_at: chrono::DateTime<chrono::Utc>,
) -> ThreadItemInput {
    let mut item = ThreadItemInput::new(entry.entry_type());
    item.item_id = Some(entry.uuid().to_string());
    item.turn_id = Some(format!("turn_{}", entry.uuid()));
    item.source = "legacy_trajectory".to_string();
    item.created_at = created_at;
    item.legacy_uuid = Some(entry.uuid().to_string());
    item.payload_ref = Some(legacy_ref(path, line_no));
    item.sequence = Some((line_no - 1) as u64);
    item.partial_lineage = entry.parent_uuid().is_none() && entry.entry_type() != "user";
    item.search_text = match entry {
        SessionEntry::User { content, .. } => Some(content.to_string()),
        SessionEntry::LlmResponse { content, .. } => Some(content.clone()),
        SessionEntry::Error { message, .. } => Some(message.clone()),
        SessionEntry::SessionEnd { final_result, .. } => final_result.clone(),
        _ => None,
    };
    item
}
