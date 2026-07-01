//! Permission decision input routing for settings-backed tool checks.

use crate::permissions::{PermissionAction, PermissionDecisionInput, PermissionPreflight};
use crate::tools::types::ToolCall;
use std::path::{Path, PathBuf};

use super::settings_permission_keys;

pub(super) fn settings_permission_inputs(
    tool_name: &str,
    tool_call: &ToolCall,
    working_dir: &Path,
    keys: Vec<String>,
    preflight_denies: Vec<PermissionPreflight>,
    scoped_allows: Vec<PermissionPreflight>,
) -> Vec<PermissionDecisionInput> {
    let lower = tool_name.to_ascii_lowercase();
    match lower.as_str() {
        "read" | "write" | "edit" | "multiedit" | "notebookedit" | "grep" | "glob" => {
            filesystem_inputs(
                tool_name,
                tool_call,
                working_dir,
                preflight_denies,
                scoped_allows,
            )
        }
        "http_client" => http_client_inputs(
            tool_name,
            tool_call,
            working_dir,
            keys,
            preflight_denies,
            scoped_allows,
        ),
        "log_analyzer" => file_reading_tool_inputs(
            tool_name,
            tool_call,
            working_dir,
            keys,
            preflight_denies,
            scoped_allows,
        ),
        "bash" => vec![with_preflights(
            PermissionDecisionInput::new(PermissionAction::Exec, tool_name, keys),
            preflight_denies,
            scoped_allows,
        )],
        "webfetch" | "openbrowser" | "websearch" => vec![with_preflights(
            network_input(tool_name, tool_call, keys),
            preflight_denies,
            scoped_allows,
        )],
        _ => vec![with_preflights(
            PermissionDecisionInput::new(PermissionAction::Tool, tool_name, keys),
            preflight_denies,
            scoped_allows,
        )],
    }
}

fn filesystem_inputs(
    tool_name: &str,
    tool_call: &ToolCall,
    working_dir: &Path,
    preflight_denies: Vec<PermissionPreflight>,
    scoped_allows: Vec<PermissionPreflight>,
) -> Vec<PermissionDecisionInput> {
    let paths = filesystem_paths(&tool_name.to_ascii_lowercase(), tool_call);
    if paths.is_empty() {
        return vec![with_preflights(
            PermissionDecisionInput::new(
                PermissionAction::Filesystem,
                tool_name,
                settings_permission_keys::actual_permission_keys(tool_name, tool_call, working_dir),
            ),
            preflight_denies,
            scoped_allows,
        )];
    }

    paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let input = PermissionDecisionInput::new(
                PermissionAction::Filesystem,
                tool_name,
                settings_permission_keys::actual_permission_keys(tool_name, tool_call, working_dir),
            )
            .with_path(path)
            .with_working_directory(working_dir.to_string_lossy());
            if index == 0 {
                with_preflights(input, preflight_denies.clone(), scoped_allows.clone())
            } else {
                input
            }
        })
        .collect()
}

fn http_client_inputs(
    tool_name: &str,
    tool_call: &ToolCall,
    working_dir: &Path,
    keys: Vec<String>,
    preflight_denies: Vec<PermissionPreflight>,
    scoped_allows: Vec<PermissionPreflight>,
) -> Vec<PermissionDecisionInput> {
    let mut inputs = Vec::new();
    if tool_call.get_argument::<String>("url").is_some() {
        inputs.push(with_preflights(
            network_input(
                tool_name,
                tool_call,
                keys.iter()
                    .filter(|key| key.starts_with("http_client("))
                    .cloned()
                    .collect(),
            ),
            preflight_denies,
            scoped_allows,
        ));
    }
    if let Some(path) = tool_call.get_argument::<String>("save_to_file") {
        inputs.push(
            PermissionDecisionInput::new(
                PermissionAction::Filesystem,
                "Write",
                keys.iter()
                    .filter(|key| key.starts_with("Write("))
                    .cloned()
                    .collect(),
            )
            .with_path(path)
            .with_working_directory(working_dir.to_string_lossy()),
        );
    }
    if inputs.is_empty() {
        inputs.push(PermissionDecisionInput::new(
            PermissionAction::Tool,
            tool_name,
            keys,
        ));
    }
    inputs
}

