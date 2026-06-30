//! Persistent thread index and recovery store.
//!
//! GH-82 keeps this module core-only: it defines and tests the store boundary
//! without routing CLI, SDK, or runtime execution through it yet.

mod backfill;
mod backfill_support;
mod error;
mod recovery;
mod sqlite;
#[cfg(test)]
mod tests;
mod traits;
mod types;

pub use backfill::backfill_legacy_path;
pub use error::{ThreadStoreError, ThreadStoreResult};
pub use recovery::detect_startup_issues;
pub use sqlite::SqliteThreadStore;
pub use traits::ThreadStore;
pub use types::{
    AppendResult, BackfillOptions, BackfillReport, DeleteMode, DeleteResult, LegacySourceKind,
    Page, RecoveryIssue, RecoveryIssueCode, RecoveryReport, SearchHit, SearchQuery,
    StoreErrorRecord, ThreadId, ThreadItemInput, ThreadItemRecord, ThreadLineage, ThreadListQuery,
    ThreadRecord, ThreadSnapshot, ThreadStatus, TurnId, TurnRecord, TurnStatus,
};
