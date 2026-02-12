//! Linux seccomp sandbox implementation
//!
//! Uses seccomp-bpf for syscall filtering on Linux.
//! Note: Full seccomp implementation requires the `libseccomp` library.
//! This module provides a placeholder implementation.

use super::types::{OsSandboxConfig, OsSandboxMode};
use crate::sandbox::SandboxError;
use tokio::process::Command;

/// Apply Linux seccomp sandbox to a command
///
/// Note: Full seccomp implementation would require:
/// 1. libseccomp bindings
/// 2. Careful syscall filtering based on requirements
///
/// This is a placeholder that logs the intended restrictions.
pub fn apply_seccomp(_cmd: &mut Command, config: &OsSandboxConfig) -> Result<(), SandboxError> {
    if !config.mode.is_enabled() {
        return Ok(());
    }

    tracing::error!(
        "Linux seccomp sandbox requested (mode: {:?}) but not implemented",
        config.mode
    );

    Err(SandboxError::InitializationFailed(
        "Linux seccomp sandbox is not implemented; disable OS sandbox or use another platform"
            .to_string(),
    ))
}

/// List of syscalls typically needed for basic process execution
#[allow(dead_code)]
const BASIC_SYSCALLS: &[&str] = &[
    "read",
    "write",
    "open",
    "close",
    "stat",
    "fstat",
    "lstat",
    "poll",
    "lseek",
    "mmap",
    "mprotect",
    "munmap",
    "brk",
    "ioctl",
    "access",
    "pipe",
    "dup",
    "dup2",
    "getpid",
    "execve",
    "exit",
    "exit_group",
    "wait4",
    "kill",
    "uname",
    "fcntl",
    "flock",
    "fsync",
    "fdatasync",
    "getcwd",
    "chdir",
    "readlink",
    "gettimeofday",
    "getuid",
    "getgid",
    "geteuid",
    "getegid",
    "getppid",
    "getpgrp",
    "setsid",
    "setpgid",
    "rt_sigaction",
    "rt_sigprocmask",
    "rt_sigreturn",
    "futex",
    "clock_gettime",
    "clock_getres",
    "nanosleep",
    "arch_prctl",
    "set_tid_address",
    "set_robust_list",
    "getrandom",
];

/// Syscalls to block for network-restricted mode
#[allow(dead_code)]
const NETWORK_SYSCALLS: &[&str] = &[
    "socket",
    "connect",
    "accept",
    "accept4",
    "sendto",
    "recvfrom",
    "sendmsg",
    "recvmsg",
    "bind",
    "listen",
    "getsockname",
    "getpeername",
    "socketpair",
    "setsockopt",
    "getsockopt",
    "shutdown",
];

/// Syscalls to block for read-only mode
#[allow(dead_code)]
const WRITE_SYSCALLS: &[&str] = &[
    "write",
    "writev",
    "pwrite64",
    "pwritev",
    "mkdir",
    "rmdir",
    "unlink",
    "unlinkat",
    "rename",
    "renameat",
    "renameat2",
    "link",
    "linkat",
    "symlink",
    "symlinkat",
    "truncate",
    "ftruncate",
    "chmod",
    "fchmod",
    "fchmodat",
    "chown",
    "fchown",
    "fchownat",
    "lchown",
    "creat",
    "mknod",
    "mknodat",
];

/// Generate a description of what would be filtered
#[allow(dead_code)]
fn describe_filter(config: &OsSandboxConfig) -> String {
    let mut desc = String::new();

    match config.mode {
        OsSandboxMode::ReadOnly => {
            desc.push_str("Filter mode: Read-only (block file modification syscalls)\n");
            desc.push_str("Blocked syscalls:\n");
            for sc in WRITE_SYSCALLS {
                desc.push_str(&format!("  - {}\n", sc));
            }
        }
        OsSandboxMode::NoNetwork => {
            desc.push_str("Filter mode: No Network (block networking syscalls)\n");
            desc.push_str("Blocked syscalls:\n");
            for sc in NETWORK_SYSCALLS {
                desc.push_str(&format!("  - {}\n", sc));
            }
        }
        OsSandboxMode::Strict => {
            desc.push_str("Filter mode: Strict (allowlist only essential syscalls)\n");
            desc.push_str("Allowed syscalls:\n");
            for sc in BASIC_SYSCALLS {
                desc.push_str(&format!("  + {}\n", sc));
            }
        }
        _ => {
            desc.push_str("Filter mode: Custom or disabled\n");
        }
    }

    if !config.allow_network {
        desc.push_str("\nNetwork access: BLOCKED\n");
    }

    if let Some(working_dir) = &config.working_dir {
        desc.push_str(&format!("\nWorking directory: {:?}\n", working_dir));
    }

    desc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_apply_seccomp_disabled() {
        let config = OsSandboxConfig::disabled();
        let mut cmd = Command::new("echo");
        let result = apply_seccomp(&mut cmd, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_seccomp_enabled() {
        let config = OsSandboxConfig::strict(PathBuf::from("/tmp"));
        let mut cmd = Command::new("echo");
        let result = apply_seccomp(&mut cmd, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_describe_filter() {
        let config = OsSandboxConfig::no_network(PathBuf::from("/tmp"));
        let desc = describe_filter(&config);
        assert!(desc.contains("No Network"));
        assert!(desc.contains("socket"));
    }
}
