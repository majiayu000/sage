//! Callback hook implementation

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use super::hook_input::HookInput;
use super::hook_output::HookOutput;
use super::hook_types::default_timeout;

/// Rust callback hook
pub struct CallbackHook {
    pub callback: Arc<dyn Fn(HookInput) -> HookOutput + Send + Sync>,
    pub timeout: Duration,
}

impl CallbackHook {
    /// Create a new callback hook
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(HookInput) -> HookOutput + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
            timeout: Duration::from_secs(default_timeout()),
        }
    }

    /// Set the timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Clone for CallbackHook {
    fn clone(&self) -> Self {
        Self {
            callback: Arc::clone(&self.callback),
            timeout: self.timeout,
        }
    }
}

impl fmt::Debug for CallbackHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackHook")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Default for CallbackHook {
    fn default() -> Self {
        Self::new(|_| HookOutput::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::events::HookEvent;

    #[test]
    fn test_callback_hook_new() {
        let hook = CallbackHook::new(|_| HookOutput::allow());
        assert_eq!(hook.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_callback_hook_with_timeout() {
        let hook = CallbackHook::new(|_| HookOutput::allow()).with_timeout(Duration::from_secs(30));
        assert_eq!(hook.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_callback_hook_execute() {
        let hook = CallbackHook::new(|input| {
            HookOutput::allow().with_reason(format!("Processed {}", input.event))
        });

        let input = HookInput::new(HookEvent::PreToolUse, "test-session");
        let output = (hook.callback)(input);
        assert!(output.should_continue);
        assert_eq!(output.reason, Some("Processed PreToolUse".to_string()));
    }

    #[test]
    fn test_callback_hook_clone() {
        let hook = CallbackHook::new(|_| HookOutput::allow());
        let cloned = hook.clone();
        assert_eq!(hook.timeout, cloned.timeout);
    }

    #[test]
    fn test_callback_hook_default() {
        let hook = CallbackHook::default();
        let input = HookInput::default();
        let output = (hook.callback)(input);
        assert_eq!(output, HookOutput::default());
    }
}
