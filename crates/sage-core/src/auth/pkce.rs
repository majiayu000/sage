//! PKCE (Proof Key for Code Exchange) implementation
//!
//! RFC 7636: https://tools.ietf.org/html/rfc7636

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE code verifier
#[derive(Debug, Clone)]
pub struct PkceVerifier {
    /// The verifier string (43-128 characters)
    verifier: String,
}

impl PkceVerifier {
    /// Generate a new random verifier
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
        let verifier = URL_SAFE_NO_PAD.encode(&bytes);

        Self { verifier }
    }

    /// Create from existing verifier string
    pub fn from_string(verifier: String) -> Result<Self, PkceError> {
        // Validate length (43-128 characters per RFC 7636)
        if verifier.len() < 43 || verifier.len() > 128 {
            return Err(PkceError::InvalidVerifierLength(verifier.len()));
        }

        // Validate characters (unreserved characters only)
        if !verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~')
        {
            return Err(PkceError::InvalidVerifierCharacters);
        }

        Ok(Self { verifier })
    }

    /// Get the verifier string
    pub fn as_str(&self) -> &str {
        &self.verifier
    }

    /// Generate the challenge from this verifier
    pub fn challenge(&self) -> PkceChallenge {
        PkceChallenge::from_verifier(self)
    }
}

impl Default for PkceVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// PKCE code challenge
#[derive(Debug, Clone)]
pub struct PkceChallenge {
    /// The challenge string (base64url encoded SHA256 hash)
    challenge: String,
    /// The challenge method (always S256)
    method: String,
}

impl PkceChallenge {
    /// Create challenge from verifier using S256 method
    pub fn from_verifier(verifier: &PkceVerifier) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_str().as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            challenge,
            method: "S256".to_string(),
        }
    }

    /// Get the challenge string
    pub fn as_str(&self) -> &str {
        &self.challenge
    }

    /// Get the challenge method
    pub fn method(&self) -> &str {
        &self.method
    }
}

/// PKCE errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum PkceError {
    #[error("Invalid verifier length: {0} (must be 43-128)")]
    InvalidVerifierLength(usize),

    #[error("Invalid verifier characters (must be unreserved URI characters)")]
    InvalidVerifierCharacters,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_generation() {
        let verifier = PkceVerifier::new();
        assert!(verifier.as_str().len() >= 43);
        assert!(verifier.as_str().len() <= 128);
    }

    #[test]
    fn test_verifier_from_string() {
        let valid = "a".repeat(43);
        assert!(PkceVerifier::from_string(valid).is_ok());

        let too_short = "a".repeat(42);
        assert!(PkceVerifier::from_string(too_short).is_err());

        let too_long = "a".repeat(129);
        assert!(PkceVerifier::from_string(too_long).is_err());

        let invalid_chars = "a".repeat(42) + "!";
        assert!(PkceVerifier::from_string(invalid_chars).is_err());
    }

    #[test]
    fn test_challenge_generation() {
        let verifier = PkceVerifier::new();
        let challenge = verifier.challenge();

        assert_eq!(challenge.method(), "S256");
        assert!(!challenge.as_str().is_empty());
    }

    #[test]
    fn test_challenge_deterministic() {
        let verifier = PkceVerifier::from_string("a".repeat(43)).unwrap();
        let challenge1 = verifier.challenge();
        let challenge2 = verifier.challenge();

        assert_eq!(challenge1.as_str(), challenge2.as_str());
    }
}
