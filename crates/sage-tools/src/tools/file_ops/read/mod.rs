//! File reading tool with line numbers and pagination

mod binary;
mod reader;
mod schema;
mod tool;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API
pub use tool::ReadTool;
