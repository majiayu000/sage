//! Mapping LSP response values into agent-facing navigation output.

use super::client::LspClientError;
use super::types::{NavigationItem, NavigationResponse};
use sage_core::tools::base::ToolError;
use serde_json::{Value, json};
use std::path::PathBuf;
use url::Url;

pub(super) fn response_json(response: NavigationResponse) -> Result<String, ToolError> {
    serde_json::to_string_pretty(&response).map_err(|error| {
        ToolError::ExecutionFailed(format!("failed to encode LSP output: {}", error))
    })
}

pub(super) fn to_tool_error(error: LspClientError) -> ToolError {
    ToolError::ExecutionFailed(error.to_string())
}

pub(super) fn lsp_position(line: u32, character: u32) -> Value {
    json!({
        "line": line.saturating_sub(1),
        "character": character.saturating_sub(1),
    })
}

pub(super) fn merge_object(target: &mut Value, extra: Value) {
    let (Some(target), Some(extra)) = (target.as_object_mut(), extra.as_object()) else {
        return;
    };
    for (key, value) in extra {
        target.insert(key.clone(), value.clone());
    }
}

pub(super) fn location_items(value: &Value) -> Vec<NavigationItem> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(values) => values.iter().filter_map(location_item).collect(),
        Value::Object(_) => location_item(value).into_iter().collect(),
        _ => Vec::new(),
    }
}

pub(super) fn workspace_symbol_items(value: &Value) -> Vec<NavigationItem> {
    let Some(values) = value.as_array() else {
        return Vec::new();
    };
    values.iter().filter_map(workspace_symbol_item).collect()
}

pub(super) fn type_hierarchy_items(
    value: &Value,
    relationship: Option<&str>,
) -> Vec<NavigationItem> {
    let Some(values) = value.as_array() else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| type_hierarchy_item(value, relationship))
        .collect()
}

fn location_item(value: &Value) -> Option<NavigationItem> {
    if value.get("targetUri").is_some() {
        return location_link_item(value);
    }
    let uri = value.get("uri").and_then(Value::as_str)?;
    let range = value.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end");
    Some(NavigationItem {
        file_path: uri_to_path(uri),
        line: one_based(start.get("line")),
        character: one_based(start.get("character")),
        end_line: end.map(|value| one_based(value.get("line"))),
        end_character: end.map(|value| one_based(value.get("character"))),
        name: None,
        kind: None,
        container_name: None,
        relationship: None,
    })
}

fn location_link_item(value: &Value) -> Option<NavigationItem> {
    let uri = value.get("targetUri").and_then(Value::as_str)?;
    let range = value
        .get("targetSelectionRange")
        .or_else(|| value.get("targetRange"))?;
    let start = range.get("start")?;
    let end = range.get("end");
    Some(NavigationItem {
        file_path: uri_to_path(uri),
        line: one_based(start.get("line")),
        character: one_based(start.get("character")),
        end_line: end.map(|value| one_based(value.get("line"))),
        end_character: end.map(|value| one_based(value.get("character"))),
        name: None,
        kind: None,
        container_name: None,
        relationship: None,
    })
}

fn workspace_symbol_item(value: &Value) -> Option<NavigationItem> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string);
    let kind = value
        .get("kind")
        .and_then(Value::as_u64)
        .map(symbol_kind_name)
        .map(str::to_string);
    let container_name = value
        .get("containerName")
        .and_then(Value::as_str)
        .map(str::to_string);

    let location = value.get("location")?;
    let mut item = if location.get("uri").is_some() && location.get("range").is_some() {
        location_item(location)?
    } else {
        let uri = location.get("uri").and_then(Value::as_str)?;
        NavigationItem {
            file_path: uri_to_path(uri),
            line: 1,
            character: 1,
            end_line: None,
            end_character: None,
            name: None,
            kind: None,
            container_name: None,
            relationship: None,
        }
    };
    item.name = name;
    item.kind = kind;
    item.container_name = container_name;
    Some(item)
}

fn type_hierarchy_item(value: &Value, relationship: Option<&str>) -> Option<NavigationItem> {
    let uri = value.get("uri").and_then(Value::as_str)?;
    let range = value.get("selectionRange").or_else(|| value.get("range"))?;
    let start = range.get("start")?;
    let end = range.get("end");
    Some(NavigationItem {
        file_path: uri_to_path(uri),
        line: one_based(start.get("line")),
        character: one_based(start.get("character")),
        end_line: end.map(|value| one_based(value.get("line"))),
        end_character: end.map(|value| one_based(value.get("character"))),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
        kind: value
            .get("kind")
            .and_then(Value::as_u64)
            .map(symbol_kind_name)
            .map(str::to_string),
        container_name: value
            .get("detail")
            .and_then(Value::as_str)
            .map(str::to_string),
        relationship: relationship.map(str::to_string),
    })
}

fn uri_to_path(uri: &str) -> String {
    Url::parse(uri)
        .ok()
        .and_then(|uri| uri.to_file_path().ok())
        .unwrap_or_else(|| PathBuf::from(uri))
        .to_string_lossy()
        .to_string()
}

fn one_based(value: Option<&Value>) -> u32 {
    value.and_then(Value::as_u64).unwrap_or_default() as u32 + 1
}

fn symbol_kind_name(kind: u64) -> &'static str {
    match kind {
        1 => "file",
        2 => "module",
        3 => "namespace",
        4 => "package",
        5 => "class",
        6 => "method",
        7 => "property",
        8 => "field",
        9 => "constructor",
        10 => "enum",
        11 => "interface",
        12 => "function",
        13 => "variable",
        14 => "constant",
        15 => "string",
        16 => "number",
        17 => "boolean",
        18 => "array",
        19 => "object",
        20 => "key",
        21 => "null",
        22 => "enum_member",
        23 => "struct",
        24 => "event",
        25 => "operator",
        26 => "type_parameter",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::NavigationStatus;
    use super::*;

    #[test]
    fn empty_result_is_ok_not_degraded() {
        let response = NavigationResponse::ok("find_references", "rust", "/repo", Vec::new());

        assert_eq!(response.status, NavigationStatus::Ok);
        assert!(response.items.is_empty());
        assert!(response.reason.is_none());
    }

    #[test]
    fn location_links_are_mapped_to_file_line_items() {
        let items = location_items(&json!([{
            "targetUri": "file:///tmp/example.rs",
            "targetSelectionRange": {
                "start": {"line": 4, "character": 8},
                "end": {"line": 4, "character": 15}
            }
        }]));

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].file_path, "/tmp/example.rs");
        assert_eq!(items[0].line, 5);
        assert_eq!(items[0].character, 9);
    }
}
