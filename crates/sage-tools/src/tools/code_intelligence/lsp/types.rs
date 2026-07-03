//! LSP data types

use serde::{Deserialize, Serialize};

/// LSP position (1-based, as shown in editors)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP location (file + position)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
    pub end_line: Option<u32>,
    pub end_character: Option<u32>,
}

/// Symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: String,
    pub location: Location,
    pub container_name: Option<String>,
}

/// Hover information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Location>,
}

/// Call hierarchy item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: String,
    pub location: Location,
    pub detail: Option<String>,
}

/// Result status for structured code navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationStatus {
    Ok,
    Degraded,
}

/// Degraded reason for code navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradedReason {
    LspUnavailable,
    CapabilityUnsupported,
    Timeout,
    ServerExited,
    ProtocolError,
}

/// A code navigation item returned to the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationItem {
    pub file_path: String,
    pub line: u32,
    pub character: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_character: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
}

/// Structured code navigation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationResponse {
    pub status: NavigationStatus,
    pub operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub workspace_root: String,
    pub items: Vec<NavigationItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<DegradedReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl NavigationResponse {
    pub fn ok(
        operation: impl Into<String>,
        language: impl Into<String>,
        workspace_root: impl Into<String>,
        items: Vec<NavigationItem>,
    ) -> Self {
        Self {
            status: NavigationStatus::Ok,
            operation: operation.into(),
            language: Some(language.into()),
            workspace_root: workspace_root.into(),
            items,
            reason: None,
            message: None,
        }
    }

    pub fn degraded(
        operation: impl Into<String>,
        language: Option<String>,
        workspace_root: impl Into<String>,
        reason: DegradedReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: NavigationStatus::Degraded,
            operation: operation.into(),
            language,
            workspace_root: workspace_root.into(),
            items: Vec::new(),
            reason: Some(reason),
            message: Some(message.into()),
        }
    }
}
