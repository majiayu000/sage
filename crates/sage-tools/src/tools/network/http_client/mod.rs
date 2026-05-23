//! HTTP Client Tool
//!
//! This module provides HTTP client functionality including:
//! - REST API interactions
//! - GraphQL support
//! - Custom headers and authentication
//! - Request/response processing
//! - SSRF protection via URL validation

mod request;
mod tool;
mod types;

pub use super::validation::{is_private_ip, validate_url_security};
pub use tool::HttpClientTool;
pub use types::{AuthType, HttpClientParams, HttpMethod, HttpResponse, RequestBody};
