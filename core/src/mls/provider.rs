//! OpenMLS crypto provider implementation
//!
//! Provides the cryptographic backend for OpenMLS using rust-crypto

use openmls_rust_crypto::OpenMlsRustCrypto;

/// Descord's OpenMLS crypto provider
///
/// Wraps OpenMlsRustCrypto which provides both crypto and storage
pub type DescordProvider = OpenMlsRustCrypto;

/// Create a new crypto provider instance
pub fn create_provider() -> DescordProvider {
    OpenMlsRustCrypto::default()
}
