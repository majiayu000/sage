use crate::types::ToolCall;
use std::path::{Path, PathBuf};

pub(super) fn expand_rm_undo_paths(path: &Path) -> Vec<PathBuf> {
    let mut expanded = expand_glob_path(path);
    if expanded.is_empty() {
        expanded.push(path.to_path_buf());
    }

    let mut files = Vec::new();
    for path in expanded {
        collect_undo_files(&path, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn expand_glob_path(path: &Path) -> Vec<PathBuf> {
    let pattern = path.to_string_lossy();
    if !contains_glob_meta(&pattern) {
        return vec![path.to_path_buf()];
    }

    glob::glob(&pattern)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .collect()
}

fn contains_glob_meta(value: &str) -> bool {
    value.chars().any(|c| matches!(c, '*' | '?' | '['))
}

fn collect_undo_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            collect_undo_files(&entry.path(), files);
        }
    } else {
        files.push(path.to_path_buf());
    }
}

pub(super) fn bash_rm_targets(tool_call: &ToolCall) -> Vec<String> {
    let Some(command) = tool_call
        .arguments
        .get("command")
        .and_then(|value| value.as_str())
    else {
        return Vec::new();
    };

    let words = first_shell_command_words(command);
    if !matches!(words.first().map(String::as_str), Some("rm")) {
        return Vec::new();
    }

    words
        .into_iter()
        .skip(1)
        .filter(|token| !token.starts_with('-'))
        .filter(|token| !token.is_empty())
        .collect()
}

fn first_shell_command_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for c in command.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        if let Some(active_quote) = quote {
            if active_quote == '"' && c == '\\' {
                escaped = true;
            } else if c == active_quote {
                quote = None;
            } else {
                current.push(c);
            }
            continue;
        }

        if c == '\\' {
            escaped = true;
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c.is_whitespace() {
            push_shell_word(&mut words, &mut current);
        } else if matches!(c, ';' | '|' | '&' | '<' | '>' | '\n' | '\r') {
            push_shell_word(&mut words, &mut current);
            break;
        } else {
            current.push(c);
        }
    }
    push_shell_word(&mut words, &mut current);
    words
}

fn push_shell_word(words: &mut Vec<String>, current: &mut String) {
    if current.is_empty() {
        return;
    }
    words.push(std::mem::take(current));
}
