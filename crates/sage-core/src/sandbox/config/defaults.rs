//! Default command lists and environment passthrough configuration.

/// Default allowed commands for restricted mode
pub fn default_allowed_commands() -> Vec<String> {
    vec![
        // File operations
        "ls".to_string(),
        "cat".to_string(),
        "head".to_string(),
        "tail".to_string(),
        "find".to_string(),
        "grep".to_string(),
        "wc".to_string(),
        "file".to_string(),
        "stat".to_string(),
        // Directory operations
        "pwd".to_string(),
        "cd".to_string(),
        "mkdir".to_string(),
        // Text processing
        "sed".to_string(),
        "awk".to_string(),
        "sort".to_string(),
        "uniq".to_string(),
        "cut".to_string(),
        "tr".to_string(),
        "diff".to_string(),
        // Development tools
        "git".to_string(),
        "cargo".to_string(),
        "rustc".to_string(),
        "npm".to_string(),
        "node".to_string(),
        "python".to_string(),
        "python3".to_string(),
        "pip".to_string(),
        "pip3".to_string(),
        // Build tools
        "make".to_string(),
        "cmake".to_string(),
        // Archive tools
        "tar".to_string(),
        "zip".to_string(),
        "unzip".to_string(),
        "gzip".to_string(),
        "gunzip".to_string(),
        // Utilities
        "echo".to_string(),
        "date".to_string(),
        "which".to_string(),
        "env".to_string(),
        "true".to_string(),
        "false".to_string(),
        "test".to_string(),
        "[".to_string(),
    ]
}

/// Strictly allowed commands for strict mode
pub fn strict_allowed_commands() -> Vec<String> {
    vec![
        "ls".to_string(),
        "cat".to_string(),
        "head".to_string(),
        "tail".to_string(),
        "grep".to_string(),
        "wc".to_string(),
        "pwd".to_string(),
        "echo".to_string(),
        "true".to_string(),
        "false".to_string(),
    ]
}

/// Default blocked commands
pub fn default_blocked_commands() -> Vec<String> {
    vec![
        // Dangerous system commands
        "rm".to_string(),
        "rmdir".to_string(),
        "mv".to_string(),
        "cp".to_string(),
        // System modification
        "chmod".to_string(),
        "chown".to_string(),
        "chgrp".to_string(),
        // Process management
        "kill".to_string(),
        "killall".to_string(),
        "pkill".to_string(),
        // Package managers (system-level)
        "apt".to_string(),
        "apt-get".to_string(),
        "yum".to_string(),
        "dnf".to_string(),
        "brew".to_string(),
        // Sudo and privilege escalation
        "sudo".to_string(),
        "su".to_string(),
        "doas".to_string(),
        // Network tools (dangerous)
        "nc".to_string(),
        "netcat".to_string(),
        "ncat".to_string(),
        "telnet".to_string(),
        // Shells (prevent shell escape)
        "sh".to_string(),
        "bash".to_string(),
        "zsh".to_string(),
        "fish".to_string(),
        "csh".to_string(),
        "tcsh".to_string(),
        // Other dangerous
        "eval".to_string(),
        "exec".to_string(),
        "source".to_string(),
        ".".to_string(),
        "dd".to_string(),
        "mkfs".to_string(),
        "fdisk".to_string(),
        "parted".to_string(),
    ]
}

/// Commands that should always be blocked regardless of mode
pub fn always_blocked_commands() -> Vec<String> {
    vec![
        "sudo".to_string(),
        "su".to_string(),
        "doas".to_string(),
        "dd".to_string(),
        "mkfs".to_string(),
        "fdisk".to_string(),
        "parted".to_string(),
    ]
}

/// Default environment variables to pass through
pub fn default_env_passthrough() -> Vec<String> {
    vec![
        "PATH".to_string(),
        "HOME".to_string(),
        "USER".to_string(),
        "LANG".to_string(),
        "LC_ALL".to_string(),
        "TERM".to_string(),
        "SHELL".to_string(),
        "EDITOR".to_string(),
        "VISUAL".to_string(),
        // Development related
        "CARGO_HOME".to_string(),
        "RUSTUP_HOME".to_string(),
        "GOPATH".to_string(),
        "GOROOT".to_string(),
        "NODE_PATH".to_string(),
        "NPM_CONFIG_PREFIX".to_string(),
        "PYTHONPATH".to_string(),
        "VIRTUAL_ENV".to_string(),
    ]
}
