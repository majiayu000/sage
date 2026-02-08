//! Session-based trajectory recording with JSONL storage
//!
//! Storage structure:
//! ```text
//! ~/.sage/projects/{escaped-cwd}/
//! ├── {session-id}.jsonl
//! └── ...
//! ```
//!
//! Each entry is appended immediately for crash safety.

pub mod entry;
pub mod replayer;
pub mod session;

pub use entry::{SessionEntry, TokenUsage};
pub use replayer::{SessionReplayer, SessionSummary};
pub use session::{SessionInfo, SessionRecorder};

use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Initialize a session recorder for the given working directory.
///
/// Returns `Some(Arc<Mutex<SessionRecorder>>)` on success, `None` on failure (logged as warning).
pub fn init_session_recorder(working_dir: &Path) -> Option<Arc<Mutex<SessionRecorder>>> {
    match SessionRecorder::new(working_dir) {
        Ok(recorder) => Some(Arc::new(Mutex::new(recorder))),
        Err(e) => {
            tracing::warn!("Failed to initialize session recorder: {}", e);
            None
        }
    }
}
