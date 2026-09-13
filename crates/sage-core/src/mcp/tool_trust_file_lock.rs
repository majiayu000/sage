//! Inter-process advisory lock for `mcp_tool_trust.json` transactions.
//!
//! The in-process `tokio::Mutex` only serializes one Sage process. Concurrent
//! Sage processes sharing `$HOME/.sage` must also coordinate via this lock so
//! first-use baselines cannot overwrite each other.

use super::error::McpError;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

/// Held while loading, checking, and saving the MCP tool trust baseline file.
#[derive(Debug)]
pub(crate) struct ToolTrustFileLock {
    _file: File,
}

impl ToolTrustFileLock {
    /// Acquire an exclusive inter-process lock for `trust_path`.
    ///
    /// Uses a sibling `*.lock` file so readers/writers of the JSON baseline
    /// share one coordination point across processes.
    pub(crate) fn acquire(trust_path: &Path) -> Result<Self, McpError> {
        let lock_path = lock_path_for(trust_path);
        ensure_lock_parent(&lock_path)?;
        let file = acquire_exclusive_file(&lock_path)?;
        Ok(Self { _file: file })
    }

    /// Non-blocking acquire for tests.
    #[cfg(test)]
    pub(crate) fn try_acquire(trust_path: &Path) -> Result<Option<Self>, McpError> {
        let lock_path = lock_path_for(trust_path);
        ensure_lock_parent(&lock_path)?;
        match try_acquire_exclusive_file(&lock_path)? {
            Some(file) => Ok(Some(Self { _file: file })),
            None => Ok(None),
        }
    }
}

impl Drop for ToolTrustFileLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let _ = unsafe { libc::flock(self._file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

fn lock_path_for(trust_path: &Path) -> PathBuf {
    let mut lock_path = trust_path.to_path_buf();
    let suffix = trust_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!("{ext}.lock"))
        .unwrap_or_else(|| "lock".to_string());
    lock_path.set_extension(suffix);
    lock_path
}

fn ensure_lock_parent(lock_path: &Path) -> Result<(), McpError> {
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            McpError::schema(format!(
                "Failed to create MCP tool trust lock directory {}: {}",
                parent.display(),
                error
            ))
        })?;
    }
    Ok(())
}

fn open_lock_file(lock_path: &Path) -> Result<File, McpError> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)
        .map_err(|error| {
            McpError::schema(format!(
                "Failed to open MCP tool trust lock {}: {}",
                lock_path.display(),
                error
            ))
        })
}

#[cfg(unix)]
fn acquire_exclusive_file(lock_path: &Path) -> Result<File, McpError> {
    use std::os::unix::io::AsRawFd;

    let file = open_lock_file(lock_path)?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };
    if rc != 0 {
        return Err(McpError::schema(format!(
            "Failed to lock MCP tool trust file {}: {}",
            lock_path.display(),
            std::io::Error::last_os_error()
        )));
    }
    Ok(file)
}

#[cfg(all(unix, test))]
fn try_acquire_exclusive_file(lock_path: &Path) -> Result<Option<File>, McpError> {
    use std::os::unix::io::AsRawFd;

    let file = open_lock_file(lock_path)?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(Some(file));
    }
    let err = std::io::Error::last_os_error();
    if err.kind() == std::io::ErrorKind::WouldBlock
        || err.raw_os_error() == Some(libc::EAGAIN)
        || err.raw_os_error() == Some(libc::EWOULDBLOCK)
    {
        return Ok(None);
    }
    Err(McpError::schema(format!(
        "Failed to try-lock MCP tool trust file {}: {err}",
        lock_path.display()
    )))
}

#[cfg(windows)]
fn acquire_exclusive_file(lock_path: &Path) -> Result<File, McpError> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::time::Duration;

    let mut delay = Duration::from_millis(5);
    for _ in 0..200 {
        match OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .share_mode(0)
            .open(lock_path)
        {
            Ok(file) => return Ok(file),
            Err(error) if error.raw_os_error() == Some(32) => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(McpError::schema(format!(
                    "Failed to lock MCP tool trust file {}: {}",
                    lock_path.display(),
                    error
                )));
            }
        }
    }
    Err(McpError::schema(format!(
        "Timed out locking MCP tool trust file {}",
        lock_path.display()
    )))
}

#[cfg(all(windows, test))]
fn try_acquire_exclusive_file(lock_path: &Path) -> Result<Option<File>, McpError> {
    use std::os::windows::fs::OpenOptionsExt;

    match OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .share_mode(0)
        .open(lock_path)
    {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.raw_os_error() == Some(32) => Ok(None),
        Err(error) => Err(McpError::schema(format!(
            "Failed to try-lock MCP tool trust file {}: {}",
            lock_path.display(),
            error
        ))),
    }
}

#[cfg(not(any(unix, windows)))]
fn acquire_exclusive_file(lock_path: &Path) -> Result<File, McpError> {
    open_lock_file(lock_path)
}

#[cfg(all(test, not(any(unix, windows))))]
fn try_acquire_exclusive_file(lock_path: &Path) -> Result<Option<File>, McpError> {
    acquire_exclusive_file(lock_path).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn acquire_blocks_second_try_acquire_until_drop() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let path = dir.path().join("mcp_tool_trust.json");
        let held = ToolTrustFileLock::acquire(&path)?;
        assert!(ToolTrustFileLock::try_acquire(&path)?.is_none());
        drop(held);
        assert!(ToolTrustFileLock::try_acquire(&path)?.is_some());
        Ok(())
    }

    #[test]
    fn concurrent_acquires_serialize() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let path = Arc::new(dir.path().join("mcp_tool_trust.json"));
        let barrier = Arc::new(Barrier::new(2));
        let mut handles = Vec::new();
        for _ in 0..2 {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _lock = ToolTrustFileLock::acquire(path.as_path()).expect("lock");
                thread::sleep(Duration::from_millis(20));
            }));
        }
        for handle in handles {
            handle.join().expect("thread");
        }
        Ok(())
    }
}
