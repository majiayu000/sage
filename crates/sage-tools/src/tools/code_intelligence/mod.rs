//! Code intelligence tools
//!
//! This module provides tools for code intelligence features:
//! - LSP: Language Server Protocol integration
//! - GoToDefinition/FindReferences/SymbolSearch/TypeHierarchy: structured navigation

pub mod lsp;

pub use lsp::{
    FindReferencesTool, GoToDefinitionTool, LspTool, SymbolSearchTool, TypeHierarchyTool,
};
