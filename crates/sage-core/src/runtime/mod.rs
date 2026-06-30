//! Unified runtime facade for CLI, SDK, and future protocol handlers.
//!
//! The runtime facade owns the stable boundary around `UnifiedExecutor` setup.
//! It intentionally wraps the existing execution loop instead of introducing a
//! second loop.

mod error;
mod facade;
mod request;
mod response;
mod status;
mod stream;

#[cfg(test)]
mod tests;

pub use error::{
    RuntimeControlResult, RuntimeOperation, boxed_runtime_unsupported_error,
    boxed_runtime_validation_error, runtime_unsupported_error,
};
pub use facade::{
    Runtime, RuntimeExecutor, default_thread_store, default_thread_store_path,
    ensure_thread_store_thread,
};
pub use request::{
    RuntimeForkRequest, RuntimeInterruptRequest, RuntimeResumeRequest, RuntimeStartRequest,
};
pub use response::RuntimeRunResult;
pub use status::{RuntimeStateCapabilities, RuntimeStateMode, RuntimeStatus};
pub use stream::RuntimeProtocolStream;
