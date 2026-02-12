//! String utility functions

/// Truncate string at character boundary (UTF-8 safe)
///
/// This function safely truncates a string to a maximum number of characters,
/// ensuring the truncation happens at a valid UTF-8 character boundary.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters to keep
///
/// # Returns
/// A string slice containing at most `max_chars` characters
///
/// # Example
/// ```ignore
/// use sage_core::utils::truncate_str;
///
/// let s = "你好世界";
/// assert_eq!(truncate_str(s, 2), "你好");
/// assert_eq!(truncate_str(s, 10), "你好世界");
/// ```
pub fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s, // String is shorter than max_chars
    }
}

/// Truncate string and add ellipsis if needed (UTF-8 safe)
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of characters including ellipsis
///
/// # Returns
/// A String that is at most `max_chars` characters, with "..." appended if truncated
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 3 {
        truncate_str(s, max_chars).to_string()
    } else {
        format!("{}...", truncate_str(s, max_chars - 3))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_ascii() {
        assert_eq!(truncate_str("hello world", 5), "hello");
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("", 5), "");
    }

    #[test]
    fn test_truncate_str_utf8() {
        // Chinese characters
        assert_eq!(truncate_str("你好世界", 2), "你好");
        assert_eq!(truncate_str("你好世界", 4), "你好世界");
        assert_eq!(truncate_str("你好世界", 10), "你好世界");

        // Mixed
        assert_eq!(truncate_str("Hello你好", 5), "Hello");
        assert_eq!(truncate_str("Hello你好", 6), "Hello你");
        assert_eq!(truncate_str("Hello你好", 7), "Hello你好");

        // Emoji
        assert_eq!(truncate_str("👋🌍", 1), "👋");
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("你好世界朋友", 5), "你好...");
    }
}
