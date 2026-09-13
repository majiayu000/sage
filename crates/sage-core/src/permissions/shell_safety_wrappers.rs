//! Strip common utility wrappers (`env`, `nice`, `nohup`, `stdbuf`, `timeout`)
//! so deny rules like `Bash(rm *)` also match `env rm`, `nice rm`, and friends.

use super::env_assignment_value_start;
use super::shell_safety_words::{
    quote_removed_shell_word, split_shell_word, strip_shell_command_word_prefix,
};

pub(super) fn strip_shell_utility_wrappers(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let before_len = segment.len();
        if let Some(rest) = strip_shell_command_word_prefix(segment, "env") {
            segment = strip_env_operands(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "nice") {
            segment = strip_nice_operands(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "nohup") {
            segment = strip_nohup_operands(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "stdbuf") {
            segment = strip_stdbuf_operands(rest);
        } else if let Some(rest) = strip_shell_command_word_prefix(segment, "timeout") {
            segment = strip_timeout_operands(rest);
        } else {
            return segment;
        }
        if segment.len() == before_len {
            return segment;
        }
    }
}

fn strip_env_operands(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if option == "--" {
            return skip_env_assignments(rest);
        }
        if option == "-" {
            segment = rest;
            continue;
        }
        if env_assignment_value_start(&option).is_some() {
            segment = rest;
            continue;
        }
        if !option.starts_with('-') {
            return segment;
        }

        match option.as_str() {
            "-u" | "-C" | "-S" | "-a" | "--unset" | "--chdir" | "--split-string" | "--argv0" => {
                let Some((_, after)) = split_shell_word(rest) else {
                    return segment;
                };
                segment = after;
            }
            opt if opt.starts_with("--unset=")
                || opt.starts_with("--chdir=")
                || opt.starts_with("--split-string=")
                || opt.starts_with("--argv0=")
                || opt.starts_with("--block-signal=")
                || opt.starts_with("--default-signal=")
                || opt.starts_with("--ignore-signal=") =>
            {
                segment = rest;
            }
            "-i" | "-0" | "-v" | "--ignore-environment" | "--null" | "--help" | "--version" => {
                segment = rest;
            }
            opt if opt.starts_with('-') && !opt.starts_with("--") => {
                segment = strip_env_short_cluster(opt, rest).unwrap_or(segment);
            }
            _ => return segment,
        }
    }
}

fn strip_env_short_cluster<'a>(option: &str, rest: &'a str) -> Option<&'a str> {
    let chars: Vec<char> = option.chars().skip(1).collect();
    if chars.is_empty() {
        return Some(rest);
    }
    if chars
        .iter()
        .any(|c| !matches!(c, 'i' | '0' | 'v' | 'u' | 'C' | 'S' | 'a'))
    {
        return None;
    }
    let needs_arg = matches!(chars.last(), Some('u' | 'C' | 'S' | 'a'));
    if needs_arg {
        split_shell_word(rest).map(|(_, after)| after)
    } else {
        Some(rest)
    }
}

fn skip_env_assignments(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if env_assignment_value_start(&option).is_some() {
            segment = rest;
            continue;
        }
        return segment;
    }
}

fn strip_nice_operands(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if option == "--" {
            return rest;
        }
        if !option.starts_with('-') {
            return segment;
        }
        if option == "-n" || option == "--adjustment" {
            let Some((_, after)) = split_shell_word(rest) else {
                return segment;
            };
            segment = after;
            continue;
        }
        if option.starts_with("--adjustment=") {
            segment = rest;
            continue;
        }
        // GNU nice accepts a bare adjustment such as `-10` / `--10`.
        if option
            .strip_prefix('-')
            .or_else(|| option.strip_prefix('+'))
            .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
        {
            segment = rest;
            continue;
        }
        return segment;
    }
}

fn strip_nohup_operands(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if option == "--" {
            return rest;
        }
        if matches!(option.as_str(), "--help" | "--version") {
            segment = rest;
            continue;
        }
        return segment;
    }
}

fn strip_stdbuf_operands(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if option == "--" {
            return rest;
        }
        if !option.starts_with('-') {
            return segment;
        }
        match option.as_str() {
            "-i" | "-o" | "-e" | "--input" | "--output" | "--error" => {
                let Some((_, after)) = split_shell_word(rest) else {
                    return segment;
                };
                segment = after;
            }
            // GNU stdbuf accepts attached modes: `-oL`, `-i0`, `-eL`.
            opt if !opt.starts_with("--")
                && (opt.starts_with("-i") || opt.starts_with("-o") || opt.starts_with("-e"))
                && opt.len() > 2 =>
            {
                segment = rest;
            }
            opt if opt.starts_with("--input=")
                || opt.starts_with("--output=")
                || opt.starts_with("--error=") =>
            {
                segment = rest;
            }
            "--help" | "--version" => segment = rest,
            _ => return segment,
        }
    }
}

fn strip_timeout_operands(mut segment: &str) -> &str {
    loop {
        segment = segment.trim_start();
        let Some((word, rest)) = split_shell_word(segment) else {
            return segment;
        };
        let option = quote_removed_shell_word(word);
        if option == "--" {
            return skip_one_word(rest);
        }
        if !option.starts_with('-') {
            // Mandatory duration token before the wrapped command.
            return rest;
        }
        match option.as_str() {
            "-k" | "-s" | "--kill-after" | "--signal" => {
                let Some((_, after)) = split_shell_word(rest) else {
                    return segment;
                };
                segment = after;
            }
            opt if opt.starts_with("--kill-after=") || opt.starts_with("--signal=") => {
                segment = rest;
            }
            "-v" | "--verbose" | "--preserve-status" | "--foreground" | "--help" | "--version" => {
                segment = rest;
            }
            _ => return segment,
        }
    }
}

fn skip_one_word(segment: &str) -> &str {
    split_shell_word(segment.trim_start())
        .map(|(_, rest)| rest)
        .unwrap_or(segment)
}
