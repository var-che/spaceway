//! Cryptographic primitives module
//!
//! This module provides:
//! - Ed25519 signing and verification
//! - Key generation and management
//! - Content hashing (Blake3)

pub mod signing;

pub use signing::{Keypair, PublicKey, SecretKey};
