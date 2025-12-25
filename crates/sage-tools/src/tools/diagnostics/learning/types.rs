//! Type definitions and global state for learning functionality

use sage_core::learning::{LearningConfig, SharedLearningEngine, create_learning_engine};
use tokio::sync::OnceCell;

/// Global learning engine instance
static GLOBAL_LEARNING_ENGINE: OnceCell<SharedLearningEngine> = OnceCell::const_new();

/// Initialize the global learning engine
pub async fn init_global_learning_engine(config: Option<LearningConfig>) -> Result<(), String> {
    let config = config.unwrap_or_default();
    let engine = create_learning_engine(config);

    GLOBAL_LEARNING_ENGINE
        .set(engine)
        .map_err(|_| "Learning engine already initialized".to_string())
}

/// Get the global learning engine
pub fn get_global_learning_engine() -> Option<SharedLearningEngine> {
    GLOBAL_LEARNING_ENGINE.get().cloned()
}

/// Ensure learning engine is initialized (creates default if not)
pub(super) async fn ensure_learning_engine() -> SharedLearningEngine {
    if let Some(engine) = GLOBAL_LEARNING_ENGINE.get() {
        return engine.clone();
    }

    // Initialize with default config
    let engine = create_learning_engine(LearningConfig::default());

    // Try to set, if fails (race condition), just get the existing one
    let _ = GLOBAL_LEARNING_ENGINE.set(engine.clone());
    GLOBAL_LEARNING_ENGINE.get().cloned().unwrap_or(engine)
}

/// Get patterns for system prompt injection
pub async fn get_learning_patterns_for_context(limit: usize) -> Vec<String> {
    let engine = match GLOBAL_LEARNING_ENGINE.get() {
        Some(e) => e,
        None => return Vec::new(),
    };

    engine.get_patterns_for_prompt(limit).await
}