fn file_reading_tool_inputs(
    tool_name: &str,
    tool_call: &ToolCall,
    working_dir: &Path,
    keys: Vec<String>,
    preflight_denies: Vec<PermissionPreflight>,
    scoped_allows: Vec<PermissionPreflight>,
) -> Vec<PermissionDecisionInput> {
    if let Some(path) = tool_call.get_argument::<String>("file_path") {
        return vec![with_preflights(
            PermissionDecisionInput::new(PermissionAction::Filesystem, "Read", keys)
                .with_path(path)
                .with_working_directory(working_dir.to_string_lossy()),
            preflight_denies,
            scoped_allows,
        )];
    }

    vec![with_preflights(
        PermissionDecisionInput::new(PermissionAction::Tool, tool_name, keys),
        preflight_denies,
        scoped_allows,
    )]
}

fn network_input(
    tool_name: &str,
    tool_call: &ToolCall,
    keys: Vec<String>,
) -> PermissionDecisionInput {
    let input = PermissionDecisionInput::new(PermissionAction::Network, tool_name, keys);
    if let Some(url) = tool_call
        .get_argument::<String>("url")
        .filter(|url| !url.trim().is_empty())
    {
        input.with_network_target(url)
    } else if let Some(query) = tool_call
        .get_argument::<String>("query")
        .filter(|query| !query.trim().is_empty())
    {
        input.with_network_target(query)
    } else {
        input
    }
}

fn filesystem_paths(tool_name: &str, tool_call: &ToolCall) -> Vec<String> {
    match tool_name {
        "read" | "write" | "edit" => tool_call
            .get_argument::<String>("file_path")
            .or_else(|| tool_call.get_argument::<String>("path"))
            .into_iter()
            .collect(),
        "notebookedit" => tool_call
            .get_argument::<String>("notebook_path")
            .into_iter()
            .collect(),
        "multiedit" => multiedit_paths(tool_call),
        "grep" => vec![
            tool_call
                .get_argument::<String>("path")
                .unwrap_or_else(|| ".".to_string()),
        ],
        "glob" => glob_paths(tool_call),
        _ => Vec::new(),
    }
}

fn glob_paths(tool_call: &ToolCall) -> Vec<String> {
    let path = tool_call.get_argument::<String>("path");
    let pattern = tool_call.get_argument::<String>("pattern");
    match (path, pattern) {
        (Some(path), Some(pattern)) => {
            vec![
                PathBuf::from(path)
                    .join(pattern)
                    .to_string_lossy()
                    .to_string(),
            ]
        }
        (Some(path), None) => vec![path],
        (None, Some(pattern)) => vec![pattern],
        (None, None) => Vec::new(),
    }
}

fn multiedit_paths(tool_call: &ToolCall) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(path) = tool_call
        .get_argument::<String>("file_path")
        .or_else(|| tool_call.get_argument::<String>("path"))
    {
        paths.push(path);
    }
    if let Some(edits) = tool_call
        .arguments
        .get("edits")
        .and_then(|value| value.as_array())
    {
        paths.extend(
            edits
                .iter()
                .filter_map(|edit| edit.get("file_path").and_then(|value| value.as_str()))
                .map(ToString::to_string),
        );
    }
    paths
}

fn with_preflights(
    input: PermissionDecisionInput,
    preflight_denies: Vec<PermissionPreflight>,
    scoped_allows: Vec<PermissionPreflight>,
) -> PermissionDecisionInput {
    input
        .with_preflight_denies(preflight_denies)
        .with_scoped_allows(scoped_allows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn pathless_grep_uses_workspace_scope() {
        let call = ToolCall::new("call-1", "grep", HashMap::new());

        assert_eq!(filesystem_paths("grep", &call), vec![".".to_string()]);
    }
}
