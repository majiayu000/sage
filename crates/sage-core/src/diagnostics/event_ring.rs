//! Bounded in-memory diagnostic event ring.

use crate::error::{SageError, SageResult};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    Tool,
    Runtime,
    Provider,
    Sandbox,
    Permission,
    Config,
    Feedback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionClass {
    Public,
    Sensitive,
    Secret,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub kind: DiagnosticEventKind,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    pub severity: DiagnosticSeverity,
    pub redaction_class: RedactionClass,
    pub payload_summary: String,
}

impl DiagnosticEvent {
    pub fn new(
        kind: DiagnosticEventKind,
        source: impl Into<String>,
        severity: DiagnosticSeverity,
        redaction_class: RedactionClass,
        payload_summary: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            kind,
            source: source.into(),
            thread_id: None,
            severity,
            redaction_class,
            payload_summary: payload_summary.into(),
        }
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEventSnapshot {
    pub capacity: usize,
    pub retained_count: usize,
    pub dropped_count: u64,
    pub freshness: DateTime<Utc>,
    pub events: Vec<DiagnosticEvent>,
}

#[derive(Debug)]
pub struct DiagnosticEventRing {
    events: RwLock<VecDeque<DiagnosticEvent>>,
    capacity: usize,
    dropped_count: RwLock<u64>,
}

impl DiagnosticEventRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
            dropped_count: RwLock::new(0),
        }
    }

    pub fn shared(capacity: usize) -> Arc<Self> {
        Arc::new(Self::new(capacity))
    }

    pub fn record(&self, event: DiagnosticEvent) {
        if self.capacity == 0 {
            self.increment_dropped();
            return;
        }

        let mut events = self.events.write();
        if events.len() >= self.capacity {
            events.pop_front();
            self.increment_dropped();
        }
        events.push_back(event);
    }

    pub fn snapshot(&self) -> DiagnosticEventSnapshot {
        let events: Vec<_> = self.events.read().iter().cloned().collect();
        DiagnosticEventSnapshot {
            capacity: self.capacity,
            retained_count: events.len(),
            dropped_count: self.dropped_count(),
            freshness: Utc::now(),
            events,
        }
    }

    pub fn dropped_count(&self) -> u64 {
        *self.dropped_count.read()
    }

    pub fn len(&self) -> usize {
        self.events.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        self.events.write().clear();
        *self.dropped_count.write() = 0;
    }

    fn increment_dropped(&self) {
        let mut dropped = self.dropped_count.write();
        *dropped = dropped.saturating_add(1);
    }
}

impl Default for DiagnosticEventRing {
    fn default() -> Self {
        Self::new(512)
    }
}

static GLOBAL_DIAGNOSTICS: once_cell::sync::Lazy<DiagnosticEventRing> =
    once_cell::sync::Lazy::new(DiagnosticEventRing::default);

pub fn global_diagnostics() -> &'static DiagnosticEventRing {
    &GLOBAL_DIAGNOSTICS
}

pub fn default_diagnostic_event_log_path() -> PathBuf {
    crate::config::default_data_dir_or_warn()
        .join("diagnostics")
        .join("events.jsonl")
}

pub fn append_diagnostic_event_to_default_store(event: &DiagnosticEvent) -> SageResult<()> {
    append_diagnostic_event_to_path(event, default_diagnostic_event_log_path())
}

pub fn append_diagnostic_event_to_path(
    event: &DiagnosticEvent,
    path: impl AsRef<Path>,
) -> SageResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            SageError::io_with_path(error.to_string(), parent.display().to_string())
        })?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| SageError::io_with_path(error.to_string(), path.display().to_string()))?;
    let line = serde_json::to_string(event)
        .map_err(|error| SageError::json(format!("serialize diagnostic event: {error}")))?;
    writeln!(file, "{line}")
        .map_err(|error| SageError::io_with_path(error.to_string(), path.display().to_string()))
}

pub fn persisted_diagnostics_snapshot(capacity: usize) -> SageResult<DiagnosticEventSnapshot> {
    persisted_diagnostics_snapshot_from_path(default_diagnostic_event_log_path(), capacity)
}

pub fn persisted_diagnostics_snapshot_from_path(
    path: impl AsRef<Path>,
    capacity: usize,
) -> SageResult<DiagnosticEventSnapshot> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(DiagnosticEventSnapshot {
            capacity,
            retained_count: 0,
            dropped_count: 0,
            freshness: Utc::now(),
            events: Vec::new(),
        });
    }
    let file = std::fs::File::open(path)
        .map_err(|error| SageError::io_with_path(error.to_string(), path.display().to_string()))?;
    let reader = std::io::BufReader::new(file);
    let mut events = VecDeque::with_capacity(capacity);
    let mut dropped_count = 0_u64;
    for line in reader.lines() {
        let line = line.map_err(|error| {
            SageError::io_with_path(error.to_string(), path.display().to_string())
        })?;
        if line.trim().is_empty() {
            continue;
        }
        let event: DiagnosticEvent = serde_json::from_str(&line)
            .map_err(|error| SageError::json(format!("parse diagnostic event log: {error}")))?;
        if capacity == 0 {
            dropped_count = dropped_count.saturating_add(1);
            continue;
        }
        if events.len() >= capacity {
            events.pop_front();
            dropped_count = dropped_count.saturating_add(1);
        }
        events.push_back(event);
    }
    let events: Vec<_> = events.into_iter().collect();
    Ok(DiagnosticEventSnapshot {
        capacity,
        retained_count: events.len(),
        dropped_count,
        freshness: Utc::now(),
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn event(label: &str) -> DiagnosticEvent {
        DiagnosticEvent::new(
            DiagnosticEventKind::Tool,
            "test",
            DiagnosticSeverity::Info,
            RedactionClass::Public,
            label,
        )
    }

    #[test]
    fn diagnostics_event_ring_retains_capacity_and_dropped_count() {
        let ring = DiagnosticEventRing::new(2);

        ring.record(event("first"));
        ring.record(event("second"));
        ring.record(event("third"));

        let snapshot = ring.snapshot();
        assert_eq!(snapshot.capacity, 2);
        assert_eq!(snapshot.retained_count, 2);
        assert_eq!(snapshot.dropped_count, 1);
        assert_eq!(snapshot.events[0].payload_summary, "second");
        assert_eq!(snapshot.events[1].payload_summary, "third");
    }

    #[test]
    fn diagnostics_event_ring_zero_capacity_drops_everything() {
        let ring = DiagnosticEventRing::new(0);

        ring.record(event("secret"));

        let snapshot = ring.snapshot();
        assert_eq!(snapshot.retained_count, 0);
        assert_eq!(snapshot.dropped_count, 1);
    }

    #[test]
    fn diagnostics_event_ring_persists_and_loads_recent_events() {
        let dir = match tempdir() {
            Ok(dir) => dir,
            Err(error) => panic!("expected tempdir: {error}"),
        };
        let path = dir.path().join("events.jsonl");

        for label in ["first", "second", "third"] {
            if let Err(error) = append_diagnostic_event_to_path(&event(label), &path) {
                panic!("expected append {label}: {error}");
            }
        }

        let snapshot = match persisted_diagnostics_snapshot_from_path(&path, 2) {
            Ok(snapshot) => snapshot,
            Err(error) => panic!("expected persisted events: {error}"),
        };

        assert_eq!(snapshot.retained_count, 2);
        assert_eq!(snapshot.dropped_count, 1);
        assert_eq!(snapshot.events[0].payload_summary, "second");
        assert_eq!(snapshot.events[1].payload_summary, "third");
    }
}
