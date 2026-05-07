//! Shared filesystem paths for Sage user state.
//!
//! Six call sites in this crate previously used
//! `dirs::home_dir().unwrap_or_default().join(".sage")` to derive the
//! data directory. `PathBuf::default()` is the **empty path**, so when
//! `HOME` is unset (CI containers, `sudo -u`, sandboxes, `launchd` jobs
//! without `UserName`), every component silently wrote `.sage/...`
//! relative to whichever directory the binary was launched from. State
//! then split across whichever directory the user happened to be in.
//!
//! That is the U-11 anti-pattern documented in this project's
//! `CLAUDE.md`: different entry points hardcoding different data
//! paths. This module is the shared, observable replacement.
//!
//! Two flavors:
//!
//! - [`default_data_dir`]: returns `SageResult<PathBuf>`. Use when the
//!   caller can propagate the error.
//! - [`default_data_dir_or_warn`]: returns `PathBuf`. Falls back to
//!   `./.sage` (the historical behavior under an empty `HOME`) but
//!   emits a `tracing::warn!` so the failure mode is no longer silent.
//!   Use only inside `Default` impls and similar synchronous
//!   constructors that cannot easily return `Result`.

use std::path::PathBuf;

use crate::error::{SageError, SageResult};

/// Subdirectory under the user's home where Sage stores its state.
pub const SAGE_STATE_SUBDIR: &str = ".sage";

/// Resolve the shared Sage data directory: `$HOME/.sage`.
///
/// Returns `Err` if the home directory cannot be determined (`HOME`
/// unset on Unix, `%USERPROFILE%` unset on Windows). Prefer this over
/// [`default_data_dir_or_warn`] whenever the caller can propagate the
/// error — a typed failure is always more debuggable than a silent
/// fallback to the current working directory.
pub fn default_data_dir() -> SageResult<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(SAGE_STATE_SUBDIR))
        .ok_or_else(|| {
            SageError::config(
                "Could not determine the user\'s home directory; set HOME (Unix) or \
             USERPROFILE (Windows) so Sage knows where to read and write its state. \
             Falling back to a relative path silently splits user state across \
             whichever directory the binary was launched from."
                    .to_string(),
            )
        })
}

/// Resolve the shared Sage data directory with a logged fallback.
///
/// Returns `$HOME/.sage` when the home directory can be determined.
/// Otherwise emits a `tracing::warn!` and returns `./.sage` — the
/// historical behavior of the call sites this helper replaces. The
/// warn log surfaces the failure mode that was previously silent.
///
/// Reserve this for synchronous `Default` impls and similar
/// constructors that cannot easily return `Result`. Use
/// [`default_data_dir`] anywhere a typed error can flow.
pub fn default_data_dir_or_warn() -> PathBuf {
    match default_data_dir() {
        Ok(path) => path,
        Err(_) => {
            let fallback = PathBuf::from(SAGE_STATE_SUBDIR);
            tracing::warn!(
                fallback_path = %fallback.display(),
                "Home directory is unavailable; falling back to a relative path. \
                 Sage state may split across directories — set HOME (Unix) or \
                 USERPROFILE (Windows) to keep state in one place."
            );
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_data_dir_returns_home_subdir_when_home_set() {
        let path = default_data_dir().expect("test environment must have a home dir");
        assert!(
            path.ends_with(SAGE_STATE_SUBDIR),
            "expected `{SAGE_STATE_SUBDIR}` suffix, got {path:?}"
        );
        assert!(
            path.is_absolute(),
            "data dir must be absolute, got {path:?}"
        );
    }

    #[test]
    fn default_data_dir_or_warn_returns_path_when_home_set() {
        let path = default_data_dir_or_warn();
        assert!(
            path.ends_with(SAGE_STATE_SUBDIR),
            "expected `{SAGE_STATE_SUBDIR}` suffix, got {path:?}"
        );
    }

    #[test]
    fn fallback_path_is_relative_sage_subdir() {
        let fallback = PathBuf::from(SAGE_STATE_SUBDIR);
        assert_eq!(fallback.to_str(), Some(".sage"));
        assert!(!fallback.is_absolute());
    }
}
