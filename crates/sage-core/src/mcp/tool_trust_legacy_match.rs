//! In-place set-array permutation matching for pre-canonicalization baselines.

use super::{
    TraverseMode, is_literal_valued_schema_key, is_order_insensitive_schema_key,
    is_schema_valued_map_key, lossless_legacy_match,
};
use crate::mcp::types::McpTool;
use serde_json::Value;

/// Depth-first in-place walk. Hashes the whole `root` once each set-array
/// assignment under `node` is complete.
///
/// Before permuting, refuses Cartesian products of individually-affordable
/// set-array factorials that would exceed `budget` (availability DoS under the
/// process-wide trust lock). Large single arrays that fail `n! <= budget` are
/// still skipped individually so sibling small arrays may be explored.
pub(super) fn walk(
    root: &mut Value,
    mode: TraverseMode,
    budget: &mut usize,
    previous: &str,
    tool: &McpTool,
) -> bool {
    // Individually oversized arrays are skipped (not partially sampled). The
    // product of the remaining affordable factorials must still fit the budget;
    // otherwise try the current wire order once and stop.
    if !permutable_cartesian_fits(root, mode, *budget) {
        return hash_once(root, budget, previous, tool);
    }

    let mut abort = false;
    // Locate the first unset order-insensitive array under root via path, then
    // permute it; when none remain, hash the root once.
    match find_set_array_path(root, mode) {
        None => hash_once(root, budget, previous, tool),
        Some(path) => permute_at_path(root, &path, &[], budget, &mut abort, previous, tool),
    }
}

fn hash_once(root: &Value, budget: &mut usize, previous: &str, tool: &McpTool) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget = budget.saturating_sub(1);
    lossless_legacy_match(previous, tool, root)
}

/// `true` when the Cartesian product of set-array `n!` values that individually
/// fit `budget` also fits `budget` (or there are no such arrays).
fn permutable_cartesian_fits(root: &Value, mode: TraverseMode, budget: usize) -> bool {
    let mut lengths = Vec::new();
    collect_set_array_lengths(root, mode, &mut lengths);
    let mut product: usize = 1;
    for len in lengths {
        if !factorial_fits(len, budget) {
            // Oversized arrays are skipped; they do not multiply the search.
            continue;
        }
        let Some(fact) = factorial(len) else {
            return false;
        };
        match product.checked_mul(fact) {
            Some(next) if next <= budget => product = next,
            _ => return false,
        }
    }
    true
}

fn collect_set_array_lengths(value: &Value, mode: TraverseMode, out: &mut Vec<usize>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(mode, TraverseMode::Schema) && key == "dependentRequired" {
                    if let Value::Object(deps) = child {
                        for reqs in deps.values() {
                            if let Value::Array(items) = reqs
                                && items.len() > 1
                            {
                                out.push(items.len());
                            }
                        }
                    }
                    continue;
                }
                let child_mode = match mode {
                    TraverseMode::Literal => TraverseMode::Literal,
                    TraverseMode::SchemaMap => TraverseMode::Schema,
                    TraverseMode::Schema => {
                        if is_schema_valued_map_key(key) {
                            TraverseMode::SchemaMap
                        } else if is_literal_valued_schema_key(key) {
                            TraverseMode::Literal
                        } else if is_order_insensitive_schema_key(key) {
                            if let Value::Array(items) = child
                                && items.len() > 1
                            {
                                out.push(items.len());
                            }
                            if matches!(key.as_str(), "enum" | "required" | "type") {
                                TraverseMode::Literal
                            } else {
                                TraverseMode::Schema
                            }
                        } else {
                            TraverseMode::Schema
                        }
                    }
                };
                collect_set_array_lengths(child, child_mode, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_set_array_lengths(child, mode, out);
            }
        }
        _ => {}
    }
}

fn factorial(n: usize) -> Option<usize> {
    let mut acc: usize = 1;
    for i in 2..=n {
        acc = acc.checked_mul(i)?;
    }
    Some(acc)
}

#[derive(Clone, PartialEq, Eq)]
enum PathStep {
    Key(String),
    Index(usize),
}

fn find_set_array_path(value: &Value, mode: TraverseMode) -> Option<Vec<PathStep>> {
    find_set_array_path_excluding(value, mode, &[])
}

