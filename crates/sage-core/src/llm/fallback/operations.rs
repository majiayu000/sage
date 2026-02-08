//! Fallback chain operations (health checks, stats, resets)

use super::manager::FallbackChain;
use super::types::{FallbackEvent, ModelStats};

impl FallbackChain {
    /// Reset a model to healthy state
    pub async fn reset_model(&self, model_id: &str) {
        let mut models = self.models.write().await;

        if let Some(state) = models.iter_mut().find(|m| m.config.model_id == model_id) {
            state.failure_count = 0;
            state.last_failure = None;
            state.config.healthy = true;
        }
    }

    /// Reset all models
    pub async fn reset_all(&self) {
        let mut models = self.models.write().await;

        for state in models.iter_mut() {
            state.failure_count = 0;
            state.last_failure = None;
            state.config.healthy = true;
        }

        *self.current_index.write().await = 0;
    }

    /// Get model statistics
    pub async fn get_stats(&self) -> Vec<ModelStats> {
        let models = self.models.read().await;

        models
            .iter()
            .map(|m| ModelStats {
                model_id: m.config.model_id.clone(),
                provider: m.config.provider.clone(),
                available: m.is_available(),
                total_requests: m.total_requests,
                successful_requests: m.successful_requests,
                success_rate: m.success_rate(),
                failure_count: m.failure_count,
            })
            .collect()
    }

    /// Get fallback history
    pub async fn get_history(&self) -> Vec<FallbackEvent> {
        self.history.read().await.iter().cloned().collect()
    }

    /// Add history event
    pub(super) async fn add_history_event(&self, event: FallbackEvent) {
        let mut history = self.history.write().await;
        history.push_back(event);

        // Trim to max size
        while history.len() > self.max_history {
            history.pop_front();
        }
    }

    /// Get model count
    pub async fn model_count(&self) -> usize {
        self.models.read().await.len()
    }

    /// Check if chain is empty
    pub async fn is_empty(&self) -> bool {
        self.models.read().await.is_empty()
    }

    /// List all models
    pub async fn list_models(&self) -> Vec<String> {
        self.models
            .read()
            .await
            .iter()
            .map(|m| m.config.model_id.clone())
            .collect()
    }
}
