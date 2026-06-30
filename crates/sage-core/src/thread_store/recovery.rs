use rusqlite::params;

use super::error::ThreadStoreResult;
use super::sqlite::SqliteThreadStore;
use super::types::{RecoveryIssue, RecoveryIssueCode, RecoveryReport};

pub async fn detect_startup_issues(store: &SqliteThreadStore) -> ThreadStoreResult<RecoveryReport> {
    store.with_conn(|conn| {
        let mut issues = Vec::new();
        let mut stmt = conn.prepare(
            r#"
            SELECT thread_id, turn_id
            FROM turns
            WHERE status = 'started' AND completed_at IS NULL
            ORDER BY thread_id ASC, turn_id ASC
            "#,
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (thread_id, turn_id) = row?;
            issues.push(RecoveryIssue {
                code: RecoveryIssueCode::IncompleteTurn,
                thread_id: Some(thread_id),
                turn_id: Some(turn_id),
                message: "turn was started but never completed".to_string(),
            });
        }

        let mut stmt = conn.prepare(
            r#"
            SELECT thread_id, code, message
            FROM store_errors
            WHERE code IN ('corrupt_jsonl', 'missing_metadata', 'schema_version_mismatch')
            ORDER BY created_at ASC, error_id ASC
            "#,
        )?;
        let rows = stmt.query_map(params![], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (thread_id, code, message) = row?;
            issues.push(RecoveryIssue {
                code: match code.as_str() {
                    "missing_metadata" => RecoveryIssueCode::MissingMetadata,
                    "schema_version_mismatch" => RecoveryIssueCode::SchemaVersionMismatch,
                    _ => RecoveryIssueCode::CorruptLegacySource,
                },
                thread_id,
                turn_id: None,
                message,
            });
        }

        Ok(RecoveryReport { issues })
    })
}
