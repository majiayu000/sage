//! Core-owned runtime API for agent memory and learning recall.

use crate::config::AgentMemoryConfig;
use crate::diagnostics::{DiagnosticRedactor, RedactionReport};
use crate::error::{SageError, SageResult};
use crate::learning::{
    LearningConfig, Pattern, PatternSource, PatternType, SharedLearningEngine,
    create_learning_engine_with_memory,
};
use crate::memory::{
    Memory, MemoryCategory, MemoryConfig, MemoryMetadata, MemoryQuery, MemorySource,
    SharedMemoryManager, create_memory_manager,
};
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

static RUNTIME_REGISTRY: Lazy<RwLock<HashMap<String, AgentMemoryRuntime>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Query signals used to recall relevant memory for a prompt.
#[derive(Debug, Clone, Default)]
pub struct RecallQuery {
    /// Current task or user request text.
    pub task_text: String,
    /// Recent non-system conversation snippets.
    pub recent_messages: Vec<String>,
    /// Files or paths touched by the current task.
    pub touched_paths: Vec<PathBuf>,
    /// Maximum item count requested by the caller.
    pub limit: usize,
}

impl RecallQuery {
    /// Build a recall query for the current task.
    pub fn for_task(task_text: impl Into<String>, limit: usize) -> Self {
        Self {
            task_text: task_text.into(),
            limit,
            ..Self::default()
        }
    }

    fn search_text(&self) -> String {
        let mut parts = Vec::new();
        if !self.task_text.trim().is_empty() {
            parts.push(self.task_text.trim().to_string());
        }
        parts.extend(
            self.recent_messages
                .iter()
                .filter_map(|message| non_empty_trimmed(message)),
        );
        parts.extend(
            self.touched_paths
                .iter()
                .map(|path| path.to_string_lossy().to_string()),
        );
        parts.join("\n")
    }
}

/// Final execution outcome stored for cross-session learning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentOutcomeKind {
    /// The task completed successfully.
    Success,
    /// The task ended in a failure, cancellation, interrupt, or max-step state.
    Failure,
}

impl AgentOutcomeKind {
    /// Stable metadata value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// Minimal, redacted task outcome record.
#[derive(Debug, Clone)]
pub struct AgentOutcomeRecord {
    /// Task text.
    pub task_text: String,
    /// Outcome kind.
    pub outcome: AgentOutcomeKind,
    /// Short result, error, or lesson summary.
    pub summary: String,
}

impl AgentOutcomeRecord {
    /// Create a new outcome record.
    pub fn new(
        task_text: impl Into<String>,
        outcome: AgentOutcomeKind,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            task_text: task_text.into(),
            outcome,
            summary: summary.into(),
        }
    }
}

/// Recalled, bounded, redacted memory ready for prompt injection.
#[derive(Debug, Clone, Default)]
pub struct RecalledContext {
    /// Rendered memory lines.
    pub memories: Vec<String>,
    /// Rendered learning pattern lines.
    pub learning_patterns: Vec<String>,
    /// Number of items dropped by bounds.
    pub dropped_count: usize,
    /// Redaction report for rendered content.
    pub redaction_report: RedactionReport,
}

impl RecalledContext {
    /// Render a dedicated prompt section, or `None` when no recall exists.
    pub fn render_prompt_section(&self) -> Option<String> {
        if self.memories.is_empty() && self.learning_patterns.is_empty() {
            return None;
        }

        let mut section = String::new();
        if !self.memories.is_empty() {
            section.push_str("Relevant memories:\n");
            for memory in &self.memories {
                section.push_str("- ");
                section.push_str(memory);
                section.push('\n');
            }
        }
        if !self.learning_patterns.is_empty() {
            section.push_str("\nRelevant learning patterns:\n");
            for pattern in &self.learning_patterns {
                section.push_str("- ");
                section.push_str(pattern);
                section.push('\n');
            }
        }
        if self.dropped_count > 0 {
            section.push_str(&format!(
                "\n{} additional recall item(s) were omitted by configured bounds.\n",
                self.dropped_count
            ));
        }

        Some(section.trim_end().to_string())
    }
}

/// Shared memory and learning runtime for a project/storage partition.
#[derive(Clone)]
pub struct AgentMemoryRuntime {
    key: String,
    storage_path: PathBuf,
    memory_manager: SharedMemoryManager,
    learning_engine: SharedLearningEngine,
}

