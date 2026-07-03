//! LSP navigation operation implementations.

use super::LspTool;
use super::client::{LspClientError, ProcessLspSession, file_uri};
use super::response::{
    location_items, lsp_position, merge_object, response_json, to_tool_error, type_hierarchy_items,
    workspace_symbol_items,
};
use super::types::{DegradedReason, NavigationResponse};
use sage_core::tools::base::ToolError;
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

impl LspTool {
    /// Go to definition.
    pub(super) async fn go_to_definition(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        self.position_request(
            "go_to_definition",
            "definitionProvider",
            "textDocument/definition",
            file_path,
            line,
            character,
            json!({}),
        )
        .await
    }

    /// Find references.
    pub(super) async fn find_references(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        self.position_request(
            "find_references",
            "referencesProvider",
            "textDocument/references",
            file_path,
            line,
            character,
            json!({"context": {"includeDeclaration": true}}),
        )
        .await
    }

    /// Search workspace symbols.
    pub(super) async fn workspace_symbol(&self, query: &str) -> Result<String, ToolError> {
        self.workspace_symbol_for_language(query, "rust").await
    }

    pub(super) async fn workspace_symbol_for_language(
        &self,
        query: &str,
        language: &str,
    ) -> Result<String, ToolError> {
        let operation = "symbol_search";
        let workspace_root = self.workspace_root_display();
        let Some(server) = self.config.servers.get(language) else {
            return response_json(NavigationResponse::degraded(
                operation,
                Some(language.to_string()),
                workspace_root,
                DegradedReason::LspUnavailable,
                format!("degraded: LSP unavailable for language '{}'", language),
            ));
        };

        let mut session = match self.start_session(server).await {
            Ok(session) => session,
            Err(error) => {
                return response_json(self.degraded_from_error(
                    operation,
                    Some(language.to_string()),
                    error,
                ));
            }
        };

        if !session.supports("workspaceSymbolProvider") {
            let response = NavigationResponse::degraded(
                operation,
                Some(language.to_string()),
                workspace_root,
                DegradedReason::CapabilityUnsupported,
                "degraded: LSP capability unsupported: workspaceSymbolProvider",
            );
            session.shutdown().await;
            return response_json(response);
        }

        let response = match session
            .request("workspace/symbol", json!({"query": query}))
            .await
        {
            Ok(value) => NavigationResponse::ok(
                operation,
                language.to_string(),
                self.workspace_root_display(),
                workspace_symbol_items(&value),
            ),
            Err(error) => self.degraded_from_error(operation, Some(language.to_string()), error),
        };
        session.shutdown().await;
        response_json(response)
    }

    /// Return a Rust type hierarchy from an LSP position.
    pub(super) async fn type_hierarchy(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let operation = "type_hierarchy";
        let resolved_path = self.resolve_workspace_path(file_path);
        let Some(language) = self.detect_language(&resolved_path) else {
            return response_json(self.degraded_no_language(operation, file_path));
        };

        let mut session = match self.session_for_file(&resolved_path, &language).await {
            Ok(session) => session,
            Err(error) => {
                return response_json(self.degraded_from_error(operation, Some(language), error));
            }
        };

        if !session.supports("typeHierarchyProvider") {
            let response = NavigationResponse::degraded(
                operation,
                Some(language),
                self.workspace_root_display(),
                DegradedReason::CapabilityUnsupported,
                "degraded: LSP capability unsupported: typeHierarchyProvider",
            );
            session.shutdown().await;
            return response_json(response);
        }

        let prepare = session
            .request(
                "textDocument/typeHierarchy/prepare",
                json!({
                    "textDocument": {"uri": file_uri(&resolved_path).map_err(to_tool_error)?},
                    "position": lsp_position(line, character)
                }),
            )
            .await;

        let response = match prepare {
            Ok(value) => {
                let mut items = type_hierarchy_items(&value, Some("self"));
                if let Some(first) = value.as_array().and_then(|values| values.first()) {
                    match session
                        .request("typeHierarchy/supertypes", json!({"item": first}))
                        .await
                    {
                        Ok(value) => items.extend(type_hierarchy_items(&value, Some("supertype"))),
                        Err(error) => {
                            session.shutdown().await;
                            return response_json(self.degraded_from_error(
                                operation,
                                Some(language),
                                error,
                            ));
                        }
                    }
                    match session
                        .request("typeHierarchy/subtypes", json!({"item": first}))
                        .await
                    {
                        Ok(value) => items.extend(type_hierarchy_items(&value, Some("subtype"))),
                        Err(error) => {
                            session.shutdown().await;
                            return response_json(self.degraded_from_error(
                                operation,
                                Some(language),
                                error,
                            ));
                        }
                    }
                }
                NavigationResponse::ok(operation, language, self.workspace_root_display(), items)
            }
            Err(error) => self.degraded_from_error(operation, Some(language), error),
        };

        session.shutdown().await;
        response_json(response)
    }

