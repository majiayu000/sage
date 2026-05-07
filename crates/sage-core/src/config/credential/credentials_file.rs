//! Credentials file management
//!
//! This module handles loading and saving credentials from JSON files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

/// Credentials stored in a JSON file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CredentialsFile {
    /// API keys indexed by provider name
    #[serde(default)]
    pub api_keys: HashMap<String, String>,

    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl CredentialsFile {
    /// Load credentials from a file
    pub fn load(path: &Path) -> Option<Self> {
        if !path.exists() {
            return None;
        }

        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(creds) => Some(creds),
                Err(e) => {
                    warn!("Failed to parse credentials file {}: {}", path.display(), e);
                    None
                }
            },
            Err(e) => {
                warn!("Failed to read credentials file {}: {}", path.display(), e);
                None
            }
        }
    }

    /// Save credentials to a file.
    ///
    /// On Unix, the file is created with mode `0o600` *before* any bytes are
    /// written, then atomically renamed onto the destination. This closes the
    /// race window in which a previous "write then chmod" sequence would leave
    /// the credentials file world-readable (typically 0o644 under the default
    /// umask) while it contained plaintext API keys.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;

            let tmp = unix_tmp_path(path);
            // Best-effort cleanup of a stale tmp from a prior crash so
            // `create_new` below has a clean slot to claim.
            let _ = std::fs::remove_file(&tmp);

            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            drop(file);

            // `rename` is atomic on the same filesystem and preserves the
            // tmp file's mode (0o600) on the destination.
            if let Err(e) = std::fs::rename(&tmp, path) {
                let _ = std::fs::remove_file(&tmp);
                return Err(e);
            }
        }

        #[cfg(not(unix))]
        {
            // Windows / other: rely on filesystem ACLs. Preserve historical
            // behavior so we don't regress non-Unix users.
            std::fs::write(path, &content)?;
        }

        Ok(())
    }

    /// Get an API key for a provider
    pub fn get_api_key(&self, provider: &str) -> Option<&str> {
        self.api_keys.get(provider).map(|s| s.as_str())
    }

    /// Set an API key for a provider
    pub fn set_api_key(&mut self, provider: impl Into<String>, key: impl Into<String>) {
        self.api_keys.insert(provider.into(), key.into());
    }
}

#[cfg(unix)]
fn unix_tmp_path(path: &Path) -> std::path::PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    name.push(format!(".tmp.{pid}.{nanos}"));
    let mut tmp = path.to_path_buf();
    tmp.set_file_name(name);
    tmp
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn save_creates_new_file_with_mode_0600() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "sk-test");
        creds.save(&path).unwrap();
        assert_eq!(
            mode_of(&path),
            0o600,
            "credentials file must be created mode 0o600 from the first byte"
        );
    }

    #[test]
    fn save_overwrites_existing_file_and_resets_mode() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        let mut creds = CredentialsFile::default();
        creds.set_api_key("openai", "first");
        creds.save(&path).unwrap();

        // Simulate a pre-existing file from before this fix landed: world-readable.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(mode_of(&path), 0o644);

        creds.set_api_key("openai", "second");
        creds.save(&path).unwrap();

        assert_eq!(
            mode_of(&path),
            0o600,
            "rewrite must restore 0o600 even if the previous file was 0o644"
        );
        let loaded = CredentialsFile::load(&path).unwrap();
        assert_eq!(loaded.get_api_key("openai"), Some("second"));
    }

    #[test]
    fn save_does_not_leave_tmp_file_behind() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("credentials.json");
        let mut creds = CredentialsFile::default();
        creds.set_api_key("anthropic", "sk-ant");
        creds.save(&path).unwrap();

        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "save must clean up after itself: {entries:?}"
        );
    }
}
