use super::normalize_command_segment;

pub(super) fn normalized_command_segments(segment: &str) -> Vec<String> {
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
    for assembled in substitution_removed_first_word_segments(&normalized) {
        if !assembled.is_empty() && !segments.contains(&assembled) {
            segments.push(assembled);
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

fn substitution_removed_first_word_segments(segment: &str) -> Vec<String> {
    let Some((word, rest)) = split_first_shell_word(segment) else {
        return Vec::new();
    };
    let Some(assembled) = remove_embedded_empty_expansions(word) else {
        return Vec::new();
    };
    if rest.is_empty() {
        vec![assembled]
    } else {
        vec![format!("{assembled} {rest}")]
    }
}

fn remove_embedded_empty_expansions(word: &str) -> Option<String> {
    let mut output = String::new();
    let mut cursor = 0;
    let mut changed = false;
    while cursor < word.len() {
        if word[cursor..].starts_with("$(") {
            if let Some(end) = word_parenthesized_end(word, cursor + 2) {
                cursor = end;
                changed = true;
                continue;
            }
        }
        if word[cursor..].starts_with("${") {
            if let Some(end) = empty_parameter_expansion_end(word, cursor + 2) {
                cursor = end;
                changed = true;
                continue;
            }
        }
        if word[cursor..].starts_with('`') {
            if let Some(end) = word[cursor + 1..].find('`') {
                cursor += end + 2;
                changed = true;
                continue;
            }
        }
        let Some(c) = word[cursor..].chars().next() else {
            break;
        };
        output.push(c);
        cursor += c.len_utf8();
    }
    (changed && !output.is_empty()).then_some(output)
}

fn empty_parameter_expansion_end(input: &str, body_start: usize) -> Option<usize> {
    let close = input[body_start..].find('}')? + body_start;
    let body = &input[body_start..close];
    (body.ends_with('+') || body.ends_with(":+")).then_some(close + '}'.len_utf8())
}

fn word_parenthesized_end(input: &str, body_start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = body_start;
    while cursor < input.len() {
        let c = input[cursor..].chars().next()?;
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Some(cursor + c.len_utf8());
            }
        }
        cursor += c.len_utf8();
    }
    None
}
