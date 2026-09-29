use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use url::Url;

#[derive(Debug, Clone)]
pub struct BrowserTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserInput {
    pub url: String,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserTool {
    pub fn new() -> Self {
        Self
    }
}

fn invalid_browser_url() -> ToolError {
    ToolError::InvalidArguments("OpenBrowser requires an http or https URL with a host".to_string())
}

/// Accept only `http` and `https` URLs that have a host.
///
/// Parsing uses the `url` crate, which canonicalizes the input before any
/// process is created. Callers must pass [`Url::as_str`] to the opener.
fn browser_url(raw: &str) -> Result<Url, ToolError> {
    let parsed = Url::parse(raw).map_err(|_| invalid_browser_url())?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
        return Err(invalid_browser_url());
    }
    Ok(parsed)
}

/// One quoted argument for `cmd /D /V:OFF /C start "" <arg>`.
///
/// `%` is doubled so `cmd` restores a literal percent. A quote is encoded as
/// `%22` before that doubling, which keeps this a single argument.
#[cfg(any(windows, test))]
fn windows_quoted_start_arg(canonical: &str) -> String {
    let mut encoded = String::with_capacity(canonical.len());
    for ch in canonical.chars() {
        if ch == '"' {
            encoded.push_str("%22");
        } else {
            encoded.push(ch);
        }
    }
    format!("\"{}\"", encoded.replace('%', "%%"))
}

/// `start` treats the first quoted string as a window title, so the title is
/// empty. `raw_arg` avoids `cmd` quoting rules for the canonical URL.
#[cfg(windows)]
fn windows_browser_command(canonical: &str) -> Command {
    let mut command = Command::new("cmd");
    command.raw_arg("/D");
    command.raw_arg("/V:OFF");
    command.raw_arg("/C");
    command.raw_arg("start");
    command.raw_arg("\"\"");
    command.raw_arg(windows_quoted_start_arg(canonical));
    command
}

fn browser_process(canonical: &str) -> Command {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg(canonical);
        command
    }
    #[cfg(windows)]
    {
        windows_browser_command(canonical)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let mut command = Command::new("xdg-open");
        command.arg(canonical);
        command
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "OpenBrowser"
    }

    fn description(&self) -> &str {
        "Open an http or https URL in the default browser.\n\n1. The tool accepts only an http or https URL with a host and opens that canonical URL in the default browser. File paths and other schemes are rejected.\n2. The tool does not return any content. It is intended for the user to visually inspect and interact with the page. You will not have access to it.\n3. You should not use `open-browser` on a URL that you have called the tool on before in the conversation history, because the page is already open in the user's browser and the user can see it and refresh it themselves. Each time you call `open-browser`, it will jump the user to the browser window, which is highly annoying to the user."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new(
            self.name(),
            self.description(),
            vec![ToolParameter::string(
                "url",
                "An http or https URL to open in the browser.",
            )],
        )
    }

    async fn execute(&self, call: &ToolCall) -> Result<ToolResult, ToolError> {
        let raw_url = call
            .get_string("url")
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'url' parameter".to_string()))?;
        let url = browser_url(&raw_url)?;
        let canonical = url.as_str();

        let result = browser_process(canonical).output().await;

        match result {
            Ok(output) => {
                if output.status.success() {
                    Ok(ToolResult::success(
                        &call.id,
                        self.name(),
                        format!("Opened {} in default browser", canonical),
                    ))
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    Err(ToolError::ExecutionFailed(format!(
                        "Failed to open browser: {}",
                        error
                    )))
                }
            }
            Err(e) => Err(ToolError::ExecutionFailed(format!(
                "Failed to execute browser command: {}",
                e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn open_browser_call(url: &str) -> ToolCall {
        let mut arguments = HashMap::new();
        arguments.insert(
            "url".to_string(),
            serde_json::Value::String(url.to_string()),
        );
        ToolCall::new("open-browser-test", "OpenBrowser", arguments)
    }

    fn assert_invalid_arguments(err: ToolError) {
        assert!(
            matches!(err, ToolError::InvalidArguments(_)),
            "expected InvalidArguments, got {err}"
        );
    }

    #[tokio::test]
    async fn open_browser_execute_rejects_command_path() {
        let err = BrowserTool::new()
            .execute(&open_browser_call("./probe.command"))
            .await
            .expect_err("a local .command path must be rejected");
        assert_invalid_arguments(err);
    }

    #[tokio::test]
    async fn open_browser_execute_rejects_file_url() {
        let err = BrowserTool::new()
            .execute(&open_browser_call("file:///etc/passwd"))
            .await
            .expect_err("file URLs must be rejected");
        assert_invalid_arguments(err);
    }

    #[test]
    fn open_browser_url_rejects_non_web_schemes() {
        for raw in [
            "probe.command:run",
            "javascript:alert(1)",
            r"C:\Windows\System32\calc.exe",
            "data:text/html,hi",
            "smb://server/share",
        ] {
            assert!(browser_url(raw).is_err(), "expected rejection for {raw}");
        }
    }

    #[test]
    fn open_browser_url_canonicalizes_https() {
        assert_eq!(
            browser_url("https://example.com").unwrap().as_str(),
            "https://example.com/"
        );
        assert_eq!(
            browser_url("HTTPS://Example.COM/a").unwrap().as_str(),
            "https://example.com/a"
        );
    }

    #[test]
    fn open_browser_url_accepts_localhost_and_command_suffix() {
        assert_eq!(
            browser_url("http://localhost:3000").unwrap().as_str(),
            "http://localhost:3000/"
        );
        assert_eq!(
            browser_url("https://example.com/path.command")
                .unwrap()
                .as_str(),
            "https://example.com/path.command"
        );
    }

    #[test]
    fn open_browser_url_reduces_leading_space_and_embedded_tabs() {
        assert_eq!(
            browser_url(" \thttps://example.com/a\tb").unwrap().as_str(),
            "https://example.com/ab"
        );
    }

    #[test]
    fn open_browser_url_encodes_quote_in_path() {
        let canonical = browser_url("https://example.com/a\"b").unwrap();
        assert_eq!(canonical.as_str(), "https://example.com/a%22b");
        let quoted = windows_quoted_start_arg(canonical.as_str());
        assert_eq!(quoted, "\"https://example.com/a%%22b\"");
        assert_eq!(quoted.chars().filter(|ch| *ch == '"').count(), 2);
    }

    #[test]
    fn open_browser_windows_quoted_start_arg_quotes_metacharacters() {
        let quoted = windows_quoted_start_arg("https://example.com/search?q=%COMSPEC%&x=1|y^z");
        assert_eq!(
            quoted,
            "\"https://example.com/search?q=%%COMSPEC%%&x=1|y^z\""
        );
        assert_eq!(quoted.chars().filter(|ch| *ch == '"').count(), 2);
        assert_eq!(
            windows_quoted_start_arg("https://example.com/a\"b"),
            "\"https://example.com/a%%22b\""
        );
    }

    #[test]
    fn open_browser_schema_limits_url_to_http_https() {
        let tool = BrowserTool::new();
        let description = tool.description();
        assert!(description.contains("http or https"));
        let schema = tool.schema();
        let url_description = schema.parameters["properties"]["url"]["description"]
            .as_str()
            .expect("url parameter description");
        assert!(url_description.contains("http or https"));
    }
}
