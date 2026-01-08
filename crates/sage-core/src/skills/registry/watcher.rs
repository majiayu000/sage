//! Skill hot reload via file system watching
//!
//! This module provides automatic skill reloading when skill files are modified,
//! enabling hot reload functionality similar to Claude Code.

use crate::error::{SageError, SageResult};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind, Debouncer, new_debouncer};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info};

use super::types::SkillRegistry;

/// Event types for skill changes
#[derive(Debug, Clone)]
pub enum SkillChangeEvent {
    /// A skill file was created
    Created(PathBuf),
    /// A skill file was modified
    Modified(PathBuf),
    /// A skill file was deleted
    Deleted(PathBuf),
    /// Multiple skills need refresh (e.g., directory rename)
    RefreshAll,
}

/// Configuration for the skill watcher
#[derive(Debug, Clone)]
pub struct SkillWatcherConfig {
    /// Debounce duration for file events
    pub debounce_duration: Duration,
    /// Whether to watch project skills
    pub watch_project: bool,
    /// Whether to watch user skills
    pub watch_user: bool,
}

impl Default for SkillWatcherConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(500),
            watch_project: true,
            watch_user: true,
        }
    }
}

/// Skill file watcher for hot reloading
pub struct SkillWatcher {
    /// Debounced file watcher
    #[allow(dead_code)]
    debouncer: Debouncer<RecommendedWatcher>,
    /// Channel for receiving change events
    event_rx: mpsc::UnboundedReceiver<SkillChangeEvent>,
    /// Watched directories
    watched_dirs: Vec<PathBuf>,
}

impl SkillWatcher {
    /// Create a new skill watcher
    ///
    /// # Arguments
    /// * `project_root` - Project root directory for watching `.sage/skills/`
    /// * `user_config_dir` - User config directory for watching `~/.config/sage/skills/`
    /// * `config` - Watcher configuration
    pub fn new(
        project_root: &Path,
        user_config_dir: &Path,
        config: SkillWatcherConfig,
    ) -> SageResult<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Create debounced watcher
        let event_tx_clone = event_tx.clone();
        let debouncer = new_debouncer(
            config.debounce_duration,
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            let change_event = match event.kind {
                                DebouncedEventKind::Any => {
                                    // Check if it's a .md file
                                    if is_skill_file(&event.path) {
                                        Some(SkillChangeEvent::Modified(event.path.clone()))
                                    } else {
                                        None
                                    }
                                }
                                DebouncedEventKind::AnyContinuous => None,
                                _ => None,
                            };

                            if let Some(change) = change_event {
                                if let Err(e) = event_tx_clone.send(change) {
                                    error!("Failed to send skill change event: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("File watcher error: {}", e);
                    }
                }
            },
        )
        .map_err(|e| SageError::agent(format!("Failed to create file watcher: {}", e)))?;

        let mut watcher = Self {
            debouncer,
            event_rx,
            watched_dirs: Vec::new(),
        };

        // Watch project skills directory
        if config.watch_project {
            let project_skills = project_root.join(".sage").join("skills");
            if let Err(e) = watcher.watch_directory(&project_skills) {
                debug!("Could not watch project skills directory: {}", e);
            } else {
                watcher.watched_dirs.push(project_skills);
            }
        }

        // Watch user skills directory
        if config.watch_user {
            let user_skills = user_config_dir.join("skills");
            if let Err(e) = watcher.watch_directory(&user_skills) {
                debug!("Could not watch user skills directory: {}", e);
            } else {
                watcher.watched_dirs.push(user_skills);
            }
        }

        Ok(watcher)
    }

    /// Watch a directory for skill changes
    fn watch_directory(&mut self, path: &Path) -> SageResult<()> {
        if !path.exists() {
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(path)
                .map_err(|e| SageError::storage(format!("Failed to create skills directory: {}", e)))?;
        }

        self.debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| SageError::agent(format!("Failed to watch directory: {}", e)))?;

        info!("Watching skills directory: {:?}", path);
        Ok(())
    }

    /// Get the next change event (async)
    pub async fn next_event(&mut self) -> Option<SkillChangeEvent> {
        self.event_rx.recv().await
    }

    /// Try to get the next change event without blocking
    pub fn try_next_event(&mut self) -> Option<SkillChangeEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Get watched directories
    pub fn watched_dirs(&self) -> &[PathBuf] {
        &self.watched_dirs
    }
}

/// Check if a path is a skill file
fn is_skill_file(path: &Path) -> bool {
    // Check for .md extension
    if path.extension().map_or(false, |ext| ext == "md") {
        return true;
    }

    // Check for SKILL.md in directory
    if path.file_name().map_or(false, |name| name == "SKILL.md") {
        return true;
    }

    false
}

/// Skill hot reload manager
///
/// Manages automatic skill reloading in response to file changes.
pub struct SkillHotReloader {
    /// Skill watcher
    watcher: SkillWatcher,
    /// Skill registry (shared reference)
    registry: Arc<RwLock<SkillRegistry>>,
    /// Whether the reloader is running
    running: bool,
}

impl SkillHotReloader {
    /// Create a new hot reloader
    pub fn new(
        registry: Arc<RwLock<SkillRegistry>>,
        project_root: &Path,
        user_config_dir: &Path,
    ) -> SageResult<Self> {
        let watcher = SkillWatcher::new(project_root, user_config_dir, SkillWatcherConfig::default())?;

        Ok(Self {
            watcher,
            registry,
            running: false,
        })
    }

