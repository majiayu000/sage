//! OS-level sandbox implementations
//!
//! This module provides platform-specific sandbox implementations:
//! - macOS: `sandbox-exec` based isolation
//! - Linux: seccomp-based syscall filtering (future)
//!
//! These provide stronger isolation than simple resource limits by restricting
//! what operations a process can perform at the OS level.

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

mod types;

pub use types::{OsSandboxConfig, OsSandboxMode, OsSandboxResult};

use crate::sandbox::SandboxError;
use tokio::process::Command;

/// Apply OS-level sandbox to a command before execution
///
/// This function modifies the command to run inside an OS-level sandbox.
/// On macOS, it wraps the command with `sandbox-exec`.
/// On Linux, it would apply seccomp filters (not yet implemented).
/// On other platforms, it's a no-op.
pub fn apply_os_sandbox(cmd: &mut Command, config: &OsSandboxConfig) -> Result<(), SandboxError> {
    match config.mode {
        OsSandboxMode::Disabled => Ok(()),
        #[cfg(target_os = "macos")]
        _ => macos::apply_sandbox_exec(cmd, config),
        #[cfg(target_os = "linux")]
        _ => linux::apply_seccomp(cmd, config),
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        _ => {
            tracing::warn!("OS-level sandbox not supported on this platform");
            Ok(())
        }
    }
}

/// Check if OS-level sandbox is available on this platform
pub fn is_os_sandbox_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        // Check if sandbox-exec exists
        std::path::Path::new("/usr/bin/sandbox-exec").exists()
    }
    #[cfg(target_os = "linux")]
    {
        // Check for seccomp support
        // This is a simplified check - real implementation would check kernel support
        std::path::Path::new("/proc/sys/kernel/seccomp").exists()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        false
    }
}

/// Get the name of the OS sandbox implementation
pub fn os_sandbox_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "sandbox-exec (macOS)"
    }
    #[cfg(target_os = "linux")]
    {
        "seccomp (Linux)"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "none"
    }
}
