//! Permission key extraction for settings-backed permission checks.

use crate::tools::types::ToolCall;
use std::path::{Path, PathBuf};

use super::settings_permission_paths;

pub(super) fn canonical_permission_tool_name(tool_name: &str) -> String {
    match tool_name.to_lowercase().as_str() {
        "bash" => "Bash".to_string(),
        "read" => "Read".to_string(),
        "write" => "Write".to_string(),
        "edit" => "Edit".to_string(),
        "multiedit" | "multi_edit" => "MultiEdit".to_string(),
        "glob" => "Glob".to_string(),
        "grep" => "Grep".to_string(),
        "task" => "Task".to_string(),
        "webfetch" | "web_fetch" => "WebFetch".to_string(),
        "websearch" | "web_search" => "WebSearch".to_string(),
        "openbrowser" | "open_browser" => "OpenBrowser".to_string(),
        "todowrite" | "todo_write" => "TodoWrite".to_string(),
        "askuserquestion" | "ask_user_question" => "AskUserQuestion".to_string(),
        "notebookedit" | "notebook_edit" => "NotebookEdit".to_string(),
        _ => tool_name.to_string(),
    }
}

pub(super) fn actual_permission_keys(
    tool_name: &str,
    call: &ToolCall,
    working_dir: &Path,
) -> Vec<String> {
    match tool_name.to_lowercase().as_str() {
        "multiedit" => {
            format_permission_keys(tool_name, multiedit_permission_arguments(call, working_dir))
        }
        "http_client" => {
            let keys = http_client_permission_keys(call, working_dir);
            if keys.is_empty() {
                vec![tool_name.to_string()]
            } else {
                keys
            }
        }
        _ => format_permission_keys(
            tool_name,
            actual_permission_argument(tool_name, call, working_dir)
                .into_iter()
                .collect(),
        ),
    }
}

fn format_permission_keys(tool_name: &str, arguments: Vec<String>) -> Vec<String> {
    if arguments.is_empty() {
        return vec![tool_name.to_string()];
    }
    arguments
        .into_iter()
        .map(|argument| format!("{}({})", tool_name, argument))
        .collect()
}

fn actual_permission_argument(
    tool_name: &str,
    call: &ToolCall,
    working_dir: &Path,
) -> Option<String> {
    match tool_name.to_lowercase().as_str() {
        "bash" => call
            .arguments
            .get("command")
            .and_then(|value| value.as_str())
            .map(|command| command.trim().to_string()),
        "read" | "write" | "edit" | "multiedit" => {
            path_permission_argument(call, &["file_path", "path"], working_dir)
        }
        "grep" => path_permission_argument(call, &["path"], working_dir),
        "glob" => glob_permission_argument(call, working_dir),
        "webfetch" => webfetch_permission_argument(call),
        "http_client" | "openbrowser" => url_permission_argument(call),
        "websearch" => call
            .get_argument::<String>("query")
            .map(|query| query.trim().to_string()),
        "notebookedit" => path_permission_argument(call, &["notebook_path"], working_dir),
        _ => None,
    }
}

fn path_permission_argument(call: &ToolCall, keys: &[&str], working_dir: &Path) -> Option<String> {
    for key in keys {
        if let Some(path) = call.get_argument::<String>(key) {
            return Some(settings_permission_paths::workspace_relative_path(
                &path,
                working_dir,
            ));
        }
    }

    None
}

fn multiedit_permission_arguments(call: &ToolCall, working_dir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = path_permission_argument(call, &["file_path", "path"], working_dir) {
        paths.push(path);
    }

    if let Some(edits) = call
        .arguments
        .get("edits")
        .and_then(|value| value.as_array())
    {
        for edit in edits {
            if let Some(path) = edit.get("file_path").and_then(|value| value.as_str()) {
                paths.push(settings_permission_paths::workspace_relative_path(
                    path,
                    working_dir,
                ));
            }
        }
    }

    paths
}

fn glob_permission_argument(call: &ToolCall, working_dir: &Path) -> Option<String> {
    let pattern = call.get_argument::<String>("pattern")?;
    let path = call.get_argument::<String>("path");
    let glob_path = path
        .map(|path| PathBuf::from(path).join(&pattern))
        .unwrap_or_else(|| PathBuf::from(pattern));
    Some(settings_permission_paths::workspace_relative_path(
        &glob_path.to_string_lossy(),
        working_dir,
    ))
}

fn webfetch_permission_argument(call: &ToolCall) -> Option<String> {
    url_permission_argument(call)
}

fn http_client_permission_keys(call: &ToolCall, working_dir: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(url) = url_permission_argument(call) {
        keys.push(format!("http_client({})", url));
    }
    if let Some(path) = path_permission_argument(call, &["save_to_file"], working_dir) {
        keys.push(format!("Write({})", path));
    }
    keys
}

fn url_permission_argument(call: &ToolCall) -> Option<String> {
    let url = call.get_argument::<String>("url")?;
    Some(normalize_webfetch_url(&url))
}

fn normalize_webfetch_url(url: &str) -> String {
    let trimmed = url.trim();
    let Ok(mut parsed) = reqwest::Url::parse(trimmed) else {
        return trimmed.to_string();
    };

    if !matches!(parsed.scheme(), "http" | "https") {
        return trimmed.to_string();
    }

    if let Some(host) = parsed.host_str() {
        let lowercase_host = host.to_ascii_lowercase();
        if parsed.set_host(Some(&lowercase_host)).is_err() {
            return trimmed.to_string();
        }
    }
    if parsed.set_username("").is_err() || parsed.set_password(None).is_err() {
        return trimmed.to_string();
    }
    parsed.set_fragment(None);

    let default_port = match parsed.scheme() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    };
    if parsed.port() == default_port && parsed.set_port(None).is_err() {
        return trimmed.to_string();
    }

    let mut normalized = parsed.to_string();
    if parsed.path() == "/" && parsed.query().is_none() && parsed.fragment().is_none() {
        normalized.truncate(normalized.trim_end_matches('/').len());
    }
    normalized
}
