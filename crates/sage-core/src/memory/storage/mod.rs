//! Memory storage backends

mod error;
mod file_storage;
mod in_memory;
mod query;
#[cfg(test)]
mod tests;
#[allow(clippy::module_inception)]
mod r#trait;

pub use error::MemoryStorageError;
pub use file_storage::FileMemoryStorage;
pub use in_memory::InMemoryStorage;
pub use r#trait::MemoryStorage;
