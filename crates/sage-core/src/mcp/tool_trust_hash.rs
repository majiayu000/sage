//! Tool identity hashing and schema canonicalization for MCP trust baselines.

use super::super::types::McpTool;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Cap permutations explored when matching a pre-canonicalization baseline.
///
/// Sized to cover full `7!` set-array reorderings (common large `required` /
/// `enum` lists) while remaining a hard upper bound so huge arrays cannot
/// trigger unbounded factorial work.
const LEGACY_REORDER_BUDGET: usize = 5040;

pub(super) fn tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(
        &tool.name,
        DescriptionEncoding::Present(tool.description.as_deref().map(collapse_whitespace)),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Prior canonical hash without an Option discriminant for description.
///
/// Retained so baselines written before None/Some("") were distinguished can
/// upgrade in place when the tool identity is otherwise unchanged.
pub(super) fn prior_canonical_tool_hash(tool: &McpTool) -> String {
    hash_tool_parts(
        &tool.name,
        DescriptionEncoding::Legacy(tool.description.as_deref().map(collapse_whitespace)),
        &canonicalize_schema_value(&tool.input_schema),
    )
}

/// Match a pre-canonicalization baseline, including set-array reorder forms.
///
/// Legacy case-folded description hashes are intentionally not matched: they
/// cannot prove capitalization was unchanged and require explicit re-baselining.
pub(super) fn legacy_raw_baseline_matches(previous: &str, tool: &McpTool) -> bool {
    set_order_schema_variants(&tool.input_schema, LEGACY_REORDER_BUDGET)
        .iter()
        .any(|schema| {
            previous
                == hash_tool_parts(
                    &tool.name,
                    DescriptionEncoding::Legacy(tool.description.clone()),
                    schema,
                )
        })
}

enum DescriptionEncoding {
    /// Current encoding: discriminant distinguishes None from Some("").
    Present(Option<String>),
    /// Legacy encoding: absent and empty both contribute no description bytes.
    Legacy(Option<String>),
}

fn hash_tool_parts(name: &str, description: DescriptionEncoding, schema: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    hasher.update(b"\0");
    match description {
        DescriptionEncoding::Present(None) => {
            hasher.update(b"0");
        }
        DescriptionEncoding::Present(Some(description)) => {
            hasher.update(b"1");
            hasher.update(description.as_bytes());
        }
        DescriptionEncoding::Legacy(Some(description)) => {
            hasher.update(description.as_bytes());
        }
        DescriptionEncoding::Legacy(None) => {}
    }
    hasher.update(b"\0");
    hasher
        .update(serde_json::to_vec(schema).unwrap_or_else(|_| b"<unserializable-schema>".to_vec()));
    hex_encode(&hasher.finalize())
}

/// Normalize JSON Schema for trust hashing: sort object keys and order-insensitive
/// set-like arrays such as `required`, `enum`, and multi-type `type`.
///
/// Arrays beneath literal-valued keywords (`const`, `default`, `examples`, and
/// `enum` item values) keep their original order so semantically distinct
/// literals diverge.
pub(super) fn canonicalize_schema_value(value: &Value) -> Value {
    canonicalize_schema_value_inner(value, false)
}

fn canonicalize_schema_value_inner(value: &Value, in_literal: bool) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            let canonical = sorted
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
                .collect();
            Value::Object(canonical)
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

/// Legacy encodings omit an Option discriminant, so absent and empty
/// descriptions collide and cannot safely prove the option state is unchanged.
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
    matches!(key, "required" | "enum" | "type")
}

fn canonicalize_set_like_array(value: &Value, items_are_literals: bool) -> Value {
    match value {
        Value::Array(items) => {
            let mut canon_items: Vec<Value> = items
                .iter()
                .map(|item| canonicalize_schema_value_inner(item, items_are_literals))
                .collect();
            canon_items.sort_by(|left, right| {
                let left_key = serde_json::to_string(left).unwrap_or_default();
                let right_key = serde_json::to_string(right).unwrap_or_default();
                left_key.cmp(&right_key)
            });
            Value::Array(canon_items)
        }
        // JSON Schema `type` is often a single string; leave non-arrays alone.
        other => canonicalize_schema_value_inner(other, false),
    }
}

fn set_order_schema_variants(value: &Value, budget: usize) -> Vec<Value> {
    set_order_schema_variants_inner(value, false, budget)
}

