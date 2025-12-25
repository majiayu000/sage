//! Memory context analysis and retrieval for agent context injection

use sage_core::memory::MemoryType;

use super::types::get_global_memory_manager;

/// Get memories for context injection (used by system prompt builder)
pub async fn get_memories_for_context(limit: usize) -> Vec<String> {
    let manager = match get_global_memory_manager() {
        Some(m) => m,
        None => return Vec::new(),
    };

    // Get pinned memories first
    let mut memories = Vec::new();

    if let Ok(pinned) = manager.pinned().await {
        for mem in pinned.iter().take(limit) {
            memories.push(format!("[{}] {}", mem.memory_type.name(), mem.content));
        }
    }

    // If we have room, add recent non-pinned memories
    if memories.len() < limit {
        let remaining = limit - memories.len();
        for mem_type in [MemoryType::Fact, MemoryType::Preference, MemoryType::Lesson] {
            if memories.len() >= limit {
                break;
            }
            if let Ok(mems) = manager.find_by_type(mem_type).await {
                for mem in mems.iter().filter(|m| !m.metadata.pinned).take(remaining) {
                    memories.push(format!("[{}] {}", mem.memory_type.name(), mem.content));
                    if memories.len() >= limit {
                        break;
                    }
                }
            }
        }
    }

    memories
}
