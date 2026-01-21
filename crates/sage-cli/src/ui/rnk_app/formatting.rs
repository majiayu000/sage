//! Text formatting utilities for rnk app

/// Wrap text with a prefix, handling line breaks properly
pub fn wrap_text_with_prefix(prefix: &str, text: &str, max_width: usize) -> Vec<String> {
    let prefix_width = unicode_width::UnicodeWidthStr::width(prefix);
    let text_width = max_width.saturating_sub(prefix_width);

    if text_width == 0 {
        return vec![truncate_to_width(prefix, max_width)];
    }

    let mut result = Vec::new();
    let mut first_line = true;

    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            if first_line {
                result.push(prefix.to_string());
                first_line = false;
            } else {
                result.push(String::new());
            }
            continue;
        }

        let wrapped = wrap_single_line(paragraph, text_width);
        for line in wrapped {
            if first_line {
                result.push(format!("{}{}", prefix, line));
                first_line = false;
            } else {
                let indent = " ".repeat(prefix_width);
                result.push(format!("{}{}", indent, line));
            }
        }
    }

    if result.is_empty() {
        result.push(prefix.to_string());
    }

    result
}

/// Wrap a single line of text to fit within max_width
pub fn wrap_single_line(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);

        if ch == '\t' {
            for _ in 0..2 {
                if current_width + 1 > max_width && current_width > 0 {
                    result.push(current_line);
                    current_line = String::new();
                    current_width = 0;
                }
                current_line.push(' ');
                current_width += 1;
            }
            continue;
        }

        if ch_width == 0 {
            continue;
        }

        if current_width + ch_width > max_width && current_width > 0 {
            result.push(current_line);
            current_line = String::new();
            current_width = 0;
        }

        current_line.push(ch);
        current_width += ch_width;
    }

    if !current_line.is_empty() || result.is_empty() {
        result.push(current_line);
    }

    result
}

/// Truncate text to fit within max_width, adding "..." if truncated
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if unicode_width::UnicodeWidthStr::width(text) <= max_width {
        return text.to_string();
    }

    let mut trimmed = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width + 3 > max_width {
            break;
        }
        trimmed.push(ch);
        width += ch_width;
    }
    trimmed.push_str("...");
    trimmed
}
