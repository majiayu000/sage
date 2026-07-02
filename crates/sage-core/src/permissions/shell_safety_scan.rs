use super::{
    UNKNOWN_EXEC_SEGMENT, is_shell_command_separator, normalize_command_segment,
    quote_removed_shell_word, shell_separator_len, skip_shell_whitespace, skip_shell_word,
    sources_stdin_segment, split_shell_word, starts_shell_comment, strip_shell_command_word_prefix,
};

pub(super) fn contains_unquoted_shell_control_metachar(input: &str) -> bool {
    let mut cursor = 0;
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
        if c == '\'' || c == '"' {
            let end = consume_quoted_span(input, cursor, c);
            if c == '"' && double_quoted_has_command_substitution(input, cursor, end) {
                return true;
            }
            cursor = end;
            continue;
        }
        if c == '\\' {
            escaped = true;
            cursor += c.len_utf8();
            continue;
        }
        if input[cursor..].starts_with("&&")
            || input[cursor..].starts_with("||")
            || input[cursor..].starts_with("$(")
            || matches!(c, ';' | '|' | '&' | '<' | '>' | '`' | '\n' | '\r')
        {
            return true;
        }
        cursor += c.len_utf8();
    }
    false
}

pub(super) fn shell_command_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut cursor = 0;
    let mut escaped = false;

    while cursor < command.len() {
        let Some(c) = command[cursor..].chars().next() else {
            break;
        };

        if escaped {
            current.push(c);
            escaped = false;
            cursor += c.len_utf8();
            continue;
        }
        if starts_shell_comment(command, cursor) {
            push_raw_segment(&mut segments, &mut current);
            cursor = next_line_start(command, cursor).unwrap_or(command.len());
            continue;
        }
        if c == '\'' || c == '"' {
            let end = consume_quoted_span(command, cursor, c);
            if c == '"' {
                let content_start = cursor + c.len_utf8();
                let content_end = quoted_content_end(command, end, c);
                segments.extend(substitution_body_segments(
                    &command[content_start..content_end],
                ));
            }
            current.push_str(&command[cursor..end]);
            cursor = end;
            continue;
        }
        if c == '\\' {
            current.push(c);
            escaped = true;
            cursor += c.len_utf8();
            continue;
        }
        if command[cursor..].starts_with("$((") {
            let (body, end) = arithmetic_expansion_body(command, cursor);
            segments.extend(substitution_body_segments(body));
            current.push_str("$(( ))");
            cursor = end;
            continue;
        }
        if command[cursor..].starts_with("$(")
            || command[cursor..].starts_with("<(")
            || command[cursor..].starts_with(">(")
        {
            let is_process_substitution =
                command[cursor..].starts_with("<(") || command[cursor..].starts_with(">(");
            if let Some((body, end)) = parenthesized_body(command, cursor + 2) {
                if is_process_substitution {
                    segments.extend(shell_command_segments(body));
                }
                if is_process_substitution && current_consumes_shell_input(&current) {
                    segments.push(UNKNOWN_EXEC_SEGMENT.to_string());
                    segments.extend(shell_input_producer_segments(body));
                }
                if command[cursor..].starts_with("$(") {
                    segments.extend(shell_command_segments(body));
                    if current.trim().is_empty() {
                        segments.push(UNKNOWN_EXEC_SEGMENT.to_string());
                    }
                }
                current.push_str("$()");
                cursor = end;
            } else {
                push_raw_segment(&mut segments, &mut current);
                cursor += 2;
            }
            continue;
        }
        if c == '`' {
            let (body, end) = backtick_body(command, cursor);
            segments.extend(shell_command_segments(body));
            if current.trim().is_empty() {
                segments.push(UNKNOWN_EXEC_SEGMENT.to_string());
            }
            current.push_str("``");
            cursor = end;
            continue;
        }
        if matches!(c, '<' | '>') {
            if current.trim().chars().all(|c| c.is_ascii_digit()) {
                current.clear();
            }
            if let Some((operand_start, operand_end)) = redirection_operand_range(command, cursor) {
                let operand = &command[operand_start..operand_end];
                segments.extend(substitution_body_segments(operand));
                if is_here_string(command, cursor) && current_consumes_shell_input(&current) {
                    segments.extend(shell_command_segments(&quote_removed_shell_word(operand)));
                }
                if operand.starts_with("<(") || operand.starts_with(">(") {
                    push_raw_segment(&mut segments, &mut current);
                    cursor = operand_start + 2;
                } else {
                    cursor = operand_end;
                }
            } else {
                cursor += c.len_utf8();
            }
            continue;
        }
        if is_shell_command_separator(command, cursor) {
            push_raw_segment(&mut segments, &mut current);
            cursor += shell_separator_len(command, cursor);
            continue;
        }

        current.push(c);
        cursor += c.len_utf8();
    }

    push_raw_segment(&mut segments, &mut current);
    segments
}

