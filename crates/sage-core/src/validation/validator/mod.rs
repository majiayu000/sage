//! Validator implementation

mod core;
mod types;

#[cfg(test)]
mod tests;

pub use core::Validator;
pub use types::{FieldError, ValidationError, ValidationResult};
