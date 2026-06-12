use super::*;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_session_cache() {
    let cache = PermissionCache::new();

    cache.set("test_key".to_string(), true).await;
    assert_eq!(cache.get("test_key").await, Some(true));

    cache.set("test_key".to_string(), false).await;
    assert_eq!(cache.get("test_key").await, Some(false));
}

#[tokio::test]
async fn test_persistent_cache() {
    let temp_dir = TempDir::new().unwrap();
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir).unwrap();

    let cache = PermissionCache::with_persistence(temp_dir.path());

    cache
        .set_with_persistence("Bash(npm *)".to_string(), true, true)
        .await
        .unwrap();

    let settings_path = sage_dir.join("settings.local.json");
    assert!(settings_path.exists());

    let content = fs::read_to_string(&settings_path).unwrap();
    assert!(content.contains("Bash(npm *)"));
}

#[tokio::test]
async fn test_cache_key_bash() {
    let mut arguments = HashMap::new();
    arguments.insert(
        "command".to_string(),
        serde_json::Value::String("npm install lodash".to_string()),
    );

    let call = ToolCall {
        id: "test-id".to_string(),
        name: "bash".to_string(),
        arguments,
        call_id: None,
    };

    let key = PermissionCache::cache_key("Bash", &call);
    assert_eq!(key, "Bash(npm *)");
}

#[tokio::test]
async fn test_cache_key_read() {
    let mut arguments = HashMap::new();
    arguments.insert(
        "file_path".to_string(),
        serde_json::Value::String("/src/main.rs".to_string()),
    );

    let call = ToolCall {
        id: "test-id".to_string(),
        name: "read".to_string(),
        arguments,
        call_id: None,
    };

    let key = PermissionCache::cache_key("Read", &call);
    assert_eq!(key, "Read(src/**)");
}

#[tokio::test]
async fn test_pattern_matches() {
    assert!(PermissionCache::pattern_matches(
        "Bash(npm *)",
        "Bash(npm *)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Bash(npm *)",
        "Bash(npm install)"
    ));
    assert!(!PermissionCache::pattern_matches(
        "Bash(npm *)",
        "Bash(yarn install)"
    ));
    assert!(PermissionCache::pattern_matches(
        "*",
        "Bash(rm -rf /tmp/foo)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Bash*",
        "Bash(rm -rf /tmp/foo)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Bash",
        "Bash(rm -rf /tmp/foo)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Read(src/**)",
        "Read(src/main.rs)"
    ));
    assert!(!PermissionCache::pattern_matches(
        "Read(src/**)",
        "Read(Src/main.rs)"
    ));
}

#[tokio::test]
async fn test_path_permission_star_does_not_cross_separator() {
    assert!(PermissionCache::pattern_matches(
        "Read(src/*.rs)",
        "Read(src/main.rs)"
    ));
    assert!(!PermissionCache::pattern_matches(
        "Read(src/*.rs)",
        "Read(src/nested/main.rs)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Read(src/**)",
        "Read(src/nested/main.rs)"
    ));
}

#[tokio::test]
async fn test_path_permission_recursive_glob_matches_direct_children() {
    assert!(PermissionCache::pattern_matches(
        "Read(src/**/*)",
        "Read(src/main.rs)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Read(src/**/*)",
        "Read(src/nested/main.rs)"
    ));
}

#[tokio::test]
async fn test_path_permission_glob_metacharacters_match_paths() {
    assert!(PermissionCache::pattern_matches(
        "Glob(src/test_?.py)",
        "Glob(src/test_a.py)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Glob(src/*.[jt]s)",
        "Glob(src/app.ts)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Glob(src/*.{js,ts})",
        "Glob(src/app.js)"
    ));
    assert!(PermissionCache::pattern_matches(
        "Glob(src/*.{js,ts})",
        "Glob(src/app.ts)"
    ));
}

#[tokio::test]
async fn test_non_path_permission_star_keeps_matching_slashes() {
    assert!(PermissionCache::pattern_matches(
        "Bash(rm -rf *)",
        "Bash(rm -rf /tmp/foo)"
    ));
}

#[tokio::test]
async fn test_webfetch_permission_star_matches_url_slashes() {
    assert!(PermissionCache::pattern_matches(
        "WebFetch(https://internal.example/*)",
        "WebFetch(https://internal.example/private/secret)"
    ));
}

#[tokio::test]
async fn test_get_with_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let sage_dir = temp_dir.path().join(".sage");
    fs::create_dir(&sage_dir).unwrap();

    let settings_content = r#"{
        "permissions": {
            "allow": ["Read(src/**)"],
            "deny": ["Bash(rm *)"]
        }
    }"#;
    fs::write(sage_dir.join("settings.local.json"), settings_content).unwrap();

    let cache = PermissionCache::with_persistence(temp_dir.path());

    assert_eq!(cache.get_with_persistence("Read(src/**)").await, Some(true));
    assert_eq!(cache.get_with_persistence("Bash(rm *)").await, Some(false));
    assert_eq!(cache.get_with_persistence("Unknown").await, None);
}
