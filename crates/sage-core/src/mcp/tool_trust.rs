//! Trust checks for MCP tool descriptions and schemas.

use super::error::McpError;
use super::tool_trust_file_lock::ToolTrustFileLock;
use super::types::McpTool;
use crate::config::default_data_dir_or_warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpToolTrustDecision {
    BaselineCreated { hash: String },
    Unchanged,
    Drift { previous: String, current: String },
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct McpToolTrustFile {
    #[serde(default)]
    tool_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub(crate) struct McpToolTrustStore {
    path: PathBuf,
    tool_hashes: BTreeMap<String, String>,
    dirty: bool,
}

impl McpToolTrustStore {
    /// Load the default trust baseline while holding an inter-process file lock.
    ///
    /// Callers must keep the returned lock alive through `save_if_dirty` so
    /// concurrent Sage processes cannot lose first-use baselines.
    pub(crate) fn load_default_locked() -> Result<(ToolTrustFileLock, Self), McpError> {
        Self::load_locked(default_path())
    }

    pub(crate) fn load_locked(
        path: impl Into<PathBuf>,
    ) -> Result<(ToolTrustFileLock, Self), McpError> {
        let path = path.into();
        let lock = ToolTrustFileLock::acquire(&path)?;
        let store = Self::load(&path)?;
        Ok((lock, store))
    }

    pub(crate) fn load(path: impl Into<PathBuf>) -> Result<Self, McpError> {
        let path = path.into();
        if !path.exists() {
            return Ok(Self {
                path,
                tool_hashes: BTreeMap::new(),
                dirty: false,
            });
        }

        let content = std::fs::read_to_string(&path).map_err(|error| {
            McpError::schema(format!(
                "Failed to read MCP tool trust baseline {}: {}",
                path.display(),
                error
            ))
        })?;
        let file: McpToolTrustFile = serde_json::from_str(&content).map_err(|error| {
            McpError::schema(format!(
                "Failed to parse MCP tool trust baseline {}: {}",
                path.display(),
                error
            ))
        })?;

        Ok(Self {
            path,
            tool_hashes: file.tool_hashes,
            dirty: false,
        })
    }

    pub(crate) fn check_tool(&mut self, server_id: &str, tool: &McpTool) -> McpToolTrustDecision {
        let key = tool_key(server_id, &tool.name);
        let hash = tool_hash(tool);
        match self.tool_hashes.get(&key) {
            Some(previous) if previous == &hash => McpToolTrustDecision::Unchanged,
            Some(previous) => {
                // Upgrade older hash encodings in place so schema reorder or the
                // prior case-folded description form alone does not false-positive.
                if previous == &casefolded_tool_hash(tool) || previous == &legacy_tool_hash(tool) {
                    self.tool_hashes.insert(key, hash);
                    self.dirty = true;
                    return McpToolTrustDecision::Unchanged;
                }
                McpToolTrustDecision::Drift {
                    previous: previous.clone(),
                    current: hash,
                }
            }
            None => {
                self.tool_hashes.insert(key, hash.clone());
                self.dirty = true;
                McpToolTrustDecision::BaselineCreated { hash }
            }
        }
    }

    pub(crate) fn save_if_dirty(&mut self) -> Result<(), McpError> {
        if !self.dirty {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                McpError::schema(format!(
                    "Failed to create MCP tool trust directory {}: {}",
                    parent.display(),
                    error
                ))
            })?;
        }
        let content = serde_json::to_string_pretty(&McpToolTrustFile {
            tool_hashes: self.tool_hashes.clone(),
        })
        .map_err(|error| {
            McpError::schema(format!(
                "Failed to serialize MCP tool trust baseline: {error}"
            ))
        })?;
        atomic_write(&self.path, content.as_bytes())?;
        self.dirty = false;
        Ok(())
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), McpError> {
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, content).map_err(|error| {
        McpError::schema(format!(
            "Failed to write MCP tool trust baseline temp {}: {}",
            temp_path.display(),
            error
        ))
    })?;
    replace_file(&temp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        McpError::schema(format!(
            "Failed to publish MCP tool trust baseline {}: {}",
            path.display(),
            error
        ))
    })?;
    Ok(())
}

/// Atomically publish `from` over `to`.
///
/// Unix `rename(2)` replaces an existing destination. On Windows, use
/// `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` so the baseline is never
/// deleted before the replacement lands (a remove-then-rename gap would accept
/// drifted schemas as a fresh baseline after a crash).
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(
                lpExistingFileName: *const u16,
                lpNewFileName: *const u16,
                dwFlags: u32,
            ) -> i32;
        }

        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

        let from_wide: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let to_wide: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        // SAFETY: both paths are NUL-terminated wide strings; flags request an
        // atomic replace of an existing destination without a delete gap.
        let ok = unsafe {
            MoveFileExW(
                from_wide.as_ptr(),
                to_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if ok == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(from, to)
    }
}

pub(crate) fn validate_tool_description_trust(
    server_id: &str,
    tool: &McpTool,
) -> Result<(), McpError> {
    let mut texts = vec![("tool name", tool.name.as_str())];
    if let Some(description) = tool.description.as_deref() {
        texts.push(("tool description", description));
    }
    collect_schema_trust_texts(&tool.input_schema, &mut texts);

    for (location, text) in texts {
        if let Some(phrase) = high_risk_phrase(text) {
            return Err(McpError::schema(format!(
                "MCP server '{server_id}' tool '{}' has untrusted {location}: matched high-risk phrase '{phrase}'",
                tool.name
            )));
        }
    }
    Ok(())
}

