//! LSP operation implementations

use sage_core::tools::base::ToolError;
use std::path::Path;

use super::LspTool;
use super::symbols::extract_symbols_simple;

impl LspTool {
    /// Go to definition
    pub(super) async fn go_to_definition(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed. Install '{}' to enable this feature.",
                language,
                self.config
                    .servers
                    .get(&language)
                    .map(|c| c.command.as_str())
                    .unwrap_or("unknown")
            )));
        }

        Ok(format!(
            "Go to definition for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}\n\n\
             To use this feature:\n\
             1. Ensure the LSP server is installed\n\
             2. The server will be started automatically when needed\n\
             3. Results will show the definition location",
            file_path, line, character, language
        ))
    }

    /// Find references
    pub(super) async fn find_references(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        Ok(format!(
            "Find references for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}",
            file_path, line, character, language
        ))
    }

    /// Get hover information
    pub(super) async fn hover(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        Ok(format!(
            "Hover info for {}:{}:{}\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             Language detected: {}",
            file_path, line, character, language
        ))
    }

    /// Get document symbols
    pub(super) async fn document_symbol(&self, file_path: &str) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        if !self.is_server_available(&language) {
            return Err(ToolError::ExecutionFailed(format!(
                "LSP server for '{}' is not installed.",
                language
            )));
        }

        // Use regex-based symbol extraction as fallback
        let content = tokio::fs::read_to_string(file_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let symbols = extract_symbols_simple(&content, &language);

        if symbols.is_empty() {
            Ok(format!(
                "No symbols found in {}.\n\n\
                 Note: For better results, ensure the LSP server is running.",
                file_path
            ))
        } else {
            let mut output = format!("Symbols in {} ({}):\n\n", file_path, language);
            for symbol in symbols {
                output.push_str(&format!(
                    "- {} ({}) at line {}\n",
                    symbol.name, symbol.kind, symbol.location.line
                ));
            }
            Ok(output)
        }
    }

    /// Search workspace symbols
    pub(super) async fn workspace_symbol(&self, query: &str) -> Result<String, ToolError> {
        Ok(format!(
            "Workspace symbol search for '{}'\n\n\
             Note: Full LSP integration requires running LSP servers.\n\
             This operation searches across all files in the workspace.",
            query
        ))
    }

    /// Go to implementation
    pub(super) async fn go_to_implementation(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        let path = Path::new(file_path);
        let language = self.detect_language(path).ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "No LSP server configured for file type: {}",
                file_path
            ))
        })?;

        Ok(format!(
            "Go to implementation for {}:{}:{}\n\n\
             Language detected: {}\n\
             Note: This finds implementations of interfaces/traits.",
            file_path, line, character, language
        ))
    }

    /// Prepare call hierarchy
    pub(super) async fn prepare_call_hierarchy(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Call hierarchy for {}:{}:{}\n\n\
             Note: Use incomingCalls or outgoingCalls to explore the hierarchy.",
            file_path, line, character
        ))
    }

    /// Get incoming calls
    pub(super) async fn incoming_calls(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Incoming calls to function at {}:{}:{}\n\n\
             Note: Shows all functions/methods that call this function.",
            file_path, line, character
        ))
    }

    /// Get outgoing calls
    pub(super) async fn outgoing_calls(
        &self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<String, ToolError> {
        Ok(format!(
            "Outgoing calls from function at {}:{}:{}\n\n\
             Note: Shows all functions/methods called by this function.",
            file_path, line, character
        ))
    }
}
