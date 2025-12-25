//! Type definitions for parallel executor

/// RAII guard for holding semaphore permits
pub(super) struct PermitGuard {
    global: Option<tokio::sync::OwnedSemaphorePermit>,
    type_permit: Option<tokio::sync::OwnedSemaphorePermit>,
    limited: Option<tokio::sync::OwnedSemaphorePermit>,
}

impl PermitGuard {
    pub(super) fn new() -> Self {
        Self {
            global: None,
            type_permit: None,
            limited: None,
        }
    }

    pub(super) fn add_global(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.global = Some(permit);
    }

    pub(super) fn add_type(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.type_permit = Some(permit);
    }

    pub(super) fn add_limited(&mut self, permit: tokio::sync::OwnedSemaphorePermit) {
        self.limited = Some(permit);
    }
}