fn set_order_schema_variants_inner(value: &Value, in_literal: bool, budget: usize) -> Vec<Value> {
    if budget == 0 {
        return vec![value.clone()];
    }
    match value {
        Value::Object(map) => {
            let mut per_key: Vec<(String, Vec<Value>)> = Vec::with_capacity(map.len());
            for (key, child) in map {
                let child_literal = in_literal || is_literal_valued_schema_key(key);
                let variants = if !in_literal && is_order_insensitive_schema_key(key) {
                    array_order_variants(child, key == "enum", budget)
                } else {
                    set_order_schema_variants_inner(child, child_literal, budget)
                };
                per_key.push((key.clone(), variants));
            }
            cartesian_objects(per_key, budget)
        }
        Value::Array(items) => {
            let per_item: Vec<Vec<Value>> = items
                .iter()
                .map(|item| set_order_schema_variants_inner(item, in_literal, budget))
                .collect();
            cartesian_arrays(per_item, budget)
        }
        other => vec![other.clone()],
    }
}

fn array_order_variants(value: &Value, items_are_literals: bool, budget: usize) -> Vec<Value> {
    match value {
        Value::Array(items) => {
            let item_variants: Vec<Vec<Value>> = items
                .iter()
                .map(|item| set_order_schema_variants_inner(item, items_are_literals, budget))
                .collect();
            let base_arrays = cartesian_arrays(item_variants, budget);
            let mut out = Vec::new();
            for array in base_arrays {
                if let Value::Array(elements) = array {
                    for perm in permutations(elements, budget.saturating_sub(out.len())) {
                        out.push(Value::Array(perm));
                        if out.len() >= budget {
                            return out;
                        }
                    }
                }
            }
            if out.is_empty() {
                vec![value.clone()]
            } else {
                out
            }
        }
        other => set_order_schema_variants_inner(other, false, budget),
    }
}

fn permutations(items: Vec<Value>, budget: usize) -> Vec<Vec<Value>> {
    if budget == 0 {
        return Vec::new();
    }
    if items.len() <= 1 {
        return vec![items];
    }

    // Size-safe large-array path: recognize set-equivalent orderings without
    // exploring full n! when n! would exceed `budget`. Always include the
    // current order and the sorted representative, then fill remaining budget
    // with Heap permutations (early-exit in the caller on hash match).
    if factorial_exceeds(items.len(), budget) {
        return bounded_set_order_variants(items, budget);
    }

    let mut out = Vec::new();
    permute_recurse(items, &mut out, budget);
    out
}

fn factorial_exceeds(n: usize, budget: usize) -> bool {
    let mut acc: usize = 1;
    for i in 2..=n {
        match acc.checked_mul(i) {
            Some(next) if next <= budget => acc = next,
            _ => return true,
        }
    }
    false
}

