//! Version information for Descord
//!
//! This module provides version constants and compatibility checking.

/// Descord version (semver format)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Major version (breaking changes)
pub const VERSION_MAJOR: u32 = 0;

/// Minor version (new features)
pub const VERSION_MINOR: u32 = 1;

/// Patch version (bug fixes)
pub const VERSION_PATCH: u32 = 1;

/// Protocol version (incremented on wire format changes)
pub const PROTOCOL_VERSION: u32 = 1;

/// Build timestamp (Unix epoch)
pub const BUILD_TIMESTAMP: u64 = 1732233600; // Nov 21, 2025 20:00:00 UTC

/// Git commit hash (if available)
pub const GIT_HASH: Option<&str> = option_env!("GIT_HASH");

/// Build profile (debug/release)
pub const BUILD_PROFILE: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "release"
};

/// Full version string with metadata
pub fn version_string() -> String {
    let mut version = format!("Descord v{}", VERSION);
    
    if let Some(hash) = GIT_HASH {
        version.push_str(&format!(" ({})", &hash[..8]));
    }
    
    if BUILD_PROFILE == "debug" {
        version.push_str(" [debug]");
    }
    
    version
}

/// Check if a peer's protocol version is compatible
pub fn is_protocol_compatible(peer_protocol_version: u32) -> bool {
    // For 0.x versions, require exact match
    if VERSION_MAJOR == 0 {
        peer_protocol_version == PROTOCOL_VERSION
    } else {
        // For 1.x+, allow same major version
        peer_protocol_version / 100 == PROTOCOL_VERSION / 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_format() {
        assert_eq!(VERSION, "0.1.0");
        assert_eq!(VERSION_MAJOR, 0);
        assert_eq!(VERSION_MINOR, 1);
        assert_eq!(VERSION_PATCH, 0);
    }

    #[test]
    fn test_protocol_compatibility() {
        // Same protocol version
        assert!(is_protocol_compatible(PROTOCOL_VERSION));
        
        // Different protocol version (incompatible in 0.x)
        assert!(!is_protocol_compatible(PROTOCOL_VERSION + 1));
    }

    #[test]
    fn test_version_string() {
        let version_str = version_string();
        assert!(version_str.starts_with("Descord v0.1.0"));
    }
}
