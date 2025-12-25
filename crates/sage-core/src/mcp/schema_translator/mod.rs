//! MCP Schema Translator
//!
//! Provides bidirectional translation between Sage tool schemas and MCP tool schemas.
//! This enables interoperability between Sage's internal tool system and MCP servers.

mod converters;
mod tests;
mod translator;
mod types;

pub use translator::SchemaTranslator;
