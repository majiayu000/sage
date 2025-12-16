//! Settings file location discovery
//!
//! This module handles finding settings files at different levels:
//! - User level: ~/.config/sage/settings.json
//! - Project level: .sage/settings.json
//! - Local level: .sage/settings.local.json

use std::path::{Path, PathBuf};

/// Settings file locations
#[derive(Debug, Clone)]
pub struct SettingsLocations {
    /// User-level settings (~/.config/sage/settings.json)
    pub user: PathBuf,

    /// Project-level settings (.sage/settings.json)
    pub project: Option<PathBuf>,

    /// Local-level settings (.sage/settings.local.json)
    pub local: Option<PathBuf>,

    /// Project root directory
    pub project_root: Option<PathBuf>,
}

impl SettingsLocations {
    /// Discover settings locations from the current directory
    pub fn discover() -> Self {
        Self::discover_from(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Discover settings locations from a specific directory
    pub fn discover_from(start_dir: impl AsRef<Path>) -> Self {
        let user = Self::get_user_settings_path();
        let project_root = Self::find_project_root(&start_dir);

        let (project, local) = if let Some(ref root) = project_root {
            let sage_dir = root.join(".sage");
            let project = sage_dir.join("settings.json");
            let local = sage_dir.join("settings.local.json");

            (
                if project.exists() {
                    Some(project)
                } else {
                    None
                },
                if local.exists() { Some(local) } else { None },
            )
        } else {
            (None, None)
        };

        Self {
            user,
            project,
            local,
            project_root,
        }
    }

    /// Get the user settings path
    pub fn get_user_settings_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("sage").join("settings.json")
    }

    /// Get the user config directory
    pub fn get_user_config_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("sage")
    }

    /// Find the project root by looking for .sage directory or .git
    fn find_project_root(start_dir: impl AsRef<Path>) -> Option<PathBuf> {
        let start = start_dir.as_ref().to_path_buf();
        let mut current = if start.is_absolute() {
            start
        } else {
            std::env::current_dir()
                .ok()?
                .join(start)
                .canonicalize()
                .ok()?
        };

        loop {
            // Check for .sage directory
            if current.join(".sage").is_dir() {
                return Some(current);
            }

            // Check for .git as fallback project root indicator
            if current.join(".git").exists() {
                return Some(current);
            }

            // Move up one directory
            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Get the project settings directory (.sage)
    pub fn get_project_settings_dir(&self) -> Option<PathBuf> {
        self.project_root.as_ref().map(|root| root.join(".sage"))
    }

    /// Check if user settings exist
    pub fn has_user_settings(&self) -> bool {
        self.user.exists()
    }

    /// Check if project settings exist
    pub fn has_project_settings(&self) -> bool {
        self.project.as_ref().map(|p| p.exists()).unwrap_or(false)
    }

    /// Check if local settings exist
    pub fn has_local_settings(&self) -> bool {
        self.local.as_ref().map(|p| p.exists()).unwrap_or(false)
    }

    /// Get all existing settings files in priority order (low to high)
    pub fn get_existing_files(&self) -> Vec<&PathBuf> {
        let mut files = Vec::new();

        if self.user.exists() {
            files.push(&self.user);
        }

        if let Some(ref project) = self.project {
            if project.exists() {
                files.push(project);
            }
        }

        if let Some(ref local) = self.local {
            if local.exists() {
                files.push(local);
            }
        }

        files
    }

    /// Initialize a new .sage directory in the project root
    pub fn init_project_settings(&self) -> std::io::Result<PathBuf> {
        let root = self
            .project_root
            .as_ref()
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let sage_dir = root.join(".sage");
        std::fs::create_dir_all(&sage_dir)?;

        Ok(sage_dir)
    }
}

impl Default for SettingsLocations {
    fn default() -> Self {
        Self::discover()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_user_settings_path() {
        let path = SettingsLocations::get_user_settings_path();
        assert!(path.to_string_lossy().contains("sage"));
        assert!(path.to_string_lossy().contains("settings.json"));
    }

    #[test]
    fn test_discover_no_project() {
        let temp_dir = TempDir::new().unwrap();
        let locations = SettingsLocations::discover_from(temp_dir.path());

        assert!(locations.project.is_none());
        assert!(locations.local.is_none());
    }

    #[test]
    fn test_discover_with_sage_dir() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        let settings_file = sage_dir.join("settings.json");
        fs::write(&settings_file, "{}").unwrap();

        let locations = SettingsLocations::discover_from(temp_dir.path());

        assert!(locations.project.is_some());
        assert!(locations.has_project_settings());
        assert_eq!(locations.project_root, Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn test_discover_with_local_settings() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        let local_file = sage_dir.join("settings.local.json");
        fs::write(&local_file, "{}").unwrap();

        let locations = SettingsLocations::discover_from(temp_dir.path());

        assert!(locations.local.is_some());
        assert!(locations.has_local_settings());
    }

    #[test]
    fn test_discover_with_git_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        let locations = SettingsLocations::discover_from(temp_dir.path());

        assert_eq!(locations.project_root, Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn test_get_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let sage_dir = temp_dir.path().join(".sage");
        fs::create_dir(&sage_dir).unwrap();

        let project_file = sage_dir.join("settings.json");
        let local_file = sage_dir.join("settings.local.json");
        fs::write(&project_file, "{}").unwrap();
        fs::write(&local_file, "{}").unwrap();

        let locations = SettingsLocations::discover_from(temp_dir.path());
        let existing = locations.get_existing_files();

        // Should have project and local (user may or may not exist)
        assert!(existing.len() >= 2);
    }

    #[test]
    fn test_init_project_settings() {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        let locations = SettingsLocations::discover_from(temp_dir.path());
        let sage_dir = locations.init_project_settings().unwrap();

        assert!(sage_dir.exists());
        assert!(sage_dir.is_dir());
        assert!(sage_dir.ends_with(".sage"));
    }
}
