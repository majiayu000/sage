//! Tool identity hashing and schema canonicalization for MCP trust baselines.

#[path = "tool_trust_legacy_match.rs"]
mod tool_trust_legacy_match;

use super::super::types::McpTool;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tool_trust_legacy_match::walk;

/// Cap permutations explored when matching a pre-canonicalization baseline.
///
/// Sized to cover full `7!` set-array reorderings. Arrays whose `n!` exceeds
/// this budget are not sampled: those baselines must re-baseline under the
/// current canonical hasher (versioned migration).
const LEGACY_REORDER_BUDGET: usize = 5040;

/// Skip permutation reconstruction when schema wire size could amplify into a
/// process-exhausting drift check.
const MAX_LEGACY_SCHEMA_BYTES: usize = 256 * 1024;

/// Cap total synchronous hash bytes (`wire_len × candidates`) during legacy
/// matching so a near-limit schema cannot amplify into GiB-scale work.
const MAX_LEGACY_SYNC_HASH_BYTES: usize = 4 * 1024 * 1024;

pub(super) fn tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(
        &tool.name,
        DescriptionEncoding::Present(tool.description.as_deref().map(collapse_whitespace)),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Match a pre-canonicalization baseline, including set-array reorder forms.
///
/// Permutes set-like arrays in a single working tree and hashes each candidate
/// immediately (no Cartesian `Vec<Value>` of full schemas). Only explores
/// complete `n!` coverage when it fits the budget; larger reorderings require
/// re-baselining.
///
/// Legacy case-folded description hashes are not matched. When the current
/// description is already lowercase (so it can collide with a case-folded
/// prior hash), a match is accepted only if the matched schema wire form
/// differs from the canonical schema — proving a lossless pre-canonicalization
/// baseline rather than a lossy prior canonicalizer.
pub(super) fn legacy_raw_baseline_matches(previous: &str, tool: &McpTool) -> bool {
    let wire = serde_json::to_vec(&tool.input_schema).unwrap_or_default();
    if wire.len() > MAX_LEGACY_SCHEMA_BYTES {
        return lossless_legacy_match(previous, tool, &tool.input_schema);
    }

    let mut working = tool.input_schema.clone();
    let mut budget = legacy_reorder_budget(wire.len());
    walk(
        &mut working,
        TraverseMode::Schema,
        &mut budget,
        previous,
        tool,
    )
}

fn legacy_reorder_budget(wire_len: usize) -> usize {
    let max_by_bytes = MAX_LEGACY_SYNC_HASH_BYTES / wire_len.max(1);
    max_by_bytes.min(LEGACY_REORDER_BUDGET)
}

fn description_casefold_ambiguous(tool: &McpTool) -> bool {
    match tool.description.as_deref().map(collapse_whitespace) {
        None => true,
        Some(text) => text == text.to_ascii_lowercase(),
    }
}

pub(super) fn lossless_legacy_match(
    previous: &str,
    tool: &McpTool,
    matched_schema: &Value,
) -> bool {
    if !legacy_hash_eq(previous, tool, matched_schema) {
        return false;
    }
    if !description_casefold_ambiguous(tool) {
        return true;
    }
    // Description bytes equal the case-folded form, so a canonical-schema
    // match cannot prove the baseline was not written by a lossy hasher.
    let canonical = canonicalize_schema_value(&tool.input_schema);
    serde_json::to_vec(matched_schema).ok() != serde_json::to_vec(&canonical).ok()
}

fn legacy_hash_eq(previous: &str, tool: &McpTool, schema: &Value) -> bool {
    previous
        == hash_tool_parts(
            &tool.name,
            DescriptionEncoding::Legacy(tool.description.clone()),
            schema,
        )
}

enum DescriptionEncoding {
    Present(Option<String>),
    Legacy(Option<String>),
}

fn hash_tool_parts(name: &str, description: DescriptionEncoding, schema: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    match description {
        DescriptionEncoding::Present(None) => hasher.update(b"0"),
        DescriptionEncoding::Present(Some(description)) => {
            hasher.update(b"1");
            hasher.update(description.as_bytes());
        }
        DescriptionEncoding::Legacy(Some(description)) => hasher.update(description.as_bytes()),
        DescriptionEncoding::Legacy(None) => {}
    }
    hasher.update(b"\0");
    hasher
        .update(serde_json::to_vec(schema).unwrap_or_else(|_| b"<unserializable-schema>".to_vec()));
    hex_encode(&hasher.finalize())
}

/// Normalize JSON Schema for trust hashing: sort object keys and order-insensitive
/// arrays (`required`, `enum`, multi-type `type`, `allOf`/`anyOf`/`oneOf`, and
/// `dependentRequired` value arrays).
///
/// Arrays beneath literal-valued keywords (`const`, `default`, `examples`, and
/// `enum` item values) keep their original order. Keys inside schema-valued maps
/// (`properties`, `$defs`, …) are property/definition names, not keywords.
pub(super) fn canonicalize_schema_value(value: &Value) -> Value {
    canonicalize_schema_value_inner(value, TraverseMode::Schema)
}

#[derive(Clone, Copy)]
pub(super) enum TraverseMode {
    /// Normal schema object: member names are keywords.
    Schema,
    /// Schema-valued map (`properties`/`$defs`/…): member names are identifiers.
    SchemaMap,
    /// Literal-valued keyword (`const`/`default`/`examples`): preserve array order.
    Literal,
}

fn canonicalize_schema_value_inner(value: &Value, mode: TraverseMode) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, child)| {
                        let canon = match mode {
                            TraverseMode::Literal => {
                                canonicalize_schema_value_inner(child, TraverseMode::Literal)
                            }
                            TraverseMode::SchemaMap => {
                                // Names are identifiers; values remain schemas.
                                canonicalize_schema_value_inner(child, TraverseMode::Schema)
                            }
                            TraverseMode::Schema => child_mode_for_schema_key(key, child),
                        };
                        (key.clone(), canon)
                    })
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| canonicalize_schema_value_inner(item, mode))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn child_mode_for_schema_key(key: &str, child: &Value) -> Value {
    if is_schema_valued_map_key(key) {
        canonicalize_schema_value_inner(child, TraverseMode::SchemaMap)
    } else if key == "dependentRequired" {
        canonicalize_dependent_required_map(child)
    } else if is_literal_valued_schema_key(key) {
        canonicalize_schema_value_inner(child, TraverseMode::Literal)
    } else if is_order_insensitive_schema_key(key) {
        canonicalize_set_like_array(child, key == "enum")
    } else {
        canonicalize_schema_value_inner(child, TraverseMode::Schema)
    }
}

