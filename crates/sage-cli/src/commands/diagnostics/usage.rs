//! Usage data extraction from session files

use super::types::UsageData;

/// Extract usage data from file content (JSONL or single JSON)
pub fn extract_usage_from_content(content: &str) -> Option<UsageData> {
    // Try to parse as JSON and extract usage information
    // This handles both single JSON objects and JSONL format

    let mut total = UsageData::default();
    let mut found_any = false;

    // Try each line as potentially valid JSON
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(usage) = extract_usage_from_json(&value) {
                total.prompt_tokens += usage.prompt_tokens;
                total.completion_tokens += usage.completion_tokens;
                total.cache_read_tokens += usage.cache_read_tokens;
                total.cache_created_tokens += usage.cache_created_tokens;
                found_any = true;
            }
        }
    }

    // Also try the entire content as a single JSON object
    if !found_any {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(usage) = extract_usage_from_json(&value) {
                return Some(usage);
            }
        }
    }

    if found_any { Some(total) } else { None }
}

/// Extract usage data from a JSON value
pub fn extract_usage_from_json(value: &serde_json::Value) -> Option<UsageData> {
    // Look for usage data in common locations
    let usage = value.get("usage").or_else(|| value.get("token_usage"))?;

    Some(UsageData {
        prompt_tokens: usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read_tokens: usage
            .get("cache_read_input_tokens")
            .or_else(|| usage.get("cache_read_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_created_tokens: usage
            .get("cache_creation_input_tokens")
            .or_else(|| usage.get("cache_created_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}