impl AgentMemoryRuntime {
    /// Registry key for this runtime.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Storage path used by this runtime.
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    /// Shared memory manager.
    pub fn memory_manager(&self) -> &SharedMemoryManager {
        &self.memory_manager
    }

    /// Shared learning engine.
    pub fn learning_engine(&self) -> &SharedLearningEngine {
        &self.learning_engine
    }
}

/// Initialize and return the core-owned agent memory runtime.
pub async fn init_agent_memory_runtime(
    config: &AgentMemoryConfig,
    working_dir: &Path,
) -> SageResult<Option<AgentMemoryRuntime>> {
    if !config.enabled {
        return Ok(None);
    }

    let storage_path = resolve_storage_path(config, working_dir);
    let key = runtime_key(&storage_path);

    if let Some(existing) = RUNTIME_REGISTRY.read().await.get(&key).cloned() {
        return Ok(Some(existing));
    }

    let memory_manager = init_global_memory_manager(config, working_dir).await?;
    let learning_engine =
        init_global_learning_engine(config, working_dir, memory_manager.clone()).await?;
    let runtime = AgentMemoryRuntime {
        key: key.clone(),
        storage_path,
        memory_manager,
        learning_engine,
    };

    let mut registry = RUNTIME_REGISTRY.write().await;
    Ok(Some(
        registry
            .entry(key)
            .or_insert_with(|| runtime.clone())
            .clone(),
    ))
}

/// Initialize the global memory manager for a storage partition.
pub async fn init_global_memory_manager(
    config: &AgentMemoryConfig,
    working_dir: &Path,
) -> SageResult<SharedMemoryManager> {
    let storage_path = resolve_storage_path(config, working_dir);
    create_memory_manager(MemoryConfig::with_file_storage(&storage_path).max_memories(10_000))
        .await
        .map_err(|error| {
            SageError::storage(format!(
                "failed to initialize agent memory at {}: {}",
                storage_path.display(),
                error
            ))
        })
}

/// Initialize the global learning engine attached to the same memory manager.
pub async fn init_global_learning_engine(
    config: &AgentMemoryConfig,
    working_dir: &Path,
    memory_manager: SharedMemoryManager,
) -> SageResult<SharedLearningEngine> {
    let storage_path = resolve_storage_path(config, working_dir);
    let learning_config = LearningConfig::with_storage(storage_path);
    let learning_engine = create_learning_engine_with_memory(learning_config, memory_manager);
    learning_engine.load_from_memory().await.map_err(|error| {
        SageError::storage(format!("failed to load agent learning patterns: {}", error))
    })?;
    Ok(learning_engine)
}

/// Recall prompt context for the current task.
pub async fn recall_agent_context(
    config: &AgentMemoryConfig,
    working_dir: &Path,
    query: &RecallQuery,
) -> SageResult<Option<RecalledContext>> {
    let Some(runtime) = init_agent_memory_runtime(config, working_dir).await? else {
        return Ok(None);
    };

    let limit = query.limit.min(config.max_recall_items).max(1);
    let search_text = query.search_text();
    let memory_query = if search_text.trim().is_empty() {
        MemoryQuery::new().limit(limit)
    } else {
        MemoryQuery::new()
            .text(search_text)
            .min_relevance(0.05)
            .limit(limit)
    };

    let memory_scores = runtime
        .memory_manager
        .search(&memory_query)
        .await
        .map_err(|error| SageError::storage(format!("failed to recall memories: {}", error)))?;
    let memories = memory_scores
        .into_iter()
        .map(|score| {
            format!(
                "[{}] {}",
                score.memory.memory_type.name(),
                score.memory.content
            )
        })
        .collect::<Vec<_>>();

    let learning_patterns = runtime.learning_engine.get_patterns_for_prompt(limit).await;
    Ok(Some(bound_and_redact_recall(
        memories,
        learning_patterns,
        config.max_recall_chars,
    )))
}