fn canonicalize_dependent_required_map(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, child)| (key.clone(), canonicalize_set_like_array(child, true)))
                    .collect(),
            )
        }
        other => canonicalize_schema_value_inner(other, TraverseMode::Schema),
    }
}

pub(super) fn is_literal_valued_schema_key(key: &str) -> bool {
    matches!(key, "const" | "default" | "examples")
}

pub(super) fn is_schema_valued_map_key(key: &str) -> bool {
    matches!(
        key,
        "properties" | "patternProperties" | "$defs" | "definitions" | "dependentSchemas"
    )
}

pub(super) fn description_option_ambiguous(tool: &McpTool) -> bool {
    match tool
        .description
        .as_deref()
        .map(collapse_whitespace)
        .as_deref()
    {
        None | Some("") => true,
        Some(_) => false,
    }
}

pub(super) fn is_order_insensitive_schema_key(key: &str) -> bool {
    matches!(
        key,
        "required" | "enum" | "type" | "allOf" | "anyOf" | "oneOf"
    )
}

fn canonicalize_set_like_array(value: &Value, items_are_literals: bool) -> Value {
    match value {
        Value::Array(items) => {
            let item_mode = if items_are_literals {
                TraverseMode::Literal
            } else {
                TraverseMode::Schema
            };
            let mut canon_items: Vec<Value> = items
                .iter()
                .map(|item| canonicalize_schema_value_inner(item, item_mode))
                .collect();
            canon_items.sort_by(|left, right| {
                serde_json::to_string(left)
                    .unwrap_or_default()
                    .cmp(&serde_json::to_string(right).unwrap_or_default())
            });
            Value::Array(canon_items)
        }
        other => canonicalize_schema_value_inner(other, TraverseMode::Schema),
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub(super) fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
