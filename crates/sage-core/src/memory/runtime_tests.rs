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

#[tokio::test]
async fn default_relative_storage_path_stays_under_working_dir() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = AgentMemoryConfig {
        enabled: true,
        enabled_set: true,
        storage_path: None,
        ..AgentMemoryConfig::default()
    };

    let runtime = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        runtime.storage_path(),
        dir.path()
            .canonicalize()
            .unwrap()
            .join(".sage/memory/agent-memory.json")
    );
    assert!(
        runtime
            .storage_path()
            .starts_with(dir.path().canonicalize().unwrap())
    );
}

#[tokio::test]
async fn escaping_storage_path_is_rejected() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = enabled_config(PathBuf::from("../escape-memory.json"));

    let result = init_agent_memory_runtime(&config, dir.path()).await;
    let error = match result {
        Ok(_) => panic!("expected escaping storage path to be rejected"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("escapes working directory"));
}

#[tokio::test]
async fn absolute_storage_path_outside_working_dir_is_rejected() {
    clear_runtime_registry_for_tests().await;
    let dir = tempdir().unwrap();
    let config = enabled_config(PathBuf::from("/tmp/sage-evil-memory.json"));

    let result = init_agent_memory_runtime(&config, dir.path()).await;
    let error = match result {
        Ok(_) => panic!("expected absolute escape to be rejected"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("escapes working directory"));
}

#[cfg(unix)]
#[tokio::test]
async fn directory_symlink_storage_path_cannot_write_outside_working_dir() {
    let dir = tempdir().unwrap();
    let working_dir = dir.path().join("project");
    let outside = dir.path().join("outside");
    std::fs::create_dir(&working_dir).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, working_dir.join("link")).unwrap();
    let config = enabled_config(PathBuf::from("link/nested/agent-memory.json"));

    let result = record_agent_outcome(
        &config,
        &working_dir,
        AgentOutcomeRecord::new("test task", AgentOutcomeKind::Success, "test outcome"),
    )
    .await;

    assert!(
        !outside.join("nested/agent-memory.json").exists(),
        "memory was written outside the working directory through a symlink"
    );
    assert!(
        matches!(result, Err(SageError::Config { ref message, .. }) if message.contains("escapes working directory")),
        "expected the storage path to be rejected: {result:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn file_symlink_storage_path_cannot_overwrite_outside_file() {
    let dir = tempdir().unwrap();
    let working_dir = dir.path().join("project");
    std::fs::create_dir(&working_dir).unwrap();
    let outside = dir.path().join("outside.json");
    let original = r#"{"version":1,"memories":[]}"#;
    std::fs::write(&outside, original).unwrap();
    std::os::unix::fs::symlink(&outside, working_dir.join("memory.json")).unwrap();
    let config = enabled_config(PathBuf::from("memory.json"));

    let result = record_agent_outcome(
        &config,
        &working_dir,
        AgentOutcomeRecord::new("test task", AgentOutcomeKind::Success, "test outcome"),
    )
    .await;

    assert_eq!(std::fs::read_to_string(&outside).unwrap(), original);
    assert!(matches!(result, Err(SageError::Config { .. })));
}

#[cfg(unix)]
#[tokio::test]
async fn internal_directory_symlink_storage_uses_canonical_path() {
    let dir = tempdir().unwrap();
    let real = dir.path().join("real");
    let link = dir.path().join("link");
    std::fs::create_dir(&real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let config = enabled_config(PathBuf::from("link/nested/agent-memory.json"));

    let runtime = init_agent_memory_runtime(&config, dir.path())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        runtime.storage_path(),
        real.canonicalize()
            .unwrap()
            .join("nested/agent-memory.json")
    );

    // Retargeting the configured alias must not redirect the initialized storage.
    let outside = tempdir().unwrap();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    runtime
        .memory_manager()
        .remember_lesson("test lesson")
        .await
        .unwrap();

    assert!(real.join("nested/agent-memory.json").is_file());
    assert!(!outside.path().join("nested").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn dangling_symlink_storage_path_is_rejected() {
    let dir = tempdir().unwrap();
    let outside = dir.path().join("missing");
    let working_dir = dir.path().join("project");
    std::fs::create_dir(&working_dir).unwrap();
    std::os::unix::fs::symlink(&outside, working_dir.join("link")).unwrap();
    let config = enabled_config(PathBuf::from("link/nested/agent-memory.json"));

    let result = init_agent_memory_runtime(&config, &working_dir).await;

    assert!(matches!(result, Err(SageError::Config { ref message, .. })
        if message.contains("failed to resolve memory.storage_path")));
    assert!(!outside.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_working_directory_allows_normal_memory_storage() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("project");
    let alias = dir.path().join("alias");
    std::fs::create_dir(&project).unwrap();
    std::os::unix::fs::symlink(&project, &alias).unwrap();
    let config = enabled_config(PathBuf::from(".sage/memory/agent-memory.json"));

    record_agent_outcome(
        &config,
        &alias,
        AgentOutcomeRecord::new("test task", AgentOutcomeKind::Success, "test outcome"),
    )
    .await
    .unwrap();

    assert!(project.join(".sage/memory/agent-memory.json").is_file());
}

#[cfg(unix)]
#[tokio::test]
async fn outcome_rejects_directory_symlink_created_after_recall() {
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let config = enabled_config(PathBuf::from(".sage/memory/agent-memory.json"));
    recall_agent_context(&config, dir.path(), &RecallQuery::for_task("test task", 4))
        .await
        .unwrap();
    assert!(!dir.path().join(".sage").exists());
    std::os::unix::fs::symlink(outside.path(), dir.path().join(".sage")).unwrap();

    let result = record_agent_outcome(
        &config,
        dir.path(),
        AgentOutcomeRecord::new("test task", AgentOutcomeKind::Success, "test outcome"),
    )
    .await;

    assert!(matches!(result, Err(SageError::Config { ref message, .. })
        if message.contains("escapes working directory")));
    assert!(!outside.path().join("memory/agent-memory.json").exists());
}
