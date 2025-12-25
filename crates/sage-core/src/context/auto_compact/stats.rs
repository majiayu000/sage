//! Statistics tracking for auto-compact operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Statistics for auto-compact operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoCompactStats {
    /// Total number of auto-compactions performed
    pub total_compactions: u64,
    /// Total tokens saved across all compactions
    pub total_tokens_saved: u64,
    /// Total messages compacted
    pub total_messages_compacted: u64,
    /// Number of times compaction was skipped (not needed)
    pub skipped_count: u64,
    /// Last compaction time
    pub last_compaction: Option<DateTime<Utc>>,
    /// Last compact ID
    pub last_compact_id: Option<Uuid>,
}
