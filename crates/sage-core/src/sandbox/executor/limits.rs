//! Platform-specific resource limit handling

use crate::sandbox::limits::ResourceLimits;
use tokio::process::Command;

/// Try to set a resource limit; return error only for unexpected failures.
/// EINVAL is treated as non-fatal because some platforms (e.g., macOS) don't
/// support certain limits like RLIMIT_AS.
#[cfg(unix)]
fn try_setrlimit(resource: libc::c_int, limit: &libc::rlimit) -> std::io::Result<()> {
    // SAFETY: setrlimit is async-signal-safe and the limit pointer is valid.
    if unsafe { libc::setrlimit(resource, limit) } != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EINVAL) {
            // Kernel doesn't support this limit — skip gracefully
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

/// Apply Unix-specific resource limits
#[cfg(unix)]
#[allow(unused_imports)]
pub(super) fn apply_unix_limits(cmd: &mut Command, limits: &ResourceLimits) {
    use std::os::unix::process::CommandExt;

    // Clone limits for the closure
    let max_memory = limits.max_memory_bytes;
    let max_cpu = limits.max_cpu_seconds;
    let max_files = limits.max_open_files;
    let max_stack = limits.max_stack_bytes;

    // SAFETY: pre_exec runs between fork() and exec() in the child process.
    // The closure only calls async-signal-safe libc functions (setrlimit).
    // All captured values (max_memory, max_cpu, max_files, max_stack) are
    // Copy types moved into the closure, so no shared mutable state exists.
    // The parent process is not affected by these limit changes.
    unsafe {
        cmd.pre_exec(move || {
            // Set memory limit (RLIMIT_AS)
            if let Some(mem) = max_memory {
                let limit = libc::rlimit {
                    rlim_cur: mem,
                    rlim_max: mem,
                };
                try_setrlimit(libc::RLIMIT_AS, &limit)?;
            }

            // Set CPU time limit (RLIMIT_CPU)
            if let Some(cpu) = max_cpu {
                let limit = libc::rlimit {
                    rlim_cur: cpu,
                    rlim_max: cpu,
                };
                try_setrlimit(libc::RLIMIT_CPU, &limit)?;
            }

            // Set open files limit (RLIMIT_NOFILE)
            if let Some(files) = max_files {
                let limit = libc::rlimit {
                    rlim_cur: files as u64,
                    rlim_max: files as u64,
                };
                try_setrlimit(libc::RLIMIT_NOFILE, &limit)?;
            }

            // Set stack size limit (RLIMIT_STACK)
            if let Some(stack) = max_stack {
                let limit = libc::rlimit {
                    rlim_cur: stack,
                    rlim_max: stack,
                };
                try_setrlimit(libc::RLIMIT_STACK, &limit)?;
            }

            Ok(())
        });
    }
}

/// No-op for non-Unix platforms
#[cfg(not(unix))]
pub(super) fn apply_unix_limits(_cmd: &mut Command, _limits: &ResourceLimits) {
    // No-op on non-Unix platforms
}