/// Store an execution outcome as both memory and learning signal.
pub async fn record_agent_outcome(
    config: &AgentMemoryConfig,
    working_dir: &Path,
    record: AgentOutcomeRecord,
) -> SageResult<Option<()>> {
    let Some(runtime) = init_agent_memory_runtime(config, working_dir).await? else {
        return Ok(None);
    };

    let redactor = DiagnosticRedactor::new();
    let task_text = truncate_chars(
        &redactor.redact_text(&record.task_text).value,
        config.max_stored_outcome_chars,
    );
    let summary = truncate_chars(
        &redactor.redact_text(&record.summary).value,
        config.max_stored_outcome_chars,
    );
    let outcome = record.outcome.as_str();
    let content = match record.outcome {
        AgentOutcomeKind::Success => {
            format!("Successful task outcome (outcome=success): {task_text}\nResult: {summary}")
        }
        AgentOutcomeKind::Failure => {
            format!("Failure lesson (outcome=failure): {task_text}\nLesson: {summary}")
        }
    };

    let memory = Memory::lesson(content)
        .with_category(MemoryCategory::Project)
        .with_metadata(
            MemoryMetadata::with_source(MemorySource::Agent)
                .with_confidence(if record.outcome == AgentOutcomeKind::Failure {
                    0.8
                } else {
                    0.7
                })
                .with_tags(["agent_outcome", outcome]),
        )
        .with_data(json!({
            "outcome": outcome,
            "task": task_text,
            "summary": summary,
        }));

    runtime
        .memory_manager
        .store(memory)
        .await
        .map_err(|error| SageError::storage(format!("failed to store agent outcome: {}", error)))?;

    let mut pattern = outcome_pattern(record.outcome, &task_text, &summary);
    pattern
        .metadata
        .insert("outcome".to_string(), outcome.to_string());
    pattern
        .metadata
        .insert("task".to_string(), truncate_chars(&task_text, 240));
    runtime
        .learning_engine
        .learn(pattern)
        .await
        .map_err(|error| {
            SageError::storage(format!("failed to record learning outcome: {}", error))
        })?;

    Ok(Some(()))
}

fn outcome_pattern(outcome: AgentOutcomeKind, task_text: &str, summary: &str) -> Pattern {
    let description = match outcome {
        AgentOutcomeKind::Success => format!(
            "Successful task pattern: {}",
            truncate_chars(task_text, 120)
        ),
        AgentOutcomeKind::Failure => format!("Failure lesson: {}", truncate_chars(task_text, 120)),
    };
    let rule = match outcome {
        AgentOutcomeKind::Success => {
            format!("For similar tasks, reuse the successful outcome: {summary}")
        }
        AgentOutcomeKind::Failure => {
            format!("For similar tasks, avoid repeating this failure: {summary}")
        }
    };
    Pattern::new(
        PatternType::ProjectSpecific,
        description,
        rule,
        PatternSource::BehaviorPattern,
    )
    .with_confidence(if outcome == AgentOutcomeKind::Failure {
        0.8
    } else {
        0.7
    })
    .with_context(format!("outcome:{}", outcome.as_str()))
}

fn bound_and_redact_recall(
    memories: Vec<String>,
    learning_patterns: Vec<String>,
    max_chars: usize,
) -> RecalledContext {
    let redactor = DiagnosticRedactor::new();
    let mut report = RedactionReport::default();
    let mut budget = max_chars;
    let mut dropped_count = 0;
    let memories = bound_items(
        memories,
        &redactor,
        &mut report,
        &mut budget,
        &mut dropped_count,
    );
    let learning_patterns = bound_items(
        learning_patterns,
        &redactor,
        &mut report,
        &mut budget,
        &mut dropped_count,
    );

    RecalledContext {
        memories,
        learning_patterns,
        dropped_count,
        redaction_report: report,
    }
}

fn bound_items(
    items: Vec<String>,
    redactor: &DiagnosticRedactor,
    report: &mut RedactionReport,
    budget: &mut usize,
    dropped_count: &mut usize,
) -> Vec<String> {
    let mut kept = Vec::new();
    for item in items {
        let redacted = redactor.redact_text(&item);
        report.merge(redacted.report);
        let len = redacted.value.chars().count();
        if len <= *budget {
            kept.push(redacted.value);
            *budget = budget.saturating_sub(len);
        } else {
            *dropped_count += 1;
        }
    }
    kept
}

fn resolve_storage_path(config: &AgentMemoryConfig, working_dir: &Path) -> PathBuf {
    let configured = config
        .storage_path
        .clone()
        .unwrap_or_else(|| PathBuf::from(".sage/memory/agent-memory.json"));
    if configured.is_absolute() {
        configured
    } else {
        working_dir.join(configured)
    }
}

fn runtime_key(storage_path: &Path) -> String {
    storage_path.to_string_lossy().to_string()
}

fn non_empty_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
pub(crate) async fn clear_runtime_registry_for_tests() {
    RUNTIME_REGISTRY.write().await.clear();
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
