use async_trait::async_trait;
use sage_core::tools::{Tool, ToolCall, ToolError, ToolParameter, ToolResult, ToolSchema};
use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
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

/// NUL-terminated UTF-16 for `ShellExecuteW`. The canonical URL is copied
/// unchanged, so `%20`, `%22`, and `%COMSPEC%` are not doubled or expanded.
#[cfg(any(windows, test))]
fn windows_wide_arg(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(not(windows))]
fn browser_process(canonical: &str) -> Command {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(target_os = "macos"))]
    let program = "xdg-open";

    let mut command = Command::new(program);
    command.arg(canonical);
    command
}

/// Open the canonical URL with `ShellExecuteW`. `lpFile` is not a `cmd /C`
/// command line, so the command interpreter does not expand `%`.
#[cfg(windows)]
fn open_browser_shell_execute(canonical: &str) -> Result<(), ToolError> {
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            hwnd: *mut std::ffi::c_void,
            lp_operation: *const u16,
            lp_file: *const u16,
            lp_parameters: *const u16,
            lp_directory: *const u16,
            n_show_cmd: i32,
        ) -> *mut std::ffi::c_void;
    }

    const SW_SHOWNORMAL: i32 = 1;
    // Values at or below 32 are ShellExecute error codes, not module handles.
    const SHELL_EXECUTE_ERROR_MAX: usize = 32;

    let operation = windows_wide_arg("open");
    let file = windows_wide_arg(canonical);
    // SAFETY: both buffers are NUL-terminated and live for this synchronous call.
    // The other pointers are null, which this API allows. The return value is
    // compared as an integer and is never dereferenced.
    let status = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if (status as usize) > SHELL_EXECUTE_ERROR_MAX {
        Ok(())
    } else {
        Err(ToolError::ExecutionFailed(format!(
            "Failed to open browser: ShellExecuteW returned {}",
            status as usize
        )))
    }
}

async fn open_canonical_url(canonical: &str) -> Result<(), ToolError> {
    #[cfg(windows)]
    {
        let canonical = canonical.to_string();
        match tokio::task::spawn_blocking(move || open_browser_shell_execute(&canonical)).await {
            Ok(result) => result,
            Err(error) => Err(ToolError::ExecutionFailed(format!(
                "Failed to execute browser command: {error}"
            ))),
        }
    }
    #[cfg(not(windows))]
    {
        match browser_process(canonical).output().await {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr);
                Err(ToolError::ExecutionFailed(format!(
                    "Failed to open browser: {error}"
                )))
            }
            Err(error) => Err(ToolError::ExecutionFailed(format!(
                "Failed to execute browser command: {error}"
            ))),
        }
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
        let canonical = url.as_str().to_string();
        open_canonical_url(&canonical).await?;
        Ok(ToolResult::success(
            &call.id,
            self.name(),
            format!("Opened {} in default browser", canonical),
        ))
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

    fn decode_wide_arg(units: &[u16]) -> String {
        assert_eq!(
            units.last().copied(),
            Some(0),
            "wide argument must be NUL-terminated"
        );
        let body = &units[..units.len() - 1];
        assert!(
            !body.contains(&0),
            "wide argument must not contain an embedded NUL"
        );
        String::from_utf16(body).expect("wide opener argument is UTF-16")
    }

    #[test]
    fn open_browser_url_encodes_quote_in_path() {
        let canonical = browser_url("https://example.com/a\"b").unwrap();
        assert_eq!(canonical.as_str(), "https://example.com/a%22b");
        assert_eq!(
            decode_wide_arg(&windows_wide_arg(canonical.as_str())),
            "https://example.com/a%22b"
        );
    }

    #[test]
    fn open_browser_windows_opener_preserves_percent_escapes() {
        let space = browser_url("https://example.com/a%20b").unwrap();
        assert_eq!(space.as_str(), "https://example.com/a%20b");
        assert_eq!(
            decode_wide_arg(&windows_wide_arg(space.as_str())),
            "https://example.com/a%20b"
        );

        let quote = browser_url("https://example.com/a%22b").unwrap();
        assert_eq!(quote.as_str(), "https://example.com/a%22b");
        assert_eq!(
            decode_wide_arg(&windows_wide_arg(quote.as_str())),
            "https://example.com/a%22b"
        );

        let comspec = browser_url("https://example.com/search?q=%COMSPEC%&x=1|y^z").unwrap();
        assert_eq!(
            comspec.as_str(),
            "https://example.com/search?q=%COMSPEC%&x=1|y^z"
        );
        let opener_arg = decode_wide_arg(&windows_wide_arg(comspec.as_str()));
        assert_eq!(opener_arg, "https://example.com/search?q=%COMSPEC%&x=1|y^z");
        assert!(
            !opener_arg.contains("%%"),
            "opener argument must not double percent signs: {opener_arg}"
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
