//! Fallback chain manager

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::state::ModelState;
use super::types::{FallbackEvent, FallbackReason, ModelConfig};

/// Model fallback chain manager
#[derive(Debug)]
pub struct FallbackChain {
    /// Models in the chain
    pub(super) models: Arc<RwLock<Vec<ModelState>>>,
    /// Current model index
    pub(super) current_index: Arc<RwLock<usize>>,
    /// Fallback history
    pub(super) history: Arc<RwLock<Vec<FallbackEvent>>>,
    /// Maximum history entries
    pub(super) max_history: usize,
}

impl FallbackChain {
    /// Create a new fallback chain
    pub fn new() -> Self {
        Self {
            models: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history: 100,
        }
    }

    /// Add a model to the chain
    pub async fn add_model(&self, config: ModelConfig) {
        let mut models = self.models.write().await;
        let state = ModelState::new(config);

        // Insert in priority order
        let pos = models
            .iter()
            .position(|m| m.config.priority > state.config.priority)
            .unwrap_or(models.len());

        models.insert(pos, state);
    }

    /// Get the current active model
    pub async fn current_model(&self) -> Option<String> {
        let models = self.models.read().await;
        let index = *self.current_index.read().await;

        models.get(index).map(|m| m.config.model_id.clone())
    }

    /// Get next available model
    pub async fn next_available(&self, context_size: Option<usize>) -> Option<String> {
        let models = self.models.read().await;

        for (i, state) in models.iter().enumerate() {
            if !state.is_available() {
                continue;
            }

            // Check context size if provided
            if let Some(size) = context_size {
                if size > state.config.max_context {
                    continue;
                }
            }

            // Update current index
            let mut current = self.current_index.write().await;
            *current = i;

            return Some(state.config.model_id.clone());
        }

        None
    }

    /// Record a successful request
    pub async fn record_success(&self, model_id: &str) {
        let mut models = self.models.write().await;

        if let Some(state) = models.iter_mut().find(|m| m.config.model_id == model_id) {
            state.record_success();
        }
    }

    /// Record a failed request and potentially trigger fallback
    pub async fn record_failure(&self, model_id: &str, reason: FallbackReason) -> Option<String> {
        let mut models = self.models.write().await;

        // Find and update the failed model
        let failed_index = models.iter().position(|m| m.config.model_id == model_id);

        if let Some(index) = failed_index {
            models[index].record_failure();

            // Record fallback event
            let event = FallbackEvent {
                from_model: model_id.to_string(),
                to_model: None,
                reason: reason.clone(),
                timestamp: Instant::now(),
            };
            self.add_history_event(event).await;

            // Check if we should fallback
            if models[index].failure_count >= models[index].config.max_retries {
                // Find next available model
                drop(models);
                return self.next_available(None).await;
            }
        }

        None
    }

    /// Force fallback to next model
    pub async fn force_fallback(&self, reason: FallbackReason) -> Option<String> {
        let current = self.current_model().await?;

        let current_index = self.current_index.write().await;
        let models = self.models.read().await;

        // Find next available model after current
        for i in (*current_index + 1)..models.len() {
            if models[i].is_available() {
                let new_model = models[i].config.model_id.clone();

                // Record event
                let event = FallbackEvent {
                    from_model: current.clone(),
                    to_model: Some(new_model.clone()),
                    reason,
                    timestamp: Instant::now(),
                };
                drop(models);
                drop(current_index);
                self.add_history_event(event).await;

                let mut current_index = self.current_index.write().await;
                *current_index = i;
                return Some(new_model);
            }
        }

        None
    }
}

impl Default for FallbackChain {
    fn default() -> Self {
        Self::new()
    }
}
