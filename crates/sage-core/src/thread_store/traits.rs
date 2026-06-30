use async_trait::async_trait;

use super::error::ThreadStoreResult;
use super::types::{
    AppendResult, BackfillOptions, BackfillReport, DeleteMode, DeleteResult, Page, RecoveryReport,
    SearchHit, SearchQuery, ThreadItemInput, ThreadLineage, ThreadListQuery, ThreadRecord,
    ThreadSnapshot, ThreadStatus,
};

#[async_trait]
pub trait ThreadStore: Send + Sync {
    fn registry_key(&self) -> Option<String> {
        None
    }

    async fn create_thread(&self, record: ThreadRecord) -> ThreadStoreResult<ThreadRecord>;

    async fn set_lineage(&self, lineage: ThreadLineage) -> ThreadStoreResult<ThreadLineage>;

    async fn set_thread_status(
        &self,
        thread_id: &str,
        status: ThreadStatus,
    ) -> ThreadStoreResult<()>;

    async fn resume_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadSnapshot>;

    async fn append_event(
        &self,
        thread_id: &str,
        turn_id: Option<&str>,
        item: ThreadItemInput,
    ) -> ThreadStoreResult<AppendResult>;

    async fn flush(&self, thread_id: &str) -> ThreadStoreResult<()>;

    async fn read_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadSnapshot>;

    async fn list_threads(&self, query: ThreadListQuery) -> ThreadStoreResult<Page<ThreadRecord>>;

    async fn search_threads(&self, query: SearchQuery) -> ThreadStoreResult<Page<SearchHit>>;

    async fn archive_thread(
        &self,
        thread_id: &str,
        reason: Option<String>,
    ) -> ThreadStoreResult<ThreadRecord>;

    async fn unarchive_thread(&self, thread_id: &str) -> ThreadStoreResult<ThreadRecord>;

    async fn delete_thread(
        &self,
        thread_id: &str,
        mode: DeleteMode,
    ) -> ThreadStoreResult<DeleteResult>;

    async fn backfill_legacy(
        &self,
        source_path: &std::path::Path,
        options: BackfillOptions,
    ) -> ThreadStoreResult<BackfillReport>;

    async fn detect_recovery(&self) -> ThreadStoreResult<RecoveryReport>;
}
