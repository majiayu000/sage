//! Session summary generation functionality.

use crate::session::SummaryGenerator;

use super::UnifiedExecutor;

impl UnifiedExecutor {
    /// Check and update session summary if needed
    pub(super) async fn maybe_update_summary(
        &mut self,
        storage: &crate::session::JsonlSessionStorage,
        session_id: &str,
    ) {
        // Load messages to check if summary update is needed
        let last_count = self.session_manager.last_summary_msg_count();
        if let Ok(messages) = storage.load_messages(&session_id.to_string()).await {
            if SummaryGenerator::should_update_summary(&messages, last_count) {
                // Generate new summary
                if let Some(summary) = SummaryGenerator::generate_simple(&messages) {
                    // Update metadata with new summary
                    if let Ok(Some(mut metadata)) =
                        storage.load_metadata(&session_id.to_string()).await
                    {
                        metadata.set_summary(&summary);
                        if storage
                            .save_metadata(&session_id.to_string(), &metadata)
                            .await
                            .is_ok()
                        {
                            self.session_manager
                                .set_last_summary_msg_count(messages.len());
                            tracing::debug!("Updated session summary: {}", summary);
                        }
                    }
                }
            }
        }
    }
}
