//! Background execution support for sub-agent executor

use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::executor::SubAgentExecutor;
use super::types::ExecutorMessage;
use super::super::types::SubAgentConfig;
use crate::error::SageResult;

impl SubAgentExecutor {
    /// Execute in background, returning channel for progress updates
    pub async fn execute_background(
        &self,
        config: SubAgentConfig,
    ) -> SageResult<(String, mpsc::Receiver<ExecutorMessage>)> {
        let (tx, rx) = mpsc::channel(100);
        let cancel = CancellationToken::new();
        let executor = Arc::new(self.clone());
        let execution_id = uuid::Uuid::new_v4().to_string();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            let result = executor.execute(config, cancel_clone).await;

            let msg = match result {
                Ok(result) => ExecutorMessage::Completed(result),
                Err(e) => ExecutorMessage::Failed(e.to_string()),
            };

            let _ = tx.send(msg).await;
        });

        Ok((execution_id, rx))
    }
}
