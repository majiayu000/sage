//! Prompt file watcher for hot reload
//!
//! Watches prompt directories for changes and triggers reloads.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Prompt file watcher for hot reload
pub struct PromptWatcher {
    /// Debounced watcher
    _debouncer: Debouncer<RecommendedWatcher>,
    /// Watched paths
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
    /// Whether watcher is active
    active: Arc<RwLock<bool>>,
}

impl PromptWatcher {
    /// Create a new watcher with a reload callback
    pub fn new<F>(on_change: F) -> anyhow::Result<Self>
    where
        F: Fn(&[PathBuf]) + Send + Sync + 'static,
    {
        let watched_paths = Arc::new(RwLock::new(Vec::new()));
        let active = Arc::new(RwLock::new(true));

        let callback = Arc::new(on_change);
        let active_clone = Arc::clone(&active);

        let debouncer = new_debouncer(
            Duration::from_millis(500),
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                if !*active_clone.read() {
                    return;
                }

                match result {
                    Ok(events) => {
                        let paths: Vec<PathBuf> = events
                            .into_iter()
                            .map(|e| e.path)
                            .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
                            .collect();

                        if !paths.is_empty() {
                            debug!("Prompt files changed: {:?}", paths);
                            callback(&paths);
                        }
                    }
                    Err(e) => {
                        error!("Watch error: {:?}", e);
                    }
                }
            },
        )?;

        Ok(Self {
            _debouncer: debouncer,
            watched_paths,
            active,
        })
    }

    /// Watch a directory for changes
    pub fn watch(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        if !path.exists() {
            warn!("Watch path does not exist: {:?}", path);
            return Ok(());
        }

        self._debouncer
            .watcher()
            .watch(&path, RecursiveMode::Recursive)?;

        self.watched_paths.write().push(path.clone());
        info!("Watching prompt directory: {:?}", path);

        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        self._debouncer.watcher().unwatch(&path)?;

        let mut paths = self.watched_paths.write();
        paths.retain(|p| p != &path);

        Ok(())
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().clone()
    }

    /// Pause watching
    pub fn pause(&self) {
        *self.active.write() = false;
    }

    /// Resume watching
    pub fn resume(&self) {
        *self.active.write() = true;
    }

    /// Check if watcher is active
    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}

/// Simple event-based watcher that returns a receiver
pub struct SimpleWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
    watched_paths: Vec<PathBuf>,
}

impl SimpleWatcher {
    /// Create a new simple watcher
    pub fn new() -> anyhow::Result<Self> {
        let (tx, rx) = channel();

        let watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
            watched_paths: Vec::new(),
        })
    }

    /// Watch a directory
    pub fn watch(&mut self, path: impl Into<PathBuf>) -> anyhow::Result<()> {
        let path = path.into();
        if path.exists() {
            self._watcher.watch(&path, RecursiveMode::Recursive)?;
            self.watched_paths.push(path);
        }
        Ok(())
    }

    /// Check for pending events (non-blocking)
    pub fn poll(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(result) = self.receiver.try_recv() {
            if let Ok(event) = result {
                for path in event.paths {
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        changed.push(path);
                    }
                }
            }
        }
        changed
    }

    /// Wait for next event (blocking)
    pub fn wait(&self) -> Option<Vec<PathBuf>> {
        match self.receiver.recv() {
            Ok(Ok(event)) => {
                let paths: Vec<PathBuf> = event
                    .paths
                    .into_iter()
                    .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
                    .collect();
                if paths.is_empty() {
                    None
                } else {
                    Some(paths)
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::tempdir;

    #[test]
    fn test_watcher_creation() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let watcher = PromptWatcher::new(move |_paths| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_pause_resume() {
        let watcher = PromptWatcher::new(|_| {}).unwrap();

        assert!(watcher.is_active());
        watcher.pause();
        assert!(!watcher.is_active());
        watcher.resume();
        assert!(watcher.is_active());
    }

    #[test]
    fn test_simple_watcher_creation() {
        let watcher = SimpleWatcher::new();
        assert!(watcher.is_ok());
    }
}