    /// Create with custom configuration
    pub fn with_config(
        registry: Arc<RwLock<SkillRegistry>>,
        project_root: &Path,
        user_config_dir: &Path,
        config: SkillWatcherConfig,
    ) -> SageResult<Self> {
        let watcher = SkillWatcher::new(project_root, user_config_dir, config)?;

        Ok(Self {
            watcher,
            registry,
            running: false,
        })
    }

    /// Start the hot reload loop
    ///
    /// This method runs in a loop, processing file change events and
    /// reloading skills as needed. Should be spawned as a background task.
    pub async fn run(&mut self) {
        self.running = true;
        info!("Skill hot reload started");

        while self.running {
            if let Some(event) = self.watcher.next_event().await {
                self.handle_event(event).await;
            }
        }

        info!("Skill hot reload stopped");
    }

    /// Handle a skill change event
    async fn handle_event(&self, event: SkillChangeEvent) {
        match event {
            SkillChangeEvent::Created(path) | SkillChangeEvent::Modified(path) => {
                info!("Skill file changed: {:?}", path);
                self.reload_skill(&path).await;
            }
            SkillChangeEvent::Deleted(path) => {
                info!("Skill file deleted: {:?}", path);
                self.remove_skill(&path).await;
            }
            SkillChangeEvent::RefreshAll => {
                info!("Refreshing all skills");
                self.reload_all().await;
            }
        }
    }

    /// Reload a single skill from file
    async fn reload_skill(&self, path: &Path) {
        let mut registry = self.registry.write().await;

        // Extract skill name from path
        let skill_name = extract_skill_name(path);

        // Remove old skill if exists
        if let Some(name) = &skill_name {
            if registry.contains(name) {
                registry.remove(name);
                debug!("Removed old skill: {}", name);
            }
        }

        // Re-discover all skills (simpler than loading a single one)
        // This ensures proper priority and source handling
        match registry.discover().await {
            Ok(count) => {
                info!("Reloaded skills: {} discovered", count);
            }
            Err(e) => {
                error!("Failed to reload skills: {}", e);
            }
        }
    }

    /// Remove a skill when its file is deleted
    async fn remove_skill(&self, path: &Path) {
        let mut registry = self.registry.write().await;

        if let Some(name) = extract_skill_name(path) {
            if registry.remove(&name).is_some() {
                info!("Removed skill: {}", name);
            }
        }
    }

    /// Reload all skills
    async fn reload_all(&self) {
        let mut registry = self.registry.write().await;

        // Clear non-builtin skills
        let builtin_names: Vec<String> = registry
            .list()
            .iter()
            .filter(|s| s.source == crate::skills::types::SkillSource::Builtin)
            .map(|s| s.name.clone())
            .collect();

        // Remove all non-builtin skills
        let all_names: Vec<String> = registry.list().iter().map(|s| s.name.clone()).collect();
        for name in all_names {
            if !builtin_names.contains(&name) {
                registry.remove(&name);
            }
        }

        // Re-discover
        match registry.discover().await {
            Ok(count) => {
                info!("Reloaded all skills: {} discovered", count);
            }
            Err(e) => {
                error!("Failed to reload all skills: {}", e);
            }
        }
    }

    /// Stop the hot reload loop
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if reloader is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// Extract skill name from file path
fn extract_skill_name(path: &Path) -> Option<String> {
    // Check for SKILL.md in directory - name is directory name
    if path.file_name().map_or(false, |name| name == "SKILL.md") {
        return path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
    }

    // Otherwise, name is file stem
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_skill_file() {
        assert!(is_skill_file(Path::new("test.md")));
        assert!(is_skill_file(Path::new("/path/to/skill/SKILL.md")));
        assert!(!is_skill_file(Path::new("test.txt")));
        assert!(!is_skill_file(Path::new("test.rs")));
    }

    #[test]
    fn test_extract_skill_name() {
        assert_eq!(
            extract_skill_name(Path::new("/skills/my-skill.md")),
            Some("my-skill".to_string())
        );
        assert_eq!(
            extract_skill_name(Path::new("/skills/my-skill/SKILL.md")),
            Some("my-skill".to_string())
        );
        assert_eq!(
            extract_skill_name(Path::new("/skills/commit.md")),
            Some("commit".to_string())
        );
    }

    #[test]
    fn test_skill_watcher_config_default() {
        let config = SkillWatcherConfig::default();
        assert_eq!(config.debounce_duration, Duration::from_millis(500));
        assert!(config.watch_project);
        assert!(config.watch_user);
    }

    #[tokio::test]
    async fn test_skill_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path().join("project");
        let user_config = temp_dir.path().join("config");

        // Create directories
        std::fs::create_dir_all(project_root.join(".sage/skills")).unwrap();
        std::fs::create_dir_all(user_config.join("skills")).unwrap();

        let watcher = SkillWatcher::new(
            &project_root,
            &user_config,
            SkillWatcherConfig::default(),
        );

        assert!(watcher.is_ok());
        let watcher = watcher.unwrap();
        assert_eq!(watcher.watched_dirs().len(), 2);
    }
}
