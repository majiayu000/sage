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
        // Hash exact description bytes so whitespace-only drift is detected.
        // Whitespace collapse remains for high-risk phrase scanning only.
        DescriptionEncoding::Present(tool.description.clone()),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Match a pre-canonicalization baseline, including set-array reorder forms.
///
/// Permutes set-like arrays in a single working tree and hashes each candidate
/// immediately (no Cartesian `Vec<Value>` of full schemas). Only explores
/// complete `n!` coverage when it fits the budget; the Cartesian product of
/// multiple affordable set-array factorials must also fit, otherwise the
/// current wire order is tried once. Larger reorderings require re-baselining.
///
/// Legacy case-folded description hashes are not matched unless
/// `parent_release` is set. Parent-release (file version 0) plain hashes used
/// exact description bytes, so already-lowercase tools with already-canonical
/// schemas must still migrate. For explicit legacy encodings under a versioned
/// file, when the current description is already lowercase (so it can collide
/// with a case-folded prior hash), a match is accepted only if the matched
/// schema wire form differs from the canonical schema — proving a lossless
/// pre-canonicalization baseline rather than a lossy prior canonicalizer.
pub(super) fn legacy_raw_baseline_matches(
    previous: &str,
    tool: &McpTool,
    parent_release: bool,
) -> bool {
    let wire = serde_json::to_vec(&tool.input_schema).unwrap_or_default();
    if wire.len() > MAX_LEGACY_SCHEMA_BYTES {
        return lossless_legacy_match(previous, tool, &tool.input_schema, parent_release);
    }

    let mut working = tool.input_schema.clone();
    // Budget by full hash preimage size (name + description + schema), not
    // schema alone — large descriptions would otherwise amplify into GiB-scale
    // hashing under the trust locks.
    let mut budget = legacy_reorder_budget(legacy_hash_preimage_len(tool, wire.len()));
    walk(
        &mut working,
        TraverseMode::Schema,
        &mut budget,
        previous,
        tool,
        parent_release,
    )
}

fn legacy_hash_preimage_len(tool: &McpTool, wire_len: usize) -> usize {
    wire_len
        .saturating_add(tool.name.len())
        .saturating_add(tool.description.as_ref().map_or(0, String::len))
}

fn legacy_reorder_budget(preimage_len: usize) -> usize {
    let max_by_bytes = MAX_LEGACY_SYNC_HASH_BYTES / preimage_len.max(1);
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
    parent_release: bool,
) -> bool {
    if !legacy_hash_eq(previous, tool, matched_schema) {
        return false;
    }
    // Parent-release plain hashes used exact description bytes, not a lossy
    // case-folded hasher. Skip the ambiguity gate so unchanged lowercase tools
    // with already-canonical schemas still upgrade.
    if parent_release {
        return true;
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
/// arrays (`required`, `enum`, multi-type `type`, `allOf`/`anyOf`/`oneOf`,
/// `dependentRequired` value arrays, and Draft-7 `dependencies` property arrays).
/// Integer-valued JSON numbers share one representation only inside the safe
/// integer range (`|n| ≤ 2^53-1`); larger float magnitudes stay as floats so
/// mantissa-rounded decimals cannot collapse onto integer baselines.
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
        Value::Number(number) => canonicalize_json_number(number),
        other => other.clone(),
    }
}

fn child_mode_for_schema_key(key: &str, child: &Value) -> Value {
    if is_schema_valued_map_key(key) {
        canonicalize_schema_value_inner(child, TraverseMode::SchemaMap)
    } else if key == "dependentRequired" {
        canonicalize_property_dependency_map(child, false)
    } else if key == "dependencies" {
        // Draft-7: array values are property deps (order-insensitive); object
        // values are schema deps and stay schemas.
        canonicalize_property_dependency_map(child, true)
    } else if is_literal_valued_schema_key(key) {
        canonicalize_schema_value_inner(child, TraverseMode::Literal)
    } else if is_order_insensitive_schema_key(key) {
        canonicalize_set_like_array(child, key == "enum")
    } else if is_schema_valued_keyword(key) {
        canonicalize_schema_value_inner(child, TraverseMode::Schema)
    } else {
        // Unknown / extension keywords may carry order-sensitive literal
        // payloads (e.g. `x-policy`). Do not reinterpret them as schemas.
        canonicalize_schema_value_inner(child, TraverseMode::Literal)
    }
}

/// Canonicalize `dependentRequired` / Draft-7 `dependencies` maps.
///
/// When `schema_valued_entries` is true (Draft-7 `dependencies`), non-array
/// values are treated as schemas. When false (`dependentRequired`), every value
/// is treated as a property-name array.
fn canonicalize_property_dependency_map(value: &Value, schema_valued_entries: bool) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, child)| {
                        let canon = if schema_valued_entries {
                            match child {
                                Value::Array(_) => canonicalize_set_like_array(child, true),
                                other => {
                                    canonicalize_schema_value_inner(other, TraverseMode::Schema)
                                }
                            }
                        } else {
                            canonicalize_set_like_array(child, true)
                        };
                        (key.clone(), canon)
                    })
                    .collect(),
            )
        }
        other => canonicalize_schema_value_inner(other, TraverseMode::Schema),
    }
}

fn canonicalize_json_number(number: &serde_json::Number) -> Value {
    // Only trust integer/unsigned storage. `as_i64()` on an f64-backed Number
    // can accept mantissa-rounded values (e.g. 2^53+1 → 2^53) and would hide
    // real drift against a true integer baseline.
    if number.is_i64() {
        if let Some(i) = number.as_i64() {
            return Value::Number(i.into());
        }
    }
    if number.is_u64() {
        if let Some(u) = number.as_u64() {
            return Value::Number(u.into());
        }
    }
    if let Some(f) = number.as_f64() {
        if f.is_finite() {
            let normalized = if f == -0.0 { 0.0 } else { f };
            // Integer-valued floats share the integer spelling (`1` == `1.0`)
            // only inside the safe JSON integer range where every integer is
            // uniquely representable in f64. Above |2^53-1|, a decimal such as
            // `9007199254740993.0` rounds before we see it and must not collapse
            // onto integer `9007199254740992`.
            if let Some(as_i64) = exact_i64_from_integral_f64(normalized) {
                return Value::Number(as_i64.into());
            }
            if let Some(canonical) = serde_json::Number::from_f64(normalized) {
                return Value::Number(canonical);
            }
        }
    }
    Value::Number(number.clone())
}

/// Convert an integral finite f64 to i64 only inside the safe integer range.
///
/// `Number.MAX_SAFE_INTEGER` (`2^53 - 1`) is the largest magnitude where every
/// integer is unique in f64; beyond that, neighboring integers round together.
fn exact_i64_from_integral_f64(value: f64) -> Option<i64> {
    if value.fract() != 0.0 {
        return None;
    }
    // 2^53 is exact in f64; 2^53-1 is the last safe consecutive integer.
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    if !(-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&value) {
        return None;
    }
    let as_i64 = value as i64;
    if as_i64 as f64 == value {
        Some(as_i64)
    } else {
        None
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

/// Keywords whose values are schemas (or arrays of schemas), not literals.
pub(super) fn is_schema_valued_keyword(key: &str) -> bool {
    matches!(
        key,
        "items"
            | "additionalItems"
            | "additionalProperties"
            | "not"
            | "if"
            | "then"
            | "else"
            | "contains"
            | "propertyNames"
            | "unevaluatedItems"
            | "unevaluatedProperties"
            | "contentSchema"
            | "prefixItems"
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
