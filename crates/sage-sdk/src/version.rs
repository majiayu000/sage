//! API Versioning for Sage Agent SDK
//!
//! This module provides versioning utilities for the Sage Agent SDK, following
//! semantic versioning principles (SemVer 2.0.0).
//!
//! ## Versioning Strategy
//!
//! The SDK uses semantic versioning with three components: MAJOR.MINOR.PATCH
//!
//! - **MAJOR**: Incremented for incompatible API changes
//! - **MINOR**: Incremented for backward-compatible functionality additions
//! - **PATCH**: Incremented for backward-compatible bug fixes
//!
//! ## Version Compatibility
//!
//! The SDK maintains backward compatibility within the same MAJOR version.
//! Clients can check compatibility using the version negotiation utilities.
//!
//! ## Deprecation Policy
//!
//! When deprecating APIs:
//! 1. Mark with `#[deprecated]` attribute and use deprecation macros
//! 2. Provide migration path in documentation
//! 3. Maintain deprecated APIs for at least one MINOR version
//! 4. Remove in next MAJOR version
//!
//! ## Example
//!
//! ```rust
//! use sage_sdk::version::{API_VERSION, Version, is_compatible};
//!
//! // Check if client version is compatible
//! let client_version = Version::new(0, 1, 0);
//! assert!(is_compatible(&client_version));
//!
//! // Parse version from string
//! let version = Version::parse("0.1.0").unwrap();
//! assert_eq!(version.major(), 0);
//! ```

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Current API version of the Sage Agent SDK
///
/// This version follows semantic versioning and represents the public API surface.
/// When the API changes in an incompatible way, the MAJOR version will be incremented.
pub const API_VERSION: Version = Version {
    major: 0,
    minor: 1,
    patch: 0,
};

/// Minimum supported API version
///
/// Clients using versions older than this may encounter compatibility issues.
/// This allows the SDK to drop support for very old versions while maintaining
/// backward compatibility within the supported range.
pub const MIN_SUPPORTED_VERSION: Version = Version {
    major: 0,
    minor: 1,
    patch: 0,
};

/// Represents a semantic version (MAJOR.MINOR.PATCH)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl Version {
    /// Create a new version
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::version::Version;
    ///
    /// let version = Version::new(1, 2, 3);
    /// assert_eq!(version.major(), 1);
    /// assert_eq!(version.minor(), 2);
    /// assert_eq!(version.patch(), 3);
    /// ```
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse a version string in the format "MAJOR.MINOR.PATCH"
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::version::Version;
    ///
    /// let version = Version::parse("1.2.3").unwrap();
    /// assert_eq!(version, Version::new(1, 2, 3));
    /// ```
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        s.parse()
    }

    /// Get the major version component
    pub const fn major(&self) -> u32 {
        self.major
    }

    /// Get the minor version component
    pub const fn minor(&self) -> u32 {
        self.minor
    }

    /// Get the patch version component
    pub const fn patch(&self) -> u32 {
        self.patch
    }

    /// Check if this version is compatible with another version
    ///
    /// Two versions are compatible if they have the same MAJOR version and
    /// this version is greater than or equal to the minimum required version.
    ///
    /// # Example
    ///
    /// ```
    /// use sage_sdk::version::Version;
    ///
    /// let v1 = Version::new(1, 2, 0);
    /// let v2 = Version::new(1, 3, 0);
    /// let v3 = Version::new(2, 0, 0);
    ///
    /// assert!(v2.is_compatible_with(&v1)); // Same major, v2 >= v1
    /// assert!(!v1.is_compatible_with(&v2)); // Same major, but v1 < v2
    /// assert!(!v3.is_compatible_with(&v1)); // Different major
    /// ```
    pub const fn is_compatible_with(&self, required: &Version) -> bool {
        // Must have same major version
        if self.major != required.major {
            return false;
        }

        // Must be at least the required version
        if self.minor < required.minor {
            return false;
        }

        if self.minor == required.minor && self.patch < required.patch {
            return false;
        }

        true
    }

    /// Check if this version is within the supported range
    ///
    /// A version is supported if it's between MIN_SUPPORTED_VERSION and API_VERSION.
    pub const fn is_supported(&self) -> bool {
        self.is_compatible_with(&MIN_SUPPORTED_VERSION)
            && API_VERSION.is_compatible_with(self)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::InvalidFormat(s.to_string()));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| VersionError::InvalidComponent("major".to_string(), parts[0].to_string()))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| VersionError::InvalidComponent("minor".to_string(), parts[1].to_string()))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| VersionError::InvalidComponent("patch".to_string(), parts[2].to_string()))?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

