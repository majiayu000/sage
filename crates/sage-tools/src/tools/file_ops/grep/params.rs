//! Tool parameter definitions for grep

use sage_core::tools::types::ToolParameter;
use std::collections::HashMap;

/// Get all tool parameters for the grep tool
pub fn get_tool_parameters() -> Vec<ToolParameter> {
    vec![
        ToolParameter::string("pattern", "The regex pattern to search for"),
        ToolParameter::optional_string(
            "path",
            "File or directory to search (default: current directory)",
        ),
        ToolParameter::optional_string(
            "glob",
            "Filter files by glob pattern (e.g., '*.rs', '**/*.ts')",
        ),
        ToolParameter::optional_string(
            "type",
            "Filter by file type: rs, js, ts, py, go, java, c, cpp, rb, php, html, css, json, yaml, xml, md, txt, toml, sql, sh",
        ),
        ToolParameter::optional_string(
            "output_mode",
            "Output mode: 'content' (matching lines), 'files_with_matches' (file paths), 'count' (match counts). Default: 'files_with_matches'",
        ),
        ToolParameter {
            name: "-i".to_string(),
            description: "Case insensitive search".to_string(),
            param_type: "boolean".to_string(),
            required: false,
            default: Some(serde_json::json!(false)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "-n".to_string(),
            description: "Show line numbers (only for output_mode='content')".to_string(),
            param_type: "boolean".to_string(),
            required: false,
            default: Some(serde_json::json!(true)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "-B".to_string(),
            description: "Lines to show before each match (only for output_mode='content')"
                .to_string(),
            param_type: "number".to_string(),
            required: false,
            default: Some(serde_json::json!(0)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "-A".to_string(),
            description: "Lines to show after each match (only for output_mode='content')"
                .to_string(),
            param_type: "number".to_string(),
            required: false,
            default: Some(serde_json::json!(0)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "-C".to_string(),
            description:
                "Lines of context (before and after) for each match (only for output_mode='content')"
                    .to_string(),
            param_type: "number".to_string(),
            required: false,
            default: Some(serde_json::json!(0)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "multiline".to_string(),
            description: "Enable multiline mode where . matches newlines".to_string(),
            param_type: "boolean".to_string(),
            required: false,
            default: Some(serde_json::json!(false)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "head_limit".to_string(),
            description: "Limit output to first N results (0 = unlimited)".to_string(),
            param_type: "number".to_string(),
            required: false,
            default: Some(serde_json::json!(0)),
            enum_values: None,
            properties: HashMap::new(),
        },
        ToolParameter {
            name: "offset".to_string(),
            description: "Skip first N results".to_string(),
            param_type: "number".to_string(),
            required: false,
            default: Some(serde_json::json!(0)),
            enum_values: None,
            properties: HashMap::new(),
        },
    ]
}
