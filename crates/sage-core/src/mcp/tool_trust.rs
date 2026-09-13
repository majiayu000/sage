//! Trust checks for MCP tool descriptions and schemas.

#[path = "tool_trust_hash.rs"]
mod tool_trust_hash;

use super::error::McpError;
use super::tool_trust_file_lock::ToolTrustFileLock;
use super::types::McpTool;
use crate::config::default_data_dir_or_warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tool_trust_hash::{
    collapse_whitespace, description_option_ambiguous, legacy_raw_baseline_matches, tool_hash,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpToolTrustDecision {
    BaselineCreated { hash: String },
    Unchanged,
    Drift { previous: String, current: String },
}

/// Current tool-identity encoding: present/absent description marker + canonical schema.
const CURRENT_HASH_ENCODING: u32 = 1;

/// Pre-current / unknown-legacy encoding. Only this value may use legacy matching.
const LEGACY_HASH_ENCODING: u32 = 0;

/// Trust-file format version. Missing/`0` is the pre-versioning parent release
/// (`BTreeMap<String, String>` plain hashes). `1` is the versioned encoding era.
const TRUST_FILE_VERSION: u32 = 1;

/// Stored baseline hash. Untagged plain strings from parent-release files
/// (file version 0) are treated as legacy encodings so upgrades can migrate.
/// Once the file is at `TRUST_FILE_VERSION`, any remaining plain string is
/// treated as the current encoding (fail-closed against cross-format preimages)
/// unless rewritten as an explicit legacy `Versioned` encoding on save/load.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredToolHash {
    Plain(String),
    Versioned {
        hash: String,
        #[serde(default = "default_stored_hash_encoding")]
        encoding: u32,
    },
}

fn default_stored_hash_encoding() -> u32 {
    CURRENT_HASH_ENCODING
}

impl StoredToolHash {
    fn current(hash: String) -> Self {
        Self::Versioned {
            hash,
            encoding: CURRENT_HASH_ENCODING,
        }
    }

    fn legacy_plain(hash: String) -> Self {
        Self::Versioned {
            hash,
            encoding: LEGACY_HASH_ENCODING,
        }
    }

    fn hash(&self) -> &str {
        match self {
            Self::Plain(hash) | Self::Versioned { hash, .. } => hash,
        }
    }

    fn encoding(&self, file_version: u32) -> u32 {
        match self {
            Self::Plain(_) if file_version < TRUST_FILE_VERSION => LEGACY_HASH_ENCODING,
            Self::Plain(_) => CURRENT_HASH_ENCODING,
            Self::Versioned { encoding, .. } => *encoding,
        }
    }

    fn allows_legacy_match(&self, file_version: u32) -> bool {
        self.encoding(file_version) == LEGACY_HASH_ENCODING
    }

    fn needs_encoding_persist(&self, file_version: u32) -> bool {
        match self {
            Self::Plain(_) => true,
            Self::Versioned { encoding, .. } => {
                *encoding != CURRENT_HASH_ENCODING || file_version < TRUST_FILE_VERSION
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct McpToolTrustFile {
    /// Absent/`0` = parent-release plain-hash file; `1` = versioned encodings.
    #[serde(default)]
    version: u32,
    #[serde(default)]
    tool_hashes: BTreeMap<String, StoredToolHash>,
}

#[derive(Debug, Clone)]
pub(crate) struct McpToolTrustStore {
    path: PathBuf,
    file_version: u32,
    tool_hashes: BTreeMap<String, StoredToolHash>,
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
                file_version: TRUST_FILE_VERSION,
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
            file_version: file.version,
            tool_hashes: file.tool_hashes,
            dirty: false,
        })
    }

    fn retag_plain_as_explicit_legacy(&mut self) -> bool {
        let mut changed = false;
        for value in self.tool_hashes.values_mut() {
            if let StoredToolHash::Plain(hash) = value.clone() {
                *value = StoredToolHash::legacy_plain(hash);
                changed = true;
            }
        }
        changed
    }

    pub(crate) fn check_tool(&mut self, server_id: &str, tool: &McpTool) -> McpToolTrustDecision {
        let key = tool_key(server_id, &tool.name);
        let hash = tool_hash(tool);
        match self.tool_hashes.get(&key).cloned() {
            // Exact equality is only safe for current-encoding baselines.
            // Legacy hashes can collide with current encodings (e.g. legacy
            // description `1Safe` vs current `Safe` both hash bytes `1Safe`).
            Some(previous)
                if previous.hash() == hash && !previous.allows_legacy_match(self.file_version) =>
            {
                // Persist versioned current encoding on exact matches so plain
                // current hashes under a versioned file stop needing rewrite.
                if previous.needs_encoding_persist(self.file_version) {
                    self.tool_hashes.insert(key, StoredToolHash::current(hash));
                    self.dirty = true;
                }
                McpToolTrustDecision::Unchanged
            }
            Some(previous) => {
                // Upgrade only legacy / parent-release plain baselines. Keep
                // loaded `file_version` stable for the whole refresh loop.
                if previous.allows_legacy_match(self.file_version)
                    && !description_option_ambiguous(tool)
                    && legacy_raw_baseline_matches(previous.hash(), tool)
                {
                    self.tool_hashes.insert(key, StoredToolHash::current(hash));
                    self.dirty = true;
                    return McpToolTrustDecision::Unchanged;
                }
                McpToolTrustDecision::Drift {
                    previous: previous.hash().to_string(),
                    current: hash,
                }
            }
            None => {
                self.tool_hashes
                    .insert(key, StoredToolHash::current(hash.clone()));
                self.dirty = true;
                McpToolTrustDecision::BaselineCreated { hash }
            }
        }
    }

    pub(crate) fn save_if_dirty(&mut self) -> Result<(), McpError> {
        if !self.dirty {
            return Ok(());
        }
        // Ensure untouched Plain entries survive the version bump as explicit
        // legacy encodings across separate per-server load/save transactions.
        if self.file_version < TRUST_FILE_VERSION {
            self.retag_plain_as_explicit_legacy();
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
            version: TRUST_FILE_VERSION,
            tool_hashes: self.tool_hashes.clone(),
        })
        .map_err(|error| {
            McpError::schema(format!(
                "Failed to serialize MCP tool trust baseline: {error}"
            ))
        })?;
        atomic_write(&self.path, content.as_bytes())?;
        self.file_version = TRUST_FILE_VERSION;
        self.dirty = false;
        Ok(())
    }
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), McpError> {
    use std::fs::File;
    use std::io::Write;

    let temp_path = path.with_extension("json.tmp");
    {
        let mut file = File::create(&temp_path).map_err(|error| {
            McpError::schema(format!(
                "Failed to create MCP tool trust baseline temp {}: {}",
                temp_path.display(),
                error
            ))
        })?;
        file.write_all(content).map_err(|error| {
            McpError::schema(format!(
                "Failed to write MCP tool trust baseline temp {}: {}",
                temp_path.display(),
                error
            ))
        })?;
        // Durable publish: sync file contents before renaming into place.
        file.sync_all().map_err(|error| {
            McpError::schema(format!(
                "Failed to sync MCP tool trust baseline temp {}: {}",
                temp_path.display(),
                error
            ))
        })?;
    }
    replace_file(&temp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        McpError::schema(format!(
            "Failed to publish MCP tool trust baseline {}: {}",
            path.display(),
            error
        ))
    })?;
    // Sync the parent directory so the renamed directory entry reaches stable
    // storage on platforms that require it (crash between rename and return
    // must not lose a first-use baseline).
    if let Some(parent) = path.parent() {
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
    }
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