fn json_sort_key(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn sort_values(items: &mut [Value]) {
    items.sort_by_key(json_sort_key);
}

/// Build a bounded set of orderings for large set-like arrays.
///
/// Includes identity + sorted (O(n log n) set-equivalence representatives) and
/// then budget-limited Heap permutations so legacy baselines stored under an
/// arbitrary prior ordering can still match after a server-side reorder.
fn bounded_set_order_variants(items: Vec<Value>, budget: usize) -> Vec<Vec<Value>> {
    let mut out: Vec<Vec<Value>> = Vec::new();
    let push_unique = |candidate: Vec<Value>, out: &mut Vec<Vec<Value>>| {
        if out.len() >= budget {
            return;
        }
        if !out.iter().any(|existing| existing == &candidate) {
            out.push(candidate);
        }
    };

    push_unique(items.clone(), &mut out);

    let mut sorted = items.clone();
    sort_values(&mut sorted);
    push_unique(sorted, &mut out);

    if out.len() >= budget {
        return out;
    }

    let mut perms = Vec::new();
    permute_recurse(items, &mut perms, budget);
    for perm in perms {
        push_unique(perm, &mut out);
        if out.len() >= budget {
            break;
        }
    }
    out
}

fn permute_recurse(mut items: Vec<Value>, out: &mut Vec<Vec<Value>>, budget: usize) {
    if out.len() >= budget {
        return;
    }
    let n = items.len();
    if n == 0 {
        out.push(Vec::new());
        return;
    }
    heap_permute(&mut items, n, out, budget);
}

fn heap_permute(items: &mut [Value], k: usize, out: &mut Vec<Vec<Value>>, budget: usize) {
    if out.len() >= budget {
        return;
    }
    if k == 1 {
        out.push(items.to_vec());
        return;
    }
    heap_permute(items, k - 1, out, budget);
    for i in 0..k - 1 {
        if out.len() >= budget {
            return;
        }
        if k.is_multiple_of(2) {
            items.swap(i, k - 1);
        } else {
            items.swap(0, k - 1);
        }
        heap_permute(items, k - 1, out, budget);
    }
}

fn cartesian_objects(per_key: Vec<(String, Vec<Value>)>, budget: usize) -> Vec<Value> {
    let mut acc = vec![serde_json::Map::new()];
    for (key, variants) in per_key {
        let mut next = Vec::new();
        for base in &acc {
            for variant in &variants {
                if next.len() >= budget {
                    return next.into_iter().map(Value::Object).collect();
                }
                let mut map = base.clone();
                map.insert(key.clone(), variant.clone());
                next.push(map);
            }
        }
        acc = next;
        if acc.is_empty() {
            break;
        }
    }
    acc.into_iter().map(Value::Object).collect()
}

fn cartesian_arrays(per_item: Vec<Vec<Value>>, budget: usize) -> Vec<Value> {
    let mut acc: Vec<Vec<Value>> = vec![Vec::new()];
    for variants in per_item {
        let mut next = Vec::new();
        for base in &acc {
            for variant in &variants {
                if next.len() >= budget {
                    return next.into_iter().map(Value::Array).collect();
                }
                let mut row = base.clone();
                row.push(variant.clone());
                next.push(row);
            }
        }
        acc = next;
        if acc.is_empty() {
            break;
        }
    }
    acc.into_iter().map(Value::Array).collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

/// Collapse runs of whitespace without changing letter case (trust hashing).
pub(super) fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_hash_distinguishes_absent_and_empty_description() {
        let absent = McpTool::new("geo");
        let empty = McpTool::new("geo").with_description("");
        assert_ne!(tool_hash(&absent), tool_hash(&empty));
    }

    #[test]
    fn tool_hash_preserves_literal_required_array_order() {
        let left = McpTool::new("auth").with_input_schema(json!({
            "const": { "required": ["admin", "user"] }
        }));
        let right = McpTool::new("auth").with_input_schema(json!({
            "const": { "required": ["user", "admin"] }
        }));
        assert_ne!(tool_hash(&left), tool_hash(&right));
    }

    #[test]
    fn tool_hash_preserves_examples_required_array_order() {
        let left = McpTool::new("auth").with_input_schema(json!({
            "examples": [{ "required": ["admin", "user"] }]
        }));
        let right = McpTool::new("auth").with_input_schema(json!({
            "examples": [{ "required": ["user", "admin"] }]
        }));
        assert_ne!(tool_hash(&left), tool_hash(&right));
    }

    #[test]
    fn legacy_raw_matches_required_reorder() {
        let original = McpTool::new("search").with_input_schema(json!({
            "type": "object",
            "required": ["b", "a"],
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "string" }
            }
        }));
        let reordered = McpTool::new("search").with_input_schema(json!({
            "type": "object",
            "required": ["a", "b"],
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "string" }
            }
        }));
        let previous = hash_tool_parts(
            &original.name,
            DescriptionEncoding::Legacy(original.description.clone()),
            &original.input_schema,
        );
        assert!(legacy_raw_baseline_matches(&previous, &reordered));
    }

    #[test]
    fn legacy_raw_matches_large_required_reorder() {
        // Seven-plus entries previously short-circuited to identity-only and
        // false-drifted on equivalent server-side reorders during migration.
        let original = McpTool::new("search").with_input_schema(json!({
            "type": "object",
            "required": ["g", "f", "e", "d", "c", "b", "a"],
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "string" },
                "c": { "type": "string" },
                "d": { "type": "string" },
                "e": { "type": "string" },
                "f": { "type": "string" },
                "g": { "type": "string" }
            }
        }));
        let reordered = McpTool::new("search").with_input_schema(json!({
            "type": "object",
            "required": ["a", "b", "c", "d", "e", "f", "g"],
            "properties": {
                "a": { "type": "string" },
                "b": { "type": "string" },
                "c": { "type": "string" },
                "d": { "type": "string" },
                "e": { "type": "string" },
                "f": { "type": "string" },
                "g": { "type": "string" }
            }
        }));
        let previous = hash_tool_parts(
            &original.name,
            DescriptionEncoding::Legacy(original.description.clone()),
            &original.input_schema,
        );
        assert!(legacy_raw_baseline_matches(&previous, &reordered));
    }
}