/// Errors that can occur during version parsing or negotiation
#[derive(Debug, Error)]
pub enum VersionError {
    /// Version string has invalid format (expected "MAJOR.MINOR.PATCH")
    #[error("Invalid version format: {0} (expected MAJOR.MINOR.PATCH)")]
    InvalidFormat(String),

    /// Version component could not be parsed as a number
    #[error("Invalid {0} component: {1}")]
    InvalidComponent(String, String),

    /// Requested version is not compatible with current API version
    #[error("Incompatible version: requested {requested}, current API is {current}")]
    Incompatible {
        requested: Version,
        current: Version,
    },

    /// Requested version is not supported (too old)
    #[error("Unsupported version: {requested} (minimum supported is {min_supported})")]
    Unsupported {
        requested: Version,
        min_supported: Version,
    },
}

/// Check if a client version is compatible with the current API
///
/// Returns `true` if the client can safely use this SDK version.
///
/// # Example
///
/// ```
/// use sage_sdk::version::{Version, is_compatible};
///
/// let client_version = Version::new(0, 1, 0);
/// assert!(is_compatible(&client_version));
/// ```
pub fn is_compatible(client_version: &Version) -> bool {
    API_VERSION.is_compatible_with(client_version) && client_version.is_supported()
}

/// Negotiate API version with a client
///
/// Returns `Ok(())` if the versions are compatible, otherwise returns an error
/// explaining the incompatibility.
///
/// # Example
///
/// ```
/// use sage_sdk::version::{Version, negotiate_version};
///
/// let client_version = Version::new(0, 1, 0);
/// assert!(negotiate_version(&client_version).is_ok());
///
/// let incompatible = Version::new(1, 0, 0);
/// assert!(negotiate_version(&incompatible).is_err());
/// ```
pub fn negotiate_version(client_version: &Version) -> Result<(), VersionError> {
    // Check if version is supported (not too old)
    if !client_version.is_supported() {
        return Err(VersionError::Unsupported {
            requested: *client_version,
            min_supported: MIN_SUPPORTED_VERSION,
        });
    }

    // Check if version is compatible (same major, within range)
    if !is_compatible(client_version) {
        return Err(VersionError::Incompatible {
            requested: *client_version,
            current: API_VERSION,
        });
    }

    Ok(())
}

/// Get the current SDK version as a string
///
/// This returns the API version in "MAJOR.MINOR.PATCH" format.
///
/// # Example
///
/// ```
/// use sage_sdk::version::version_string;
///
/// let version = version_string();
/// assert_eq!(version, "0.1.0");
/// ```
pub fn version_string() -> String {
    API_VERSION.to_string()
}

/// Get version information for display
///
/// Returns a formatted string with version details suitable for CLI output.
///
/// # Example
///
/// ```
/// use sage_sdk::version::version_info;
///
/// let info = version_info();
/// assert!(info.contains("Sage Agent SDK"));
/// assert!(info.contains("0.1.0"));
/// ```
pub fn version_info() -> String {
    format!(
        "Sage Agent SDK v{}\nMinimum Supported Version: v{}\nAPI Stability: {}",
        API_VERSION,
        MIN_SUPPORTED_VERSION,
        if API_VERSION.major == 0 {
            "Development (pre-1.0, API may change)"
        } else {
            "Stable"
        }
    )
}