    /// Legacy hover operation: not part of GH137 navigation surface.
    pub(super) async fn hover(
        &self,
        file_path: &str,
        _line: u32,
        _character: u32,
    ) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("hover", file_path))
    }

    /// Legacy document symbols operation: superseded by SymbolSearch.
    pub(super) async fn document_symbol(&self, file_path: &str) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("document_symbol", file_path))
    }

    /// Legacy implementation operation.
    pub(super) async fn go_to_implementation(
        &self,
        file_path: &str,
        _line: u32,
        _character: u32,
    ) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("go_to_implementation", file_path))
    }

    /// Legacy call hierarchy operation.
    pub(super) async fn prepare_call_hierarchy(
        &self,
        file_path: &str,
        _line: u32,
        _character: u32,
    ) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("prepare_call_hierarchy", file_path))
    }

    /// Legacy incoming calls operation.
    pub(super) async fn incoming_calls(
        &self,
        file_path: &str,
        _line: u32,
        _character: u32,
    ) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("incoming_calls", file_path))
    }

    /// Legacy outgoing calls operation.
    pub(super) async fn outgoing_calls(
        &self,
        file_path: &str,
        _line: u32,
        _character: u32,
    ) -> Result<String, ToolError> {
        response_json(self.legacy_degraded("outgoing_calls", file_path))
    }

    async fn position_request(
        &self,
        operation: &str,
        capability: &str,
        method: &str,
        file_path: &str,
        line: u32,
        character: u32,
        extra_params: Value,
    ) -> Result<String, ToolError> {
        let resolved_path = self.resolve_workspace_path(file_path);
        let Some(language) = self.detect_language(&resolved_path) else {
            return response_json(self.degraded_no_language(operation, file_path));
        };

        let mut session = match self.session_for_file(&resolved_path, &language).await {
            Ok(session) => session,
            Err(error) => {
                return response_json(self.degraded_from_error(operation, Some(language), error));
            }
        };

        if !session.supports(capability) {
            let response = NavigationResponse::degraded(
                operation,
                Some(language),
                self.workspace_root_display(),
                DegradedReason::CapabilityUnsupported,
                format!("degraded: LSP capability unsupported: {}", capability),
            );
            session.shutdown().await;
            return response_json(response);
        }

        let mut params = json!({
            "textDocument": {"uri": file_uri(&resolved_path).map_err(to_tool_error)?},
            "position": lsp_position(line, character),
        });
        merge_object(&mut params, extra_params);

        let response = match session.request(method, params).await {
            Ok(value) => NavigationResponse::ok(
                operation,
                language,
                self.workspace_root_display(),
                location_items(&value),
            ),
            Err(error) => self.degraded_from_error(operation, Some(language), error),
        };
        session.shutdown().await;
        response_json(response)
    }

    async fn session_for_file(
        &self,
        file_path: &Path,
        language: &str,
    ) -> Result<ProcessLspSession, LspClientError> {
        let server = self.config.servers.get(language).ok_or_else(|| {
            LspClientError::Unavailable(format!("no LSP server configured for '{}'", language))
        })?;
        let mut session = self.start_session(server).await?;
        session.open_document(file_path).await?;
        Ok(session)
    }

    async fn start_session(
        &self,
        server: &super::config::LspServerConfig,
    ) -> Result<ProcessLspSession, LspClientError> {
        ProcessLspSession::start(
            server,
            &self.working_directory,
            Duration::from_millis(self.config.request_timeout_ms),
        )
        .await
    }

    fn degraded_no_language(&self, operation: &str, file_path: &str) -> NavigationResponse {
        NavigationResponse::degraded(
            operation,
            None,
            self.workspace_root_display(),
            DegradedReason::LspUnavailable,
            format!("degraded: LSP unavailable for file type: {}", file_path),
        )
    }

    fn degraded_from_error(
        &self,
        operation: &str,
        language: Option<String>,
        error: LspClientError,
    ) -> NavigationResponse {
        let reason = match error {
            LspClientError::Unavailable(_) => DegradedReason::LspUnavailable,
            LspClientError::CapabilityUnsupported(_) => DegradedReason::CapabilityUnsupported,
            LspClientError::Timeout(_) => DegradedReason::Timeout,
            LspClientError::ServerExited(_) => DegradedReason::ServerExited,
            LspClientError::Protocol(_) | LspClientError::Io(_) => DegradedReason::ProtocolError,
        };
        NavigationResponse::degraded(
            operation,
            language,
            self.workspace_root_display(),
            reason,
            format!("degraded: {}", error),
        )
    }

    fn legacy_degraded(&self, operation: &str, file_path: &str) -> NavigationResponse {
        let resolved_path = self.resolve_workspace_path(file_path);
        NavigationResponse::degraded(
            operation,
            self.detect_language(&resolved_path),
            self.workspace_root_display(),
            DegradedReason::CapabilityUnsupported,
            format!(
                "degraded: '{}' is not part of the structured navigation surface; use GoToDefinition, FindReferences, SymbolSearch, or TypeHierarchy. file={}",
                operation,
                resolved_path.display()
            ),
        )
    }

    fn workspace_root_display(&self) -> String {
        self.working_directory.to_string_lossy().to_string()
    }
}
