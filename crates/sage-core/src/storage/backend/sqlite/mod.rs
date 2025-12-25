//! SQLite backend module
//!
//! Provides SQLite database backend with in-memory simulation.

mod backend;
mod handlers;

#[cfg(test)]
mod tests;

pub use backend::SqliteBackend;
