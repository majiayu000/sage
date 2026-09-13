//! Tool identity hashing and schema canonicalization for MCP trust baselines.

use super::super::types::McpTool;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Cap permutations explored when matching a pre-canonicalization baseline.
///
/// Sized to cover full `7!` set-array reorderings. Arrays whose `n!` exceeds
/// this budget are not sampled: those baselines must re-baseline under the
/// current canonical hasher (versioned migration).
const LEGACY_REORDER_BUDGET: usize = 5040;

/// Skip permutation reconstruction when schema wire size could amplify into a
/// process-exhausting drift check.
const MAX_LEGACY_SCHEMA_BYTES: usize = 256 * 1024;

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
    let mut budget = LEGACY_REORDER_BUDGET;
    walk(&mut working, false, &mut budget, previous, tool)
}

fn description_casefold_ambiguous(tool: &McpTool) -> bool {
    match tool.description.as_deref().map(collapse_whitespace) {
        None => true,
        Some(text) => text == text.to_ascii_lowercase(),
    }
}

fn lossless_legacy_match(previous: &str, tool: &McpTool, matched_schema: &Value) -> bool {
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
/// arrays (`required`, `enum`, multi-type `type`, and `allOf`/`anyOf`/`oneOf`).
///
/// Arrays beneath literal-valued keywords (`const`, `default`, `examples`, and
/// `enum` item values) keep their original order.
pub(super) fn canonicalize_schema_value(value: &Value) -> Value {
    canonicalize_schema_value_inner(value, false)
}

fn canonicalize_schema_value_inner(value: &Value, in_literal: bool) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, child)| {
                        let child_literal = in_literal || is_literal_valued_schema_key(key);
                        let canon = if !in_literal && is_order_insensitive_schema_key(key) {
                            canonicalize_set_like_array(child, key == "enum")
                        } else {
                            canonicalize_schema_value_inner(child, child_literal)
                        };
                        (key.clone(), canon)
                    })
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| canonicalize_schema_value_inner(item, in_literal))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn is_literal_valued_schema_key(key: &str) -> bool {
    matches!(key, "const" | "default" | "examples")
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

fn is_order_insensitive_schema_key(key: &str) -> bool {
    matches!(
        key,
        "required" | "enum" | "type" | "allOf" | "anyOf" | "oneOf"
    )
}

fn canonicalize_set_like_array(value: &Value, items_are_literals: bool) -> Value {
    match value {
        Value::Array(items) => {
            let mut canon_items: Vec<Value> = items
                .iter()
                .map(|item| canonicalize_schema_value_inner(item, items_are_literals))
                .collect();
            canon_items.sort_by(|left, right| {
                serde_json::to_string(left)
                    .unwrap_or_default()
                    .cmp(&serde_json::to_string(right).unwrap_or_default())
            });
            Value::Array(canon_items)
        }
        other => canonicalize_schema_value_inner(other, false),
    }
}

/// Depth-first in-place walk. Hashes the whole `root` once each set-array
/// assignment under `node` is complete.
fn walk(
    root: &mut Value,
    in_literal: bool,
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    // Locate the first unset order-insensitive array under root via path, then
    // permute it; when none remain, hash the root once.
    match find_set_array_path(root, in_literal) {
        None => {
            if *budget == 0 {
                return false;
            }
            *budget = budget.saturating_sub(1);
            lossless_legacy_match(previous, tool, root)
        }
        Some(path) => permute_at_path(root, &path, budget, previous, tool),
    }
}

#[derive(Clone)]
enum PathStep {
    Key(String),
    Index(usize),
}

fn find_set_array_path(value: &Value, in_literal: bool) -> Option<Vec<PathStep>> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_literal = in_literal || is_literal_valued_schema_key(key);
                if !in_literal && is_order_insensitive_schema_key(key) {
                    if let Value::Array(items) = child {
                        if items.len() > 1 {
                            return Some(vec![PathStep::Key(key.clone())]);
                        }
                    }
                }
                if let Some(mut sub) = find_set_array_path(child, child_literal) {
                    sub.insert(0, PathStep::Key(key.clone()));
                    return Some(sub);
                }
            }
            None
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                if let Some(mut sub) = find_set_array_path(child, in_literal) {
                    sub.insert(0, PathStep::Index(index));
                    return Some(sub);
                }
            }
            None
        }
        _ => None,
    }
}

