//! Fast file pattern matching tool using glob patterns

mod matcher;
mod schema;
#[cfg(test)]
mod tests;
mod types;

// Re-export public types
pub use types::GlobTool;
pub use types::MAX_FILES;
