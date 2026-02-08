//! Authentication module for OAuth 2.0 + PKCE
//!
//! Provides:
//! - OAuth 2.0 authorization code flow
//! - PKCE (Proof Key for Code Exchange) support
//! - Token management and refresh
//! - Secure token storage

mod oauth;
mod pkce;
mod token;

pub use oauth::{OAuthClient, OAuthConfig, OAuthError};
pub use pkce::{PkceChallenge, PkceVerifier};
pub use token::{TokenManager, TokenInfo, TokenStorage};
