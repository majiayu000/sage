//! Prompt template system
//!
//! This module provides a template system for creating reusable prompts
//! with variable substitution and composition.
//!
//! # Example
//!
//! ```rust,ignore
//! use sage_core::prompts::{PromptTemplate, PromptRegistry};
//!
//! let template = PromptTemplate::new("greeting", "Hello {{name}}, welcome to {{place}}!");
//! let rendered = template.render(&[("name", "Alice"), ("place", "Sage")]);
//! assert_eq!(rendered, "Hello Alice, welcome to Sage!");
//! ```

pub mod template;
pub mod registry;
pub mod builtin;

pub use template::{PromptTemplate, PromptVariable, RenderError};
pub use registry::PromptRegistry;
pub use builtin::BuiltinPrompts;
