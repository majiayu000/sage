use serde::{Deserialize, Serialize};

use crate::thread_store::ThreadStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStateMode {
    Ephemeral,
    ThreadStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStateCapabilities {
    pub mode: RuntimeStateMode,
    pub can_persist_threads: bool,
    pub can_resume_threads: bool,
    pub can_fork_threads: bool,
}

impl RuntimeStateCapabilities {
    pub fn ephemeral() -> Self {
        Self {
            mode: RuntimeStateMode::Ephemeral,
            can_persist_threads: false,
            can_resume_threads: false,
            can_fork_threads: false,
        }
    }

    pub fn thread_store() -> Self {
        Self {
            mode: RuntimeStateMode::ThreadStore,
            can_persist_threads: true,
            can_resume_threads: true,
            can_fork_threads: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub thread_id: String,
    pub state: RuntimeStateCapabilities,
    pub thread_status: ThreadStatus,
    pub turn_count: usize,
    pub item_count: usize,
}
