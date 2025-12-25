//! Panic recovery utilities

use super::super::RecoveryError;
use std::future::Future;
use std::panic::AssertUnwindSafe;

/// Execute a function with panic recovery
pub async fn catch_panic<T, F, Fut>(task: F) -> Result<T, RecoveryError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    match std::panic::catch_unwind(AssertUnwindSafe(|| task())) {
        Ok(future) => Ok(future.await),
        Err(panic) => {
            let message = if let Some(s) = panic.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            Err(RecoveryError::TaskPanic { message })
        }
    }
}
