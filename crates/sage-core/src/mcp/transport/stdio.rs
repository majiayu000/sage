//! Standard I/O transport for MCP
//!
//! Spawns a subprocess and communicates via stdin/stdout.

use super::McpTransport;
use crate::mcp::error::McpError;
use crate::mcp::protocol::McpMessage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Stdio transport for MCP communication
pub struct StdioTransport {
    /// Child process
    child: Option<Child>,
    /// Stdin writer
    stdin: Option<ChildStdin>,
    /// Stdout reader
    stdout: Option<BufReader<ChildStdout>>,
    /// Line buffer for reading
    line_buffer: String,
    /// Whether connected
    connected: bool,
}

impl StdioTransport {
    /// Spawn a new MCP server process
    pub async fn spawn(
        command: impl AsRef<str>,
        args: &[impl AsRef<str>],
    ) -> Result<Self, McpError> {
        Self::spawn_with_env(command, args, &HashMap::new()).await
    }

    /// Spawn with environment variables
    pub async fn spawn_with_env(
        command: impl AsRef<str>,
        args: &[impl AsRef<str>],
        env: &HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut cmd = Command::new(command.as_ref());

        cmd.args(args.iter().map(|a| a.as_ref()))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Set environment variables
        for (key, value) in env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn().map_err(|e| {
            McpError::Connection(format!(
                "Failed to spawn MCP server '{}': {}",
                command.as_ref(),
                e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Connection("Failed to get stdin handle".into()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Connection("Failed to get stdout handle".into()))?;

        Ok(Self {
            child: Some(child),
            stdin: Some(stdin),
            stdout: Some(BufReader::new(stdout)),
            line_buffer: String::new(),
            connected: true,
        })
    }

    /// Create from existing stdin/stdout (for testing)
    #[cfg(test)]
    pub fn from_parts(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            child: None,
            stdin: Some(stdin),
            stdout: Some(BufReader::new(stdout)),
            line_buffer: String::new(),
            connected: true,
        }
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, message: McpMessage) -> Result<(), McpError> {
        let stdin = self.stdin.as_mut().ok_or(McpError::NotInitialized)?;

        // Serialize message to JSON
        let json = serde_json::to_string(&message)?;

        // Write as a single line (newline delimited JSON)
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        Ok(())
    }

    async fn receive(&mut self) -> Result<McpMessage, McpError> {
        let stdout = self.stdout.as_mut().ok_or(McpError::NotInitialized)?;

        // Clear the buffer
        self.line_buffer.clear();

        // Read a line
        let bytes_read = stdout.read_line(&mut self.line_buffer).await?;

        if bytes_read == 0 {
            self.connected = false;
            return Err(McpError::Connection("Connection closed".into()));
        }

        // Parse the JSON
        let message: McpMessage = serde_json::from_str(self.line_buffer.trim())?;

        Ok(message)
    }

    async fn close(&mut self) -> Result<(), McpError> {
        self.connected = false;

        // Close stdin to signal EOF
        self.stdin.take();

        // Wait for the child process to exit
        if let Some(mut child) = self.child.take() {
            // Give the process a chance to exit gracefully
            tokio::select! {
                result = child.wait() => {
                    result.map_err(|e| McpError::Transport(e.to_string()))?;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    // Force kill if it doesn't exit
                    child.kill().await.ok();
                }
            }
        }

        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        // Best effort cleanup
        if let Some(mut child) = self.child.take() {
            // Start kill but don't wait
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::protocol::McpRequest;

    // Note: Most tests require a real MCP server process
    // These are basic unit tests for the type system

    #[test]
    fn test_transport_not_connected() {
        let transport = StdioTransport {
            child: None,
            stdin: None,
            stdout: None,
            line_buffer: String::new(),
            connected: false,
        };

        assert!(!transport.is_connected());
    }

    #[tokio::test]
    async fn test_serialize_request() {
        let request = McpRequest::new(1i64, "tools/list");
        let json = serde_json::to_string(&request).unwrap();

        assert!(json.contains("\"method\":\"tools/list\""));
    }
}