/// Macro to mark a function or type as deprecated with version information
///
/// This macro generates a standard deprecation warning with the version when
/// the API was deprecated and a suggested alternative.
///
/// # Example
///
/// ```rust,ignore
/// use sage_sdk::deprecated_since;
///
/// #[deprecated_since(version = "0.2.0", note = "Use new_function() instead")]
/// pub fn old_function() {
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! deprecated_since {
    (version = $ver:expr, note = $note:expr) => {
        #[deprecated(since = $ver, note = $note)]
    };
}

/// Macro to mark experimental APIs that may change
///
/// Experimental APIs are not covered by semantic versioning guarantees and
/// may change or be removed in any release.
///
/// # Example
///
/// ```rust,ignore
/// use sage_sdk::experimental;
///
/// #[experimental(note = "This API is experimental and may change")]
/// pub fn experimental_function() {
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! experimental {
    (note = $note:expr) => {
        #[doc = concat!("\n\n**⚠️ EXPERIMENTAL**: ", $note, "\n\n")]
        #[doc = "This API is experimental and not covered by semantic versioning guarantees."]
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 2);
        assert_eq!(v.patch(), 3);
    }

    #[test]
    fn test_version_parsing() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));

        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.3.4").is_err());
        assert!(Version::parse("a.b.c").is_err());
    }

    #[test]
    fn test_version_display() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_compatibility() {
        let v1_0_0 = Version::new(1, 0, 0);
        let v1_2_0 = Version::new(1, 2, 0);
        let v1_2_3 = Version::new(1, 2, 3);
        let v2_0_0 = Version::new(2, 0, 0);

        // Same major version, newer API is compatible with older clients
        assert!(v1_2_3.is_compatible_with(&v1_0_0));
        assert!(v1_2_3.is_compatible_with(&v1_2_0));
        assert!(v1_2_3.is_compatible_with(&v1_2_3));

        // Same major version, older API is not compatible with newer clients
        assert!(!v1_0_0.is_compatible_with(&v1_2_0));
        assert!(!v1_2_0.is_compatible_with(&v1_2_3));

        // Different major versions are incompatible
        assert!(!v2_0_0.is_compatible_with(&v1_0_0));
        assert!(!v1_0_0.is_compatible_with(&v2_0_0));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(1, 1, 1);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v1 < v4);
    }

    #[test]
    fn test_current_api_version() {
        assert_eq!(API_VERSION, Version::new(0, 1, 0));
        assert_eq!(MIN_SUPPORTED_VERSION, Version::new(0, 1, 0));
    }

    #[test]
    fn test_is_compatible() {
        let current = Version::new(0, 1, 0);
        assert!(is_compatible(&current));

        // Future minor version should be compatible with current
        let future_minor = Version::new(0, 2, 0);
        assert!(!is_compatible(&future_minor)); // Not compatible because API is older

        // Different major version
        let different_major = Version::new(1, 0, 0);
        assert!(!is_compatible(&different_major));
    }

    #[test]
    fn test_negotiate_version() {
        let current = Version::new(0, 1, 0);
        assert!(negotiate_version(&current).is_ok());

        let incompatible = Version::new(1, 0, 0);
        assert!(negotiate_version(&incompatible).is_err());
    }

    #[test]
    fn test_version_string() {
        assert_eq!(version_string(), "0.1.0");
    }

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains("Sage Agent SDK"));
        assert!(info.contains("0.1.0"));
        assert!(info.contains("Development"));
    }

    #[test]
    fn test_is_supported() {
        let current = Version::new(0, 1, 0);
        assert!(current.is_supported());

        // Too old (below minimum)
        let too_old = Version::new(0, 0, 1);
        assert!(!too_old.is_supported());

        // Future version (above current)
        let future = Version::new(0, 2, 0);
        assert!(!future.is_supported());
    }
}
