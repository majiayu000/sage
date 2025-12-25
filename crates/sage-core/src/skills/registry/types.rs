//! Skill registry types and core implementation

use crate::error::SageResult;
use std::collections::HashMap;
use std::path::PathBuf;

use super::super::types::{Skill, SkillSource};

/// Skill registry for managing skills
pub struct SkillRegistry {
    /// Registered skills by name
    pub(super) skills: HashMap<String, Skill>,
    /// Project root directory
    pub(super) project_root: PathBuf,
    /// User config directory
    pub(super) user_config_dir: PathBuf,
}

impl SkillRegistry {
    /// Create a new skill registry
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        let user_config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("sage");

        Self {
            skills: HashMap::new(),
            project_root: project_root.into(),
            user_config_dir,
        }
    }

    /// Set user config directory
    pub fn with_user_config_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.user_config_dir = dir.into();
        self
    }

    /// Register a skill
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all skills
    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// List enabled skills
    pub fn list_enabled(&self) -> Vec<&Skill> {
        self.skills.values().filter(|s| s.enabled).collect()
    }

    /// Check if a skill exists
    pub fn contains(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Remove a skill
    pub fn remove(&mut self, name: &str) -> Option<Skill> {
        self.skills.remove(name)
    }

    /// Enable a skill
    pub fn enable(&mut self, name: &str) -> bool {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.enabled = true;
            true
        } else {
            false
        }
    }

    /// Disable a skill
    pub fn disable(&mut self, name: &str) -> bool {
        if let Some(skill) = self.skills.get_mut(name) {
            skill.enabled = false;
            true
        } else {
            false
        }
    }

    /// Get skill count
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Get builtin skill count
    pub fn builtin_count(&self) -> usize {
        self.skills
            .values()
            .filter(|s| s.source == SkillSource::Builtin)
            .count()
    }

    /// Discover skills from file system
    pub async fn discover(&mut self) -> SageResult<usize> {
        let mut count = 0;

        // Discover project skills
        let project_dir = self.project_root.join(".sage").join("skills");
        count += self.discover_from_dir(&project_dir, true).await?;

        // Discover user skills
        let user_dir = self.user_config_dir.join("skills");
        count += self.discover_from_dir(&user_dir, false).await?;

        Ok(count)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new(".")
    }
}
