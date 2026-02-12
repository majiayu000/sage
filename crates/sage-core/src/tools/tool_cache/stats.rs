//! Cache statistics

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct ToolCacheStats {
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Insertions
    pub inserts: u64,
    /// Expirations
    pub expirations: u64,
    /// Manual clears
    pub clears: u64,
}

impl ToolCacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Format stats as summary string
    pub fn summary(&self) -> String {
        format!(
            "hits: {}, misses: {}, hit rate: {:.1}%, inserts: {}",
            self.hits,
            self.misses,
            self.hit_rate() * 100.0,
            self.inserts
        )
    }
}
