use super::session::*;
use crate::types::TokenUsage;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_session_recorder() {
    let temp_dir = TempDir::new().unwrap();

    // Create a fake home directory
    let fake_home = temp_dir.path().join("home");
    std::fs::create_dir_all(&fake_home).unwrap();
    // SAFETY: This test runs in isolation and does not access HOME concurrently
    unsafe {
        std::env::set_var("HOME", &fake_home);
    }

    let working_dir = temp_dir.path().join("project");
    std::fs::create_dir_all(&working_dir).unwrap();

    let mut recorder = SessionRecorder::new(&working_dir).unwrap();

    // Start session
    recorder
        .record_session_start("Test task", "glm", "glm-4.7")
        .await
        .unwrap();

    // Record user message
    recorder
        .record_user_message(serde_json::json!({"role": "user", "content": "Hello"}))
        .await
        .unwrap();

    // Record LLM response
    recorder
        .record_llm_response(
            "Hi there!",
            "glm-4.7",
            Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: None,
                cache_write_tokens: None,
                cost_estimate: None,
            }),
            None,
        )
        .await
        .unwrap();

    // Record session end
    recorder
        .record_session_end(true, Some("Done".to_string()))
        .await
        .unwrap();

    // Verify file exists
    assert!(recorder.file_path().exists());

    // Load and verify entries
    let entries = SessionRecorder::load_entries(recorder.file_path())
        .await
        .unwrap();

    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].entry_type(), "session_start");
    assert_eq!(entries[1].entry_type(), "user");
    assert_eq!(entries[2].entry_type(), "llm_response");
    assert_eq!(entries[3].entry_type(), "session_end");
}

#[test]
fn test_escape_path() {
    let path = Path::new("/Users/test/code/project");
    let escaped = SessionRecorder::escape_path(path);
    assert_eq!(escaped, "Users-test-code-project");
}
