//! Tests for tool base functionality

#![cfg(test)]

// Test modules - use path attributes since files are in parent directory
#[path = "test_command.rs"]
mod test_command;
#[path = "test_error.rs"]
mod test_error;
#[path = "test_execution.rs"]
mod test_execution;
#[path = "test_filesystem.rs"]
mod test_filesystem;
#[path = "test_mocks.rs"]
mod test_mocks;