fn permute_at_path(
    root: &mut Value,
    path: &[PathStep],
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    let array = match get_array_mut(root, path) {
        Some(array) => array,
        None => return walk_after_skip(root, path, budget, previous, tool),
    };
    let n = array.len();
    if !factorial_fits(n, *budget) {
        // Versioned migration: do not partially sample large set arrays.
        // Leave identity order and continue searching for other set arrays
        // deeper/elsewhere by temporarily treating this node as done: mark by
        // walking children only through a one-shot deeper search that skips
        // this exact path.
        return walk_skipping(root, path, budget, previous, tool);
    }

    // Take ownership of the array for Heap permutation, then restore.
    let mut items = std::mem::take(array);
    let hit = heap_permute(&mut items, n, &mut |perm| {
        if let Some(slot) = get_array_mut(root, path) {
            *slot = perm.to_vec();
        }
        // Continue walk for remaining set arrays / hash.
        // Temporarily hide this array from find_set_array_path by noting we
        // must search after this path — walk from root but skip paths equal
        // to `path` when len<=1 already handled; for permuted array with
        // len>1, find_set_array_path would rediscover it. So search children
        // of this array and siblings only via walk_after_perm.
        walk_after_perm(root, path, budget, previous, tool)
    });
    if let Some(slot) = get_array_mut(root, path) {
        *slot = items;
    }
    hit
}

fn walk_after_perm(
    root: &mut Value,
    path: &[PathStep],
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    // Search for another set-array that is not `path` itself.
    match find_set_array_path_excluding(root, false, path) {
        None => {
            if *budget == 0 {
                return false;
            }
            *budget = budget.saturating_sub(1);
            lossless_legacy_match(previous, tool, root)
        }
        Some(next) => permute_at_path(root, &next, budget, previous, tool),
    }
}

fn walk_after_skip(
    root: &mut Value,
    path: &[PathStep],
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    walk_after_perm(root, path, budget, previous, tool)
}

fn walk_skipping(
    root: &mut Value,
    path: &[PathStep],
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    walk_after_perm(root, path, budget, previous, tool)
}

fn find_set_array_path_excluding(
    value: &Value,
    in_literal: bool,
    exclude: &[PathStep],
) -> Option<Vec<PathStep>> {
    find_set_array_path_excluding_inner(value, in_literal, exclude, &[])
}

fn find_set_array_path_excluding_inner(
    value: &Value,
    in_literal: bool,
    exclude: &[PathStep],
    prefix: &[PathStep],
) -> Option<Vec<PathStep>> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_literal = in_literal || is_literal_valued_schema_key(key);
                let mut here = prefix.to_vec();
                here.push(PathStep::Key(key.clone()));
                if !in_literal && is_order_insensitive_schema_key(key) {
                    if let Value::Array(items) = child {
                        if items.len() > 1 && !path_eq(&here, exclude) {
                            return Some(here);
                        }
                    }
                }
                if let Some(sub) =
                    find_set_array_path_excluding_inner(child, child_literal, exclude, &here)
                {
                    return Some(sub);
                }
            }
            None
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let mut here = prefix.to_vec();
                here.push(PathStep::Index(index));
                if let Some(sub) =
                    find_set_array_path_excluding_inner(child, in_literal, exclude, &here)
                {
                    return Some(sub);
                }
            }
            None
        }
        _ => None,
    }
}

fn path_eq(left: &[PathStep], right: &[PathStep]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right.iter()).all(|(l, r)| match (l, r) {
        (PathStep::Key(a), PathStep::Key(b)) => a == b,
        (PathStep::Index(a), PathStep::Index(b)) => a == b,
        _ => false,
    })
}

fn get_array_mut<'a>(root: &'a mut Value, path: &[PathStep]) -> Option<&'a mut Vec<Value>> {
    let mut cur = root;
    for step in &path[..path.len().saturating_sub(1)] {
        cur = match step {
            PathStep::Key(key) => cur.as_object_mut()?.get_mut(key)?,
            PathStep::Index(index) => cur.as_array_mut()?.get_mut(*index)?,
        };
    }
    match path.last()? {
        PathStep::Key(key) => match cur.as_object_mut()?.get_mut(key)? {
            Value::Array(items) => Some(items),
            _ => None,
        },
        PathStep::Index(index) => match cur.as_array_mut()?.get_mut(*index)? {
            Value::Array(items) => Some(items),
            _ => None,
        },
    }
}

fn heap_permute(
    items: &mut [Value],
    k: usize,
    visit: &mut dyn FnMut(&mut [Value]) -> bool,
) -> bool {
    if k <= 1 {
        return visit(items);
    }
    if heap_permute(items, k - 1, visit) {
        return true;
    }
    for i in 0..k - 1 {
        if k.is_multiple_of(2) {
            items.swap(i, k - 1);
        } else {
            items.swap(0, k - 1);
        }
        if heap_permute(items, k - 1, visit) {
            return true;
        }
    }
    false
}

fn factorial_fits(n: usize, budget: usize) -> bool {
    let mut acc: usize = 1;
    for i in 2..=n {
        match acc.checked_mul(i) {
            Some(next) if next <= budget => acc = next,
            _ => return false,
        }
    }
    true
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
