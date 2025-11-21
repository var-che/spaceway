//! Descord Core Library
//!
//! A privacy-preserving, decentralized communication platform core library.
//! This library provides the foundational components for building decentralized
//! Discord-like applications with E2E encryption, CRDT-based conflict resolution,
//! and MLS group key management.

pub mod client;
pub mod crdt;
pub mod crypto;
pub mod forum;
pub mod mls;
pub mod network;
pub mod permissions;
pub mod storage;

// Testing utilities - available for integration tests
#[cfg(any(test, feature = "test-utils"))]
pub mod smoothtest;
pub mod types;
pub mod version;

pub use client::{Client, ClientConfig};
pub use permissions::{Permissions, PermissionResult};
pub use types::*;
pub use version::{VERSION, version_string, PROTOCOL_VERSION};

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cryptographic operation failed: {0}")]
    Crypto(String),

    #[error("CRDT operation failed: {0}")]
    Crdt(String),

    #[error("Storage operation failed: {0}")]
    Storage(String),

    #[error("Network operation failed: {0}")]
    Network(String),

    #[error("MLS operation failed: {0}")]
    Mls(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid epoch: expected {expected}, got {actual}")]
    InvalidEpoch { expected: u64, actual: u64 },

    #[error("Member not found: {0}")]
    MemberNotFound(String),

    #[error("Operation rejected: {0}")]
    Rejected(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
