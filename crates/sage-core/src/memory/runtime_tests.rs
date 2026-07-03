use super::*;
use std::sync::Arc;
use tempfile::tempdir;

fn enabled_config(path: PathBuf) -> AgentMemoryConfig {
    AgentMemoryConfig {
        enabled: true,
        enabled_set: true,
        storage_path: Some(path),
        max_recall_items: 4,
        max_recall_chars: 500,
        max_stored_outcome_chars: 300,
        ..AgentMemoryConfig::default()
    }
}

#[tokio::test]
async fn disabled_runtime_returns_none() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = AgentMemoryConfig::default();

    let runtime = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap();

    assert!(runtime.is_none());
}

#[tokio::test]
async fn runtime_reuses_same_storage_partition() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = enabled_config(dir.path().join("memory.json"));

    let first = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap()
        .unwrap();
    let second = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first.key(), second.key());
    assert!(Arc::ptr_eq(first.memory_manager(), second.memory_manager()));
    assert!(Arc::ptr_eq(
        first.learning_engine(),
        second.learning_engine()
    ));
}

#[tokio::test]
async fn recall_redacts_and_bounds_memory() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let mut config = enabled_config(dir.path().join("memory.json"));
    config.max_recall_chars = 80;
    let runtime = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap()
        .unwrap();
    runtime
        .memory_manager()
        .remember_lesson("Use cargo check before completion. OPENAI_API_KEY=sk-secret12345")
        .await
        .unwrap();
    runtime
        .memory_manager()
        .remember_lesson("cargo check second lesson should be dropped by the configured bound")
        .await
        .unwrap();

    let recalled = recall_agent_context(
        &config,
        dir.path(),
        &RecallQuery::for_task("cargo check", config.max_recall_items),
    )
    .await
    .unwrap()
    .unwrap();
    let rendered = recalled.render_prompt_section().unwrap();

    assert!(rendered.contains("cargo check"));
    assert!(!rendered.contains("sk-secret12345"));
    assert!(recalled.redaction_report.replacements > 0);
    assert!(recalled.dropped_count > 0);
}

#[tokio::test]
async fn failure_outcome_is_recalled_as_lesson() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = enabled_config(dir.path().join("memory.json"));

    record_agent_outcome(
        &config,
        dir.path(),
        AgentOutcomeRecord::new(
            "Fix failing cargo check",
            AgentOutcomeKind::Failure,
            "cargo check failed because the Config struct missed memory defaults",
        ),
    )
    .await
    .unwrap();

    let recalled = recall_agent_context(
        &config,
        dir.path(),
        &RecallQuery::for_task("cargo check Config memory", config.max_recall_items),
    )
    .await
    .unwrap()
    .unwrap();
    let rendered = recalled.render_prompt_section().unwrap();

    assert!(rendered.contains("Failure lesson"));
    assert!(rendered.contains("outcome"));
    assert!(rendered.contains("memory defaults"));
}
