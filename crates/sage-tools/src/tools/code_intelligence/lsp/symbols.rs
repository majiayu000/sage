//! Simple regex-based symbol extraction (fallback when LSP not available)

use super::types::{Location, SymbolInfo};

/// Extract symbols from source code using regex patterns
pub fn extract_symbols_simple(content: &str, language: &str) -> Vec<SymbolInfo> {
    let mut symbols = Vec::new();

    let patterns: Vec<(&str, &str)> = match language {
        "rust" => vec![
            (r"(?m)^pub\s+fn\s+(\w+)", "function"),
            (r"(?m)^fn\s+(\w+)", "function"),
            (r"(?m)^pub\s+struct\s+(\w+)", "struct"),
            (r"(?m)^struct\s+(\w+)", "struct"),
            (r"(?m)^pub\s+enum\s+(\w+)", "enum"),
            (r"(?m)^enum\s+(\w+)", "enum"),
            (r"(?m)^pub\s+trait\s+(\w+)", "trait"),
            (r"(?m)^trait\s+(\w+)", "trait"),
            (r"(?m)^impl\s+(\w+)", "impl"),
        ],
        "typescript" | "javascript" => vec![
            (r"(?m)^export\s+function\s+(\w+)", "function"),
            (r"(?m)^function\s+(\w+)", "function"),
            (r"(?m)^export\s+class\s+(\w+)", "class"),
            (r"(?m)^class\s+(\w+)", "class"),
            (r"(?m)^export\s+interface\s+(\w+)", "interface"),
            (r"(?m)^interface\s+(\w+)", "interface"),
            (r"(?m)^const\s+(\w+)\s*=", "constant"),
        ],
        "python" => vec![
            (r"(?m)^def\s+(\w+)", "function"),
            (r"(?m)^class\s+(\w+)", "class"),
            (r"(?m)^async\s+def\s+(\w+)", "function"),
        ],
        "go" => vec![
            (r"(?m)^func\s+(\w+)", "function"),
            (r"(?m)^func\s+\([^)]+\)\s+(\w+)", "method"),
            (r"(?m)^type\s+(\w+)\s+struct", "struct"),
            (r"(?m)^type\s+(\w+)\s+interface", "interface"),
        ],
        _ => vec![],
    };

    for (pattern, kind) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            for (line_num, line) in content.lines().enumerate() {
                if let Some(caps) = re.captures(line) {
                    if let Some(name) = caps.get(1) {
                        symbols.push(SymbolInfo {
                            name: name.as_str().to_string(),
                            kind: kind.to_string(),
                            location: Location {
                                file_path: String::new(),
                                line: u32::try_from(line_num + 1).unwrap_or(u32::MAX),
                                character: 1,
                                end_line: None,
                                end_character: None,
                            },
                            container_name: None,
                        });
                    }
                }
            }
        }
    }

    symbols
}
