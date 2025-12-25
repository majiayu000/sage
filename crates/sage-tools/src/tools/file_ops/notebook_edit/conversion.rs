//! Source content conversion utilities for Jupyter notebooks

/// Parse source field to string
#[allow(dead_code)]
pub fn source_to_string(source: &serde_json::Value) -> String {
    match source {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

/// Convert string to source field (array of strings with newlines preserved)
pub fn string_to_source(content: &str) -> serde_json::Value {
    if content.is_empty() {
        return serde_json::Value::Array(vec![]);
    }

    let lines: Vec<serde_json::Value> = content
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            // Add newline to all lines except the last one
            if i < content.split('\n').count() - 1 {
                serde_json::Value::String(format!("{}\n", line))
            } else {
                serde_json::Value::String(line.to_string())
            }
        })
        .collect();

    serde_json::Value::Array(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_conversion() {
        // Test string to source
        let content = "line1\nline2\nline3";
        let source = string_to_source(content);
        assert!(source.is_array());

        // Test source to string
        let result = source_to_string(&source);
        assert_eq!(result, content);
    }
}