pub(super) fn substitution_body_segments(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut cursor = 0;
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
        if c == '\'' || c == '"' {
            let end = consume_quoted_span(input, cursor, c);
            if c == '"' {
                let content_start = cursor + c.len_utf8();
                let content_end = quoted_content_end(input, end, c);
                segments.extend(substitution_body_segments(
                    &input[content_start..content_end],
                ));
            }
            cursor = end;
            continue;
        }
        if c == '\\' {
            escaped = true;
            cursor += c.len_utf8();
            continue;
        }
        if input[cursor..].starts_with("$((") {
            let (body, end) = arithmetic_expansion_body(input, cursor);
            segments.extend(substitution_body_segments(body));
            cursor = end;
            continue;
        }
        if input[cursor..].starts_with("$(") {
            if let Some((body, end)) = parenthesized_body(input, cursor + 2) {
                segments.extend(shell_command_segments(body));
                cursor = end;
            } else {
                cursor += 2;
            }
            continue;
        }
        if c == '`' {
            let (body, end) = backtick_body(input, cursor);
            segments.extend(shell_command_segments(body));
            cursor = end;
            continue;
        }
        cursor += c.len_utf8();
    }

    segments
}

fn push_raw_segment(segments: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }
    current.clear();
}

fn consume_quoted_span(input: &str, cursor: usize, quote: char) -> usize {
    let mut escaped = false;
    let mut next = cursor + quote.len_utf8();
    while next < input.len() {
        let Some(c) = input[next..].chars().next() else {
            break;
        };
        next += c.len_utf8();
        if quote == '"' && escaped {
            escaped = false;
        } else if quote == '"' && c == '\\' {
            escaped = true;
        } else if c == quote {
            return next;
        }
    }
    input.len()
}

fn quoted_content_end(input: &str, end: usize, quote: char) -> usize {
    end.checked_sub(quote.len_utf8())
        .filter(|candidate| input[*candidate..end].starts_with(quote))
        .unwrap_or(end)
}

fn double_quoted_has_command_substitution(input: &str, start: usize, end: usize) -> bool {
    let content_start = start + '"'.len_utf8();
    let content_end = quoted_content_end(input, end, '"');
    input[content_start..content_end].contains("$(")
        || input[content_start..content_end].contains('`')
}

fn arithmetic_expansion_body(input: &str, cursor: usize) -> (&str, usize) {
    let body_start = cursor + 3;
    let mut next = body_start;
    while next < input.len() {
        if input[next..].starts_with("))") {
            return (&input[body_start..next], next + 2);
        }
        next += input[next..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);
    }
    (&input[body_start..], input.len())
}

fn parenthesized_body(input: &str, body_start: usize) -> Option<(&str, usize)> {
    let mut depth = 1usize;
    let mut cursor = body_start;
    let mut escaped = false;
    while cursor < input.len() {
        let c = input[cursor..].chars().next()?;
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '\'' || c == '"' {
            cursor = consume_quoted_span(input, cursor, c);
            continue;
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Some((&input[body_start..cursor], cursor + c.len_utf8()));
            }
        }
        cursor += c.len_utf8();
    }
    None
}

