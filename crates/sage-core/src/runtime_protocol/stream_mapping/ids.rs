pub(super) fn legacy_item_id(kind: &str, sequence: u64) -> String {
    format!("item_legacy_{}_{:03}", kind, sequence.saturating_add(1))
}

pub(super) fn tool_item_id(call_id: &str) -> String {
    if call_id.starts_with("item_") {
        call_id.to_string()
    } else {
        format!("item_{}", id_fragment(call_id))
    }
}

pub(super) fn id_fragment(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}
