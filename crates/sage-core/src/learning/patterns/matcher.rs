//! Style detection functions for code analysis

use std::collections::HashMap;

/// Detect indentation style from code
pub fn detect_indentation_style(code: &str) -> Option<String> {
    let lines: Vec<&str> = code
        .lines()
        .filter(|l| l.starts_with(' ') || l.starts_with('\t'))
        .collect();

    if lines.is_empty() {
        return None;
    }

    let tab_lines = lines.iter().filter(|l| l.starts_with('\t')).count();
    let space_lines = lines.len() - tab_lines;

    if tab_lines > space_lines {
        Some("tabs".to_string())
    } else if space_lines > 0 {
        // Detect space count
        let mut space_counts: HashMap<usize, usize> = HashMap::new();
        for line in lines.iter().filter(|l| l.starts_with(' ')) {
            let spaces = line.len() - line.trim_start().len();
            if spaces > 0 {
                // Common indentation levels
                for level in [2, 4, 8] {
                    if spaces % level == 0 {
                        *space_counts.entry(level).or_insert(0) += 1;
                    }
                }
            }
        }

        space_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(spaces, _)| format!("{} spaces", spaces))
    } else {
        None
    }
}

/// Detect quote style (single vs double)
pub fn detect_quote_style(code: &str) -> Option<String> {
    let single_count = code.matches('\'').count();
    let double_count = code.matches('"').count();

    if single_count == 0 && double_count == 0 {
        return None;
    }

    if single_count > double_count * 2 {
        Some("single quotes".to_string())
    } else if double_count > single_count * 2 {
        Some("double quotes".to_string())
    } else {
        None // No clear preference
    }
}

/// Detect semicolon style
pub fn detect_semicolon_style(code: &str) -> Option<String> {
    let lines: Vec<&str> = code.lines().filter(|l| !l.trim().is_empty()).collect();
    let semi_lines = lines.iter().filter(|l| l.trim().ends_with(';')).count();

    if lines.is_empty() {
        return None;
    }

    let ratio = semi_lines as f32 / lines.len() as f32;

    if ratio > 0.7 {
        Some("always semicolons".to_string())
    } else if ratio < 0.1 {
        Some("no semicolons".to_string())
    } else {
        None
    }
}

/// Detect naming convention
pub fn detect_naming_convention(code: &str) -> Option<String> {
    let snake_case = regex::Regex::new(r"\b[a-z]+_[a-z_]+\b").ok()?;
    let camel_case = regex::Regex::new(r"\b[a-z]+[A-Z][a-zA-Z]+\b").ok()?;
    let pascal_case = regex::Regex::new(r"\b[A-Z][a-z]+[A-Z][a-zA-Z]+\b").ok()?;

    let snake_count = snake_case.find_iter(code).count();
    let camel_count = camel_case.find_iter(code).count();
    let pascal_count = pascal_case.find_iter(code).count();

    let total = snake_count + camel_count + pascal_count;
    if total < 5 {
        return None;
    }

    if snake_count > camel_count && snake_count > pascal_count {
        Some("snake_case".to_string())
    } else if camel_count > snake_count && camel_count > pascal_count {
        Some("camelCase".to_string())
    } else if pascal_count > snake_count && pascal_count > camel_count {
        Some("PascalCase".to_string())
    } else {
        None
    }
}

/// Detect brace style
pub fn detect_brace_style(code: &str) -> Option<String> {
    // Check for same-line opening braces (K&R style)
    let same_line = regex::Regex::new(r"\)\s*\{").ok()?;
    // Check for next-line opening braces (Allman style)
    let next_line = regex::Regex::new(r"\)\s*\n\s*\{").ok()?;

    let same_count = same_line.find_iter(code).count();
    let next_count = next_line.find_iter(code).count();

    if same_count == 0 && next_count == 0 {
        return None;
    }

    if same_count > next_count * 2 {
        Some("K&R braces (same line)".to_string())
    } else if next_count > same_count * 2 {
        Some("Allman braces (next line)".to_string())
    } else {
        None
    }
}
