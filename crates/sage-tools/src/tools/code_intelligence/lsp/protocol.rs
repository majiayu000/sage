//! JSON-RPC framing for stdio LSP servers.

use super::client::LspClientError;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};

/// JSON-RPC peer over an LSP byte stream.
pub(super) struct JsonRpcPeer<R, W> {
    reader: R,
    writer: W,
    next_id: u64,
}

impl<R, W> JsonRpcPeer<R, W>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub(super) fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            next_id: 1,
        }
    }

    pub(super) async fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout_duration: Duration,
    ) -> Result<Value, LspClientError> {
        let id = self.next_id;
        self.next_id += 1;
        let payload = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        write_lsp_message(&mut self.writer, &payload).await?;

        tokio::time::timeout(timeout_duration, self.read_response(id, method))
            .await
            .map_err(|_| LspClientError::Timeout(method.to_string()))?
    }

    pub(super) async fn notify(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), LspClientError> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        write_lsp_message(&mut self.writer, &payload).await?;
        Ok(())
    }

    async fn read_response(&mut self, id: u64, method: &str) -> Result<Value, LspClientError> {
        loop {
            let message = read_lsp_message(&mut self.reader).await?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = message.get("error") {
                let code = error
                    .get("code")
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("LSP request failed");
                if code == -32601 {
                    return Err(LspClientError::CapabilityUnsupported(method.to_string()));
                }
                return Err(LspClientError::Protocol(format!(
                    "{} failed: {}",
                    method, message
                )));
            }

            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

pub(super) fn buffered_reader<R>(reader: R) -> BufReader<R>
where
    R: tokio::io::AsyncRead + Unpin,
{
    BufReader::new(reader)
}

pub(super) async fn read_lsp_message<R>(reader: &mut R) -> Result<Value, LspClientError>
where
    R: AsyncBufRead + Unpin,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(LspClientError::ServerExited(
                "server closed stdout".to_string(),
            ));
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        let Some((name, value)) = trimmed.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                LspClientError::Protocol(format!("invalid Content-Length header: {}", error))
            })?);
        }
    }

    let length = content_length
        .ok_or_else(|| LspClientError::Protocol("missing Content-Length header".to_string()))?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await?;
    serde_json::from_slice(&body)
        .map_err(|error| LspClientError::Protocol(format!("invalid JSON-RPC body: {}", error)))
}

pub(super) async fn write_lsp_message<W>(
    writer: &mut W,
    payload: &Value,
) -> Result<(), LspClientError>
where
    W: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(payload)
        .map_err(|error| LspClientError::Protocol(format!("invalid JSON payload: {}", error)))?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, split};

    #[tokio::test]
    async fn peer_sends_initialize_did_open_and_request_in_order() {
        let (client_io, server_io) = tokio::io::duplex(16_384);
        let (client_read, client_write) = split(client_io);
        let (server_read, mut server_write) = split(server_io);
        let mut peer = JsonRpcPeer::new(buffered_reader(client_read), client_write);

        let server = tokio::spawn(async move {
            let mut reader = buffered_reader(server_read);
            let mut methods = Vec::new();

            let initialize = read_lsp_message(&mut reader).await.unwrap();
            methods.push(initialize["method"].as_str().unwrap().to_string());
            write_lsp_message(
                &mut server_write,
                &json!({
                    "jsonrpc": "2.0",
                    "id": initialize["id"],
                    "result": {
                        "capabilities": {
                            "definitionProvider": true
                        }
                    }
                }),
            )
            .await
            .unwrap();

            let initialized = read_lsp_message(&mut reader).await.unwrap();
            methods.push(initialized["method"].as_str().unwrap().to_string());

            let did_open = read_lsp_message(&mut reader).await.unwrap();
            methods.push(did_open["method"].as_str().unwrap().to_string());

            let definition = read_lsp_message(&mut reader).await.unwrap();
            methods.push(definition["method"].as_str().unwrap().to_string());
            write_lsp_message(
                &mut server_write,
                &json!({
                    "jsonrpc": "2.0",
                    "id": definition["id"],
                    "result": []
                }),
            )
            .await
            .unwrap();
            server_write.shutdown().await.unwrap();

            methods
        });

        let initialize = peer
            .request(
                "initialize",
                json!({"capabilities": {}}),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert!(
            initialize["capabilities"]["definitionProvider"]
                .as_bool()
                .unwrap()
        );
        peer.notify("initialized", json!({})).await.unwrap();
        peer.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": "file:///tmp/lib.rs",
                    "languageId": "rust",
                    "version": 1,
                    "text": "pub fn sample() {}"
                }
            }),
        )
        .await
        .unwrap();
        let result = peer
            .request(
                "textDocument/definition",
                json!({
                    "textDocument": {"uri": "file:///tmp/lib.rs"},
                    "position": {"line": 0, "character": 7}
                }),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        assert_eq!(result, json!([]));

        assert_eq!(
            server.await.unwrap(),
            vec![
                "initialize",
                "initialized",
                "textDocument/didOpen",
                "textDocument/definition"
            ]
        );
    }
}
