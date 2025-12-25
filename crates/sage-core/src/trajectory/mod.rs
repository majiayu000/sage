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
