//! Test Generator Tool
//!
//! This tool provides test generation capabilities including:
//! - Unit test generation
//! - Integration test generation
//! - Mock generation
//! - Test data generation

mod generator;
mod schema;
mod tool;
mod types;

#[cfg(test)]
mod tests;

// Re-export public items
pub use types::TestGeneratorTool;
