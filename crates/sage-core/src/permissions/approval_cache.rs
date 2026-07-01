use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalCacheDecision {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalCacheLookup {
    Hit(ApprovalCacheDecision),
    Expired,
    Miss,
}

#[derive(Debug, Clone)]
struct ApprovalCacheEntry {
    decision: ApprovalCacheDecision,
    expires_at: Option<Instant>,
}

#[derive(Debug, Default, Clone)]
pub struct ApprovalCache {
    entries: HashMap<String, ApprovalCacheEntry>,
}

impl ApprovalCache {
    pub fn insert(
        &mut self,
        key: impl Into<String>,
        decision: ApprovalCacheDecision,
        ttl: Option<Duration>,
        now: Instant,
    ) {
        let expires_at = ttl.map(|ttl| now + ttl);
        self.entries.insert(
            key.into(),
            ApprovalCacheEntry {
                decision,
                expires_at,
            },
        );
    }

    pub fn lookup(&mut self, key: &str, now: Instant) -> ApprovalCacheLookup {
        let Some(entry) = self.entries.get(key) else {
            return ApprovalCacheLookup::Miss;
        };

        if entry.expires_at.is_some_and(|expires_at| now >= expires_at) {
            self.entries.remove(key);
            return ApprovalCacheLookup::Expired;
        }

        ApprovalCacheLookup::Hit(entry.decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_returns_allow_and_deny_hits() {
        let now = Instant::now();
        let mut cache = ApprovalCache::default();

        cache.insert(
            "Bash(cargo test)",
            ApprovalCacheDecision::Allow,
            Some(Duration::from_secs(60)),
            now,
        );
        cache.insert(
            "Bash(rm *)",
            ApprovalCacheDecision::Deny,
            Some(Duration::from_secs(60)),
            now,
        );

        assert_eq!(
            cache.lookup("Bash(cargo test)", now + Duration::from_secs(1)),
            ApprovalCacheLookup::Hit(ApprovalCacheDecision::Allow)
        );
        assert_eq!(
            cache.lookup("Bash(rm *)", now + Duration::from_secs(1)),
            ApprovalCacheLookup::Hit(ApprovalCacheDecision::Deny)
        );
    }

    #[test]
    fn cache_expires_entries() {
        let now = Instant::now();
        let mut cache = ApprovalCache::default();

        cache.insert(
            "Write(src/tmp.txt)",
            ApprovalCacheDecision::Allow,
            Some(Duration::from_millis(10)),
            now,
        );

        assert_eq!(
            cache.lookup("Write(src/tmp.txt)", now + Duration::from_millis(11)),
            ApprovalCacheLookup::Expired
        );
        assert_eq!(
            cache.lookup("Write(src/tmp.txt)", now + Duration::from_millis(12)),
            ApprovalCacheLookup::Miss
        );
    }
}