fn backtick_body(input: &str, cursor: usize) -> (&str, usize) {
    let body_start = cursor + '`'.len_utf8();
    let mut next = body_start;
    let mut escaped = false;
    while next < input.len() {
        let Some(c) = input[next..].chars().next() else {
            break;
        };
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '`' {
            return (&input[body_start..next], next + c.len_utf8());
        }
        next += c.len_utf8();
    }
    (&input[body_start..], input.len())
}

fn redirection_operand_range(input: &str, cursor: usize) -> Option<(usize, usize)> {
    let operator = input[cursor..].chars().next()?;
    let mut next = cursor + operator.len_utf8();

    if input[next..].starts_with(operator) {
        next += operator.len_utf8();
        if operator == '<' && input[next..].starts_with(operator) {
            next += operator.len_utf8();
        }
    } else if operator == '<' && input[next..].starts_with('>') {
        next += '>'.len_utf8();
    } else if operator == '>' && input[next..].starts_with('|') {
        next += '|'.len_utf8();
    }
    if input[next..].starts_with('&') {
        next += '&'.len_utf8();
    }

    next = skip_shell_whitespace(input, next);
    Some((next, redirection_word_end(input, next)))
}

fn is_here_string(input: &str, cursor: usize) -> bool {
    input[cursor..].starts_with("<<<")
}

fn current_sources_stdin(current: &str) -> bool {
    sources_stdin_segment(current)
}

fn current_consumes_shell_input(current: &str) -> bool {
    current_sources_stdin(current)
        || current_sources_argument(current)
        || current_invokes_stdin_shell(current)
}

fn current_sources_argument(current: &str) -> bool {
    matches!(normalize_command_segment(current).as_str(), "source" | ".")
}

fn current_invokes_stdin_shell(current: &str) -> bool {
    let normalized = normalize_command_segment(current);
    let Some((word, rest)) = split_shell_word(&normalized) else {
        return false;
    };
    let command = quote_removed_shell_word(word);
    (matches!(command.as_str(), "bash" | "sh")
        || command.ends_with("/bash")
        || command.ends_with("/sh"))
        && shell_invocation_reads_stdin(rest)
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
    }
}

fn shell_input_producer_segments(body: &str) -> Vec<String> {
    let mut segments = Vec::new();
    for segment in shell_command_segments(body) {
        let Some(rest) = strip_shell_command_word_prefix(&segment, "printf") else {
            continue;
        };
        let Some((format, mut rest)) = split_shell_word(rest) else {
            continue;
        };
        let format = quote_removed_shell_word(format);
        if format.contains("%s") {
            while let Some((arg, after)) = split_shell_word(rest) {
                let script = quote_removed_shell_word(arg).replace("\\n", "\n");
                segments.extend(shell_command_segments(&script));
                rest = after;
            }
        } else {
            let script = format.replace("\\n", "\n");
            segments.extend(shell_command_segments(&script));
        }
    }
    segments
}

fn redirection_word_end(input: &str, cursor: usize) -> usize {
    if input[cursor..].starts_with("$(") {
        return parenthesized_body(input, cursor + 2)
            .map(|(_, end)| end)
            .unwrap_or_else(|| skip_shell_word(input, cursor));
    }
    if input[cursor..].starts_with("<(") || input[cursor..].starts_with(">(") {
        return parenthesized_body(input, cursor + 2)
            .map(|(_, end)| end)
            .unwrap_or_else(|| skip_shell_word(input, cursor));
    }
    if input[cursor..].starts_with('`') {
        return backtick_body(input, cursor).1;
    }
    skip_shell_word(input, cursor)
}

fn next_line_start(input: &str, cursor: usize) -> Option<usize> {
    input[cursor..]
        .find('\n')
        .map(|offset| cursor + offset + '\n'.len_utf8())
}
