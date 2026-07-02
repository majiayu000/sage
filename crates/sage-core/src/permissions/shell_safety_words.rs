pub(super) fn strip_shell_word_prefix<'a>(segment: &'a str, prefix: &str) -> Option<&'a str> {
    segment
        .strip_prefix(prefix)
        .filter(|rest| rest.chars().next().is_some_and(char::is_whitespace))
        .map(str::trim_start)
}

pub(super) fn strip_shell_command_word_prefix<'a>(
    segment: &'a str,
    prefix: &str,
) -> Option<&'a str> {
    let (word, rest) = split_shell_word(segment)?;
    (quote_removed_shell_word(word) == prefix).then_some(rest)
}

pub(super) fn split_shell_word(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let end = skip_shell_word(input, 0);
    (end > 0).then(|| (&input[..end], input[end..].trim_start()))
}

pub(super) fn normalize_shell_whitespace(segment: &str) -> String {
    segment.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn quote_removed_shell_word(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;
    let mut ansi_c_quote = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped && quote.is_none() {
            output.push(c);
            escaped = false;
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && escaped {
                if matches!(c, '$' | '`' | '"' | '\\' | '\n') {
                    output.push(c);
                } else {
                    output.push('\\');
                    output.push(c);
                }
                escaped = false;
            } else if ansi_c_quote && c == '\\' {
                output.push(read_ansi_c_escape(&mut chars));
            } else if active_quote == '"' && c == '\\' {
                escaped = true;
            } else if c == active_quote {
                quote = None;
                ansi_c_quote = false;
            } else {
                output.push(c);
            }
            continue;
        }

        if c == '$' && matches!(chars.peek(), Some('\'') | Some('"')) {
            let shell_quote = chars.next().unwrap_or('\'');
            quote = Some(shell_quote);
            ansi_c_quote = shell_quote == '\'';
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

fn read_ansi_c_escape(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> char {
    let Some(first) = chars.next() else {
        return '\\';
    };
    if first.is_digit(8) {
        let mut value = first.to_digit(8).unwrap_or(0);
        for _ in 0..2 {
            let Some(next) = chars.peek().copied().filter(|c| c.is_digit(8)) else {
                break;
            };
            chars.next();
            value = value * 8 + next.to_digit(8).unwrap_or(0);
        }
        return char::from_u32(value).unwrap_or(first);
    }
    if first == 'x' {
        return read_hex_escape(chars, 2).unwrap_or(first);
    }
    if first == 'u' {
        return read_hex_escape(chars, 4).unwrap_or(first);
    }
    if first == 'U' {
        return read_hex_escape(chars, 8).unwrap_or(first);
    }
    match first {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '\'' => '\'',
        '"' => '"',
        other => other,
    }
}

fn read_hex_escape(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    max: usize,
) -> Option<char> {
    let mut value = 0;
    let mut seen = false;
    for _ in 0..max {
        let Some(next) = chars.peek().copied().and_then(|c| c.to_digit(16)) else {
            break;
        };
        chars.next();
        value = value * 16 + next;
        seen = true;
    }
    seen.then(|| char::from_u32(value)).flatten()
}

pub(super) fn skip_shell_word(input: &str, mut cursor: usize) -> usize {
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

        if c.is_whitespace() || matches!(c, ';' | '|' | '&' | '<' | '>') {
            break;
        }

        cursor += c.len_utf8();
    }

    cursor
}

pub(super) fn skip_shell_whitespace(input: &str, mut cursor: usize) -> usize {
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

pub(super) fn starts_shell_comment(input: &str, cursor: usize) -> bool {
    input[cursor..].starts_with('#')
        && input[..cursor]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || matches!(c, ';' | '&' | '|'))
}
