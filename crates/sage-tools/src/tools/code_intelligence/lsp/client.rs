//! Process-backed LSP client lifecycle.

use super::config::LspServerConfig;
use super::protocol::{JsonRpcPeer, buffered_reader};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Errors from LSP process and protocol handling.
#[derive(Debug, Error)]
pub(super) enum LspClientError {
    #[error("LSP unavailable: {0}")]
    Unavailable(String),
    #[error("LSP capability unsupported: {0}")]
    CapabilityUnsupported(String),
    #[error("LSP request timed out: {0}")]
    Timeout(String),
    #[error("LSP server exited: {0}")]
    ServerExited(String),
    #[error("LSP protocol error: {0}")]
    Protocol(String),
    #[error("LSP I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// A running LSP session for one language/workspace.
pub(super) struct ProcessLspSession {
    child: Child,
    peer: JsonRpcPeer<tokio::io::BufReader<ChildStdout>, ChildStdin>,
    capabilities: Value,
    timeout: Duration,
    language_id: String,
    workspace_root: PathBuf,
}

impl ProcessLspSession {
    pub(super) async fn start(
        server: &LspServerConfig,
        workspace_root: &Path,
        timeout: Duration,
    ) -> Result<Self, LspClientError> {
        let mut command = Command::new(&server.command);
        command
            .args(&server.args)
            .current_dir(workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|error| {
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
            ) {
                LspClientError::Unavailable(format!(
                    "failed to start '{}': {}",
                    server.command, error
                ))
            } else {
                LspClientError::Io(error)
            }
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            LspClientError::Protocol("failed to capture LSP server stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            LspClientError::Protocol("failed to capture LSP server stdout".to_string())
        })?;

        let mut session = Self {
            child,
            peer: JsonRpcPeer::new(buffered_reader(stdout), stdin),
            capabilities: Value::Null,
            timeout,
            language_id: server.language_id.clone(),
            workspace_root: workspace_root.to_path_buf(),
        };
        session.initialize().await?;
        Ok(session)
    }

    pub(super) fn supports(&self, capability: &str) -> bool {
        match self.capabilities.get(capability) {
            Some(Value::Bool(value)) => *value,
            Some(Value::Object(_)) => true,
            Some(Value::Array(values)) => !values.is_empty(),
            _ => false,
        }
    }

    pub(super) async fn open_document(&mut self, file_path: &Path) -> Result<(), LspClientError> {
        let text = tokio::fs::read_to_string(file_path).await?;
        self.peer
            .notify(
                "textDocument/didOpen",
                json!({
                    "textDocument": {
                        "uri": file_uri(file_path)?,
                        "languageId": self.language_id,
                        "version": 1,
                        "text": text
                    }
                }),
            )
            .await
    }

    pub(super) async fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, LspClientError> {
        self.peer.request(method, params, self.timeout).await
    }

    pub(super) async fn shutdown(mut self) {
        let _ = self
            .peer
            .request("shutdown", Value::Null, self.timeout)
            .await;
        let _ = self.peer.notify("exit", Value::Null).await;
        if tokio::time::timeout(Duration::from_millis(500), self.child.wait())
            .await
            .is_err()
        {
            let _ = self.child.kill().await;
        }
    }

    async fn initialize(&mut self) -> Result<(), LspClientError> {
        let root_uri = file_uri(&self.workspace_root)?;
        let result = self
            .peer
            .request(
                "initialize",
                json!({
                    "processId": std::process::id(),
                    "rootPath": self.workspace_root.to_string_lossy(),
                    "rootUri": root_uri,
                    "capabilities": {
                        "textDocument": {
                            "definition": {"dynamicRegistration": false},
                            "references": {"dynamicRegistration": false},
                            "typeHierarchy": {"dynamicRegistration": false}
                        },
                        "workspace": {
                            "symbol": {"dynamicRegistration": false}
                        }
                    }
                }),
                self.timeout,
            )
            .await?;

        self.capabilities = result.get("capabilities").cloned().unwrap_or(Value::Null);
        self.peer.notify("initialized", json!({})).await?;
        Ok(())
    }
}

pub(super) fn file_uri(path: &Path) -> Result<String, LspClientError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    url::Url::from_file_path(&absolute)
        .map(|uri| uri.to_string())
        .map_err(|_| LspClientError::Protocol(format!("invalid file path: {}", path.display())))
}
