use super::{JsonlSessionStorage, line_preview};
use crate::session::types::unified::SessionMessage;
use crate::session::types::{FileHistorySnapshot, SessionContext};
use tempfile::TempDir;

#[test]
fn preview_preserves_utf8_and_byte_limit() {
    for line in [
        String::new(),
        "short".to_string(),
        "a".repeat(60),
        format!("{}中", "a".repeat(49)),
        format!("{}💥", "a".repeat(48)),
        "中".repeat(30),
        "💥".repeat(20),
        format!("{}中", "a".repeat(47)),
    ] {
        let preview = line_preview(&line);
        assert!(line.starts_with(preview));
        assert!(preview.len() <= 50);
        if preview.len() < line.len() {
            let next = line[preview.len()..].chars().next().unwrap();
            assert!(preview.len() + next.len_utf8() > 50);
        }
    }
}

// A per-thread subscriber keeps WARN enabled, which forces evaluation of the
// diagnostic excerpt that used to panic. Tokio's current-thread test runtime
// keeps the reader future and subscriber on the same thread across awaits.
async fn assert_recovers_after_malformed_unicode(reader: &str) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::sink)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    assert!(tracing::enabled!(tracing::Level::WARN));

    let temp = TempDir::new().unwrap();
    let storage = JsonlSessionStorage::new(temp.path());
    let id = "synthetic-session".to_string();
    let dir = temp.path().join(&id);
    tokio::fs::create_dir(&dir).await.unwrap();
    let message = SessionMessage::user(
        "valid message after corrupt records",
        &id,
        SessionContext::new(temp.path().to_path_buf()),
    );
    let snapshot = FileHistorySnapshot::new(&message.uuid);
    let malformed = format!("\n{}中\n{}💥\nnot json\n\n", "a".repeat(49), "a".repeat(48));
    tokio::fs::write(
        dir.join("messages.jsonl"),
        format!("{malformed}{}\n", serde_json::to_string(&message).unwrap()),
    )
    .await
    .unwrap();
    tokio::fs::write(
        dir.join("snapshots.jsonl"),
        format!("{malformed}{}\n", serde_json::to_string(&snapshot).unwrap()),
    )
    .await
    .unwrap();

    match reader {
        "load_messages" => {
            let messages = storage.load_messages(&id).await.unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].uuid, message.uuid);
        }
        "load_snapshots" => {
            let snapshots = storage.load_snapshots(&id).await.unwrap();
            assert_eq!(snapshots.len(), 1);
            assert_eq!(snapshots[0].message_id, message.uuid);
        }
        "get_message" => {
            let loaded = storage
                .get_message(&id, &message.uuid)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(loaded.uuid, message.uuid);
        }
        "get_messages_until" => {
            let messages = storage
                .get_messages_until(&id, &message.uuid)
                .await
                .unwrap();
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].uuid, message.uuid);
        }
        _ => panic!("unknown reader"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn load_messages_recovers_after_malformed_unicode() {
    assert_recovers_after_malformed_unicode("load_messages").await;
}

#[tokio::test(flavor = "current_thread")]
async fn load_snapshots_recovers_after_malformed_unicode() {
    assert_recovers_after_malformed_unicode("load_snapshots").await;
}

#[tokio::test(flavor = "current_thread")]
async fn get_message_recovers_after_malformed_unicode() {
    assert_recovers_after_malformed_unicode("get_message").await;
}

#[tokio::test(flavor = "current_thread")]
async fn get_messages_until_recovers_after_malformed_unicode() {
    assert_recovers_after_malformed_unicode("get_messages_until").await;
}
