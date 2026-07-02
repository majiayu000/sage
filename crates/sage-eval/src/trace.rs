//! Trajectory helpers for eval runs.

use anyhow::Result;
use sage_core::trajectory::SessionEntry;
use std::path::Path;

pub async fn write_jsonl(path: &Path, entries: &[SessionEntry]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut content = String::new();
    for entry in entries {
        content.push_str(&serde_json::to_string(entry)?);
        content.push('\n');
    }
    tokio::fs::write(path, content).await?;
    Ok(())
}

pub async fn read_jsonl(path: &Path) -> Result<Vec<SessionEntry>> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut entries = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        entries.push(serde_json::from_str(line)?);
    }
    Ok(entries)
}
