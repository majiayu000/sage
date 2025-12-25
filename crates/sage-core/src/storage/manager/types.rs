//! Storage manager types and statistics

use crate::storage::backend::BackendType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connected to primary database
    Primary,
    /// Connected to fallback database
    Fallback,
    /// Not connected
    #[default]
    Disconnected,
    /// Reconnecting
    Reconnecting,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary => write!(f, "Primary (PostgreSQL)"),
            Self::Fallback => write!(f, "Fallback (SQLite)"),
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Reconnecting => write!(f, "Reconnecting..."),
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total queries executed
    pub total_queries: u64,
    /// Successful queries
    pub successful_queries: u64,
    /// Failed queries
    pub failed_queries: u64,
    /// Times fallback was triggered
    pub fallback_count: u64,
    /// Times reconnected to primary
    pub reconnect_count: u64,
    /// Current backend type
    pub backend_type: Option<BackendType>,
    /// Connection status
    pub status: ConnectionStatus,
    /// Last error message
    pub last_error: Option<String>,
    /// Connected since
    pub connected_since: Option<DateTime<Utc>>,
}

/// Health check information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthInfo {
    /// Is connected
    pub connected: bool,
    /// Current backend type
    pub backend_type: Option<BackendType>,
    /// Connection status
    pub status: ConnectionStatus,
    /// Uptime duration
    pub uptime: Option<chrono::Duration>,
    /// Total queries executed
    pub total_queries: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Number of times fallback was triggered
    pub fallback_count: u64,
}
