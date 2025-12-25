//! Cache eviction and cleanup logic

use super::cache::McpCache;
use std::sync::atomic::Ordering;
use tracing::debug;

impl McpCache {
    /// Remove expired entries from all caches
    pub fn cleanup_expired(&self) {
        let mut evicted = 0;

        // Cleanup tools
        self.tools_map().retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup resources
        self.resources_map().retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup prompts
        self.prompts_map().retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        // Cleanup resource content
        self.resource_content_map().retain(|_, entry| {
            if entry.is_expired() {
                evicted += 1;
                false
            } else {
                true
            }
        });

        if evicted > 0 {
            self.stats_mut()
                .evictions
                .fetch_add(evicted, Ordering::Relaxed);
            debug!("Cleaned up {} expired cache entries", evicted);
        }
    }
}
