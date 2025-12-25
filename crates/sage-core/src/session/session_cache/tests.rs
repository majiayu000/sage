//! Tests for session cache

#[cfg(test)]
mod tests {
    use super::super::manager::SessionCache;
    use super::super::types::{RecentSession, SessionCacheConfig};
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_session_cache_creation() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache.json")),
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();
        assert!(!cache.is_dirty().await);
    }

    #[tokio::test]
    async fn test_tool_trust_settings() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache2.json")),
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();

        // Initially no trust settings
        let trust = cache.get_tool_trust().await;
        assert!(trust.always_allowed.is_empty());

        // Allow a tool
        cache.update_tool_trust(|t| t.allow("bash")).await.unwrap();

        let trust = cache.get_tool_trust().await;
        assert!(trust.always_allowed.contains("bash"));
        assert!(cache.is_dirty().await);
    }

    #[tokio::test]
    async fn test_recent_sessions() {
        let config = SessionCacheConfig {
            enabled: true,
            global_cache_path: Some(PathBuf::from("/tmp/test_cache3.json")),
            max_recent_sessions: 5,
            ..Default::default()
        };

        let cache = SessionCache::new(config).await.unwrap();

        // Add sessions
        for i in 0..7 {
            cache
                .add_recent_session(RecentSession {
                    id: format!("session-{}", i),
                    name: Some(format!("Session {}", i)),
                    working_directory: "/tmp".to_string(),
                    model: Some("claude-3.5-sonnet".to_string()),
                    last_active: Utc::now(),
                    message_count: i * 10,
                    description: None,
                })
                .await
                .unwrap();
        }

        let sessions = cache.get_recent_sessions().await;
        assert_eq!(sessions.len(), 5); // Max is 5
        assert_eq!(sessions[0].id, "session-6"); // Most recent first
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // Create and populate cache
        {
            let config = SessionCacheConfig {
                enabled: true,
                global_cache_path: Some(cache_path.clone()),
                ..Default::default()
            };

            let cache = SessionCache::new(config).await.unwrap();

            cache
                .update_tool_trust(|t| {
                    t.allow("read");
                    t.deny("rm");
                })
                .await
                .unwrap();

            cache
                .update_preferences(|p| {
                    p.default_model = Some("claude-3.5-sonnet".to_string());
                })
                .await
                .unwrap();

            cache.save().await.unwrap();
        }

        // Load and verify
        {
            let config = SessionCacheConfig {
                enabled: true,
                global_cache_path: Some(cache_path),
                ..Default::default()
            };

            let cache = SessionCache::new(config).await.unwrap();

            let trust = cache.get_tool_trust().await;
            assert!(trust.always_allowed.contains("read"));
            assert!(trust.always_denied.contains("rm"));

            let prefs = cache.get_preferences().await;
            assert_eq!(prefs.default_model, Some("claude-3.5-sonnet".to_string()));
        }
    }
}