fn permute_at_path(
    root: &mut Value,
    path: &[PathStep],
    assigned: &[Vec<PathStep>],
    budget: &mut usize,
    abort: &mut bool,
    previous: &str,
    tool: &McpTool,
) -> bool {
    if *abort || *budget == 0 {
        *abort = true;
        return false;
    }
    let array = match get_array_mut(root, path) {
        Some(array) => array,
        None => return walk_after_assigned(root, assigned, budget, abort, previous, tool),
    };
    let n = array.len();
    let mut next_assigned = assigned.to_vec();
    next_assigned.push(path.to_vec());
    if !factorial_fits(n, *budget) {
        // Versioned migration: do not partially sample large set arrays.
        return walk_after_assigned(root, &next_assigned, budget, abort, previous, tool);
    }

    // Take ownership of the array for Heap permutation, then restore.
    let mut items = std::mem::take(array);
    let hit = heap_permute(&mut items, n, abort, &mut |perm, abort| {
        if *abort {
            return false;
        }
        if let Some(slot) = get_array_mut(root, path) {
            *slot = perm.to_vec();
        }
        // Continue with every already-assigned path excluded so nested
        // set-arrays cannot rediscover ancestors and stack-overflow.
        walk_after_assigned(root, &next_assigned, budget, abort, previous, tool)
    });
    if let Some(slot) = get_array_mut(root, path) {
        *slot = items;
    }
    hit
}

fn walk_after_assigned(
    root: &mut Value,
    assigned: &[Vec<PathStep>],
    budget: &mut usize,
    abort: &mut bool,
    previous: &str,
    tool: &McpTool,
) -> bool {
    if *abort {
        return false;
    }
    match find_set_array_path_excluding(root, TraverseMode::Schema, assigned) {
        None => {
            if *budget == 0 {
                // Stop sibling/outer permutation generation; do not keep
                // walking the Cartesian product after the hash budget is spent.
                *abort = true;
                return false;
            }
            hash_once(root, budget, previous, tool)
        }
        Some(next) => permute_at_path(root, &next, assigned, budget, abort, previous, tool),
    }
}

fn find_set_array_path_excluding(
    value: &Value,
    mode: TraverseMode,
    exclude: &[Vec<PathStep>],
) -> Option<Vec<PathStep>> {
    find_set_array_path_excluding_inner(value, mode, exclude, &[])
}

fn find_set_array_path_excluding_inner(
    value: &Value,
    mode: TraverseMode,
    exclude: &[Vec<PathStep>],
    prefix: &[PathStep],
) -> Option<Vec<PathStep>> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let mut here = prefix.to_vec();
                here.push(PathStep::Key(key.clone()));
                if matches!(mode, TraverseMode::Schema) && key == "dependentRequired" {
                    if let Some(path) = find_dependent_required_set_array(child, &here, exclude) {
                        return Some(path);
                    }
                    continue;
                }
                let child_mode = match mode {
                    TraverseMode::Literal => TraverseMode::Literal,
                    TraverseMode::SchemaMap => TraverseMode::Schema,
                    TraverseMode::Schema => {
                        if is_schema_valued_map_key(key) {
                            TraverseMode::SchemaMap
                        } else if is_literal_valued_schema_key(key) {
                            TraverseMode::Literal
                        } else if is_order_insensitive_schema_key(key) {
                            if let Value::Array(items) = child {
                                if items.len() > 1 && !path_in_excludes(&here, exclude) {
                                    return Some(here);
                                }
                            }
                            // enum/required/type items are values; combinators are schemas.
                            if matches!(key.as_str(), "enum" | "required" | "type") {
                                TraverseMode::Literal
                            } else {
                                TraverseMode::Schema
                            }
                        } else {
                            TraverseMode::Schema
                        }
                    }
                };
                if let Some(sub) =
                    find_set_array_path_excluding_inner(child, child_mode, exclude, &here)
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
                if let Some(sub) = find_set_array_path_excluding_inner(child, mode, exclude, &here)
                {
                    return Some(sub);
                }
            }
            None
        }
        _ => None,
    }
}

fn find_dependent_required_set_array(
    value: &Value,
    prefix: &[PathStep],
    exclude: &[Vec<PathStep>],
) -> Option<Vec<PathStep>> {
    let Value::Object(map) = value else {
        return None;
    };
    for (prop, reqs) in map {
        let mut here = prefix.to_vec();
        here.push(PathStep::Key(prop.clone()));
        if let Value::Array(items) = reqs
            && items.len() > 1
            && !path_in_excludes(&here, exclude)
        {
            return Some(here);
        }
    }
    None
}

fn path_in_excludes(path: &[PathStep], exclude: &[Vec<PathStep>]) -> bool {
    exclude.iter().any(|ex| path == ex.as_slice())
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
    abort: &mut bool,
    visit: &mut dyn FnMut(&mut [Value], &mut bool) -> bool,
) -> bool {
    if *abort {
        return false;
    }
    if k <= 1 {
        return visit(items, abort);
    }
    if heap_permute(items, k - 1, abort, visit) {
        return true;
    }
    for i in 0..k - 1 {
        if *abort {
            return false;
        }
        if k.is_multiple_of(2) {
            items.swap(i, k - 1);
        } else {
            items.swap(0, k - 1);
        }
        if heap_permute(items, k - 1, abort, visit) {
            return true;
        }
    }
    false
}

fn factorial_fits(n: usize, budget: usize) -> bool {
    match factorial(n) {
        Some(fact) => fact <= budget,
        None => false,
    }
}