fn default_path() -> PathBuf {
    default_data_dir_or_warn().join("mcp_tool_trust.json")
}

fn tool_key(server_id: &str, tool_name: &str) -> String {
    serde_json::to_string(&(server_id, tool_name))
        .unwrap_or_else(|_| format!("{}\0{}", server_id, tool_name))
}

fn tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(
        &tool.name,
        tool.description.as_deref().map(collapse_whitespace),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Prior hash that case-folded descriptions; retained for baseline upgrade.
fn casefolded_tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(
        &tool.name,
        tool.description.as_deref().map(normalize_whitespace),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Pre-canonicalization hash retained so existing baselines can be upgraded
/// without treating equivalent schemas as drift.
fn legacy_tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(&tool.name, tool.description.clone(), &tool.input_schema)
}

fn hash_tool_parts(name: &str, description: Option<String>, schema: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    if let Some(description) = description.as_deref() {
        hasher.update(description.as_bytes());
    }
    hasher.update(b"\0");
    hasher
        .update(serde_json::to_vec(schema).unwrap_or_else(|_| b"<unserializable-schema>".to_vec()));
    hex_encode(&hasher.finalize())
}

/// Normalize JSON Schema for trust hashing: sort object keys and order-insensitive
/// set-like arrays such as `required`, `enum`, and multi-type `type`.
fn canonicalize_schema_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            let canonical = sorted
                .into_iter()
                .map(|(key, child)| {
                    let canon = if is_order_insensitive_schema_key(key) {
                        canonicalize_set_like_array(child)
                    } else {
                        canonicalize_schema_value(child)
                    };
                    (key.clone(), canon)
                })
                .collect();
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_schema_value).collect()),
        other => other.clone(),
    }
}

fn is_order_insensitive_schema_key(key: &str) -> bool {
    matches!(key, "required" | "enum" | "type")
}

fn canonicalize_set_like_array(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            let mut canon_items: Vec<Value> = items.iter().map(canonicalize_schema_value).collect();
            canon_items.sort_by(|left, right| {
                let left_key = serde_json::to_string(left).unwrap_or_default();
                let right_key = serde_json::to_string(right).unwrap_or_default();
                left_key.cmp(&right_key)
            });
            Value::Array(canon_items)
        }
        // JSON Schema `type` is often a single string; leave non-arrays alone.
        other => canonicalize_schema_value(other),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn collect_schema_trust_texts<'a>(value: &'a Value, texts: &mut Vec<(&'static str, &'a str)>) {
    match value {
        Value::Object(object) => {
            for key in object.keys() {
                texts.push(("schema key", key.as_str()));
            }
            if let Some(description) = object.get("description").and_then(|v| v.as_str()) {
                texts.push(("schema description", description));
            }
            for value in object.values() {
                collect_schema_trust_texts(value, texts);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_schema_trust_texts(value, texts);
            }
        }
        _ => {}
    }
}

fn high_risk_phrase(text: &str) -> Option<&'static str> {
    let lower = normalize_whitespace(text);
    if contains_previous_instruction_override(&lower, "ignore") {
        return Some("ignore previous instructions");
    }
    if contains_previous_instruction_override(&lower, "disregard") {
        return Some("disregard previous instructions");
    }
    const HIGH_RISK_WORD_PHRASES: &[&str] = &[
        "override system",
        "override developer",
        "override system prompt",
        "ignore system prompt",
        "reveal system prompt",
        "you must obey this tool",
        "act as system",
    ];

    HIGH_RISK_WORD_PHRASES
        .iter()
        .copied()
        .find(|phrase| contains_word_phrase(&lower, phrase))
        .or_else(|| contains_priority_authority_claim(&lower).then_some("higher priority than"))
}

fn contains_priority_authority_claim(text: &str) -> bool {
    contains_word_phrase(text, "higher priority than")
        && [
            "system prompt",
            "system message",
            "developer message",
            "developer instruction",
            "developer instructions",
            "user instruction",
            "user instructions",
            "previous instruction",
            "previous instructions",
        ]
        .into_iter()
        .any(|phrase| contains_word_phrase(text, phrase))
}

/// Collapse runs of whitespace without changing letter case (trust hashing).
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Collapse whitespace and case-fold for high-risk phrase scanning only.
fn normalize_whitespace(text: &str) -> String {
    collapse_whitespace(text).to_ascii_lowercase()
}

fn normalized_word_tokens(text: &str) -> Vec<&str> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
}

fn contains_word_phrase(text: &str, phrase: &str) -> bool {
    let tokens = normalized_word_tokens(text);
    let phrase_tokens = normalized_word_tokens(phrase);
    if phrase_tokens.is_empty() {
        return false;
    }
    tokens
        .windows(phrase_tokens.len())
        .any(|window| window == phrase_tokens.as_slice())
}

fn contains_previous_instruction_override(text: &str, verb: &str) -> bool {
    let tokens = normalized_word_tokens(text);
    for index in 0..tokens.len() {
        if tokens[index] != verb {
            continue;
        }
        let mut cursor = index + 1;
        while cursor < tokens.len()
            && matches!(tokens[cursor], "all" | "any" | "the" | "your" | "of")
        {
            cursor += 1;
        }
        if matches!(
            tokens.get(cursor),
            Some(&"previous" | &"prior" | &"earlier")
        ) && tokens.get(cursor + 1) == Some(&"instructions")
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[path = "tool_trust_tests.rs"]
mod tool_trust_tests;
