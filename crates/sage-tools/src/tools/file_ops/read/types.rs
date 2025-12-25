//! Type definitions and constants for the Read tool

/// Maximum line length before truncation
pub const MAX_LINE_LENGTH: usize = 2000;

/// Default maximum lines to read
pub const DEFAULT_MAX_LINES: usize = 2000;

/// Maximum file size in bytes (100MB)
pub const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Maximum allowed limit for lines
pub const MAX_LIMIT: usize = 10000;
