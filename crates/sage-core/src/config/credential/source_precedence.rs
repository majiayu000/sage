//! Canonical credential source precedence.

use super::source::{CredentialPriority, CredentialSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialPrecedenceEntry {
    pub priority: CredentialPriority,
    pub rank: u8,
    pub secure_by_default: bool,
    pub legacy_plaintext: bool,
}

const PRECEDENCE: [CredentialPrecedenceEntry; 8] = [
    entry(CredentialPriority::CliArgument, 1, false, false),
    entry(CredentialPriority::Environment, 2, false, false),
    entry(CredentialPriority::SystemKeychain, 3, true, false),
    entry(CredentialPriority::OAuthToken, 4, true, false),
    entry(CredentialPriority::ProjectConfig, 5, false, true),
    entry(CredentialPriority::GlobalConfig, 6, false, true),
    entry(CredentialPriority::AutoImported, 7, false, true),
    entry(CredentialPriority::Default, 8, false, false),
];

pub fn credential_source_precedence() -> &'static [CredentialPrecedenceEntry] {
    &PRECEDENCE
}

const fn entry(
    priority: CredentialPriority,
    rank: u8,
    secure_by_default: bool,
    legacy_plaintext: bool,
) -> CredentialPrecedenceEntry {
    CredentialPrecedenceEntry {
        priority,
        rank,
        secure_by_default,
        legacy_plaintext,
    }
}

pub fn precedence_rank(source: &CredentialSource) -> u8 {
    credential_source_precedence()
        .iter()
        .find(|entry| entry.priority == source.priority())
        .map(|entry| entry.rank)
        .unwrap_or(u8::MAX)
}

pub fn is_legacy_plaintext_source(source: &CredentialSource) -> bool {
    matches!(
        source,
        CredentialSource::ProjectConfig { .. }
            | CredentialSource::GlobalConfig { .. }
            | CredentialSource::AutoImported { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::credential::CredentialSource;

    #[test]
    fn secure_backend_precedes_legacy_plaintext() {
        assert!(
            precedence_rank(&CredentialSource::keychain("sage"))
                < precedence_rank(&CredentialSource::global("credentials.json"))
        );
        assert!(is_legacy_plaintext_source(&CredentialSource::project(
            ".sage/credentials.json"
        )));
    }
}
