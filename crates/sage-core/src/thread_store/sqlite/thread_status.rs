use chrono::Utc;
use rusqlite::params;

use super::SqliteThreadStore;
use crate::thread_store::{ThreadStatus, ThreadStoreResult};

pub(super) fn update_thread_status_row(
    store: &SqliteThreadStore,
    thread_id: &str,
    status: ThreadStatus,
) -> ThreadStoreResult<()> {
    store.with_conn(|conn| {
        conn.execute(
            "UPDATE threads SET status = ?1, updated_at = ?2 WHERE thread_id = ?3",
            params![status.as_str(), Utc::now().to_rfc3339(), thread_id],
        )?;
        Ok(())
    })
}
