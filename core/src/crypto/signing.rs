//! Ed25519 signing and verification

use crate::{Error, Result};
use crate::types::{Signature, UserId};
use ed25519_dalek::{Signer, Verifier};
use rand::rngs::OsRng;

/// Ed25519 keypair
#[derive(Clone)]
pub struct Keypair {
    inner: ed25519_dalek::SigningKey,
}

impl Keypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let inner = ed25519_dalek::SigningKey::generate(&mut rng);
        Self { inner }
    }

    /// Create keypair from secret key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let inner = ed25519_dalek::SigningKey::from_bytes(bytes);
        Ok(Self { inner })
    }

    /// Get the secret key bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Get the public key
    pub fn public_key(&self) -> PublicKey {
        PublicKey {
            inner: self.inner.verifying_key(),
        }
    }

    /// Get the user ID (public key bytes)
    pub fn user_id(&self) -> UserId {
        UserId(self.public_key().to_bytes())
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.inner.sign(message);
        Signature(sig.to_bytes())
    }
}

/// Ed25519 public key
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PublicKey {
    inner: ed25519_dalek::VerifyingKey,
}

impl PublicKey {
    /// Create public key from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let inner = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|e| Error::Crypto(format!("Invalid public key: {}", e)))?;
        Ok(Self { inner })
    }

    /// Get the public key bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Get the user ID
    pub fn user_id(&self) -> UserId {
        UserId(self.to_bytes())
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        let sig = ed25519_dalek::Signature::from_bytes(&signature.0);
        self.inner
            .verify(message, &sig)
            .map_err(|_| Error::InvalidSignature)
    }
}

impl From<UserId> for PublicKey {
    fn from(user_id: UserId) -> Self {
        Self::from_bytes(&user_id.0).expect("Valid UserId should always convert to PublicKey")
    }
}

/// Ed25519 secret key (not exposed directly, use Keypair)
pub struct SecretKey {
    bytes: [u8; 32],
}

impl SecretKey {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Convert to keypair
    pub fn to_keypair(&self) -> Result<Keypair> {
        Keypair::from_bytes(&self.bytes)
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        // Zero out secret key on drop
        use core::sync::atomic::{compiler_fence, Ordering};
        self.bytes.fill(0);
        compiler_fence(Ordering::SeqCst);
    }
}

/// Hash content using Blake3
pub fn hash_content(data: &[u8]) -> crate::types::ContentHash {
    let hash = blake3::hash(data);
    crate::types::ContentHash(*hash.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = Keypair::generate();
        let public_key = keypair.public_key();
        let user_id = keypair.user_id();
        
        assert_eq!(user_id.0, public_key.to_bytes());
    }

    #[test]
    fn test_sign_verify() {
        let keypair = Keypair::generate();
        let message = b"Hello, Descord!";
        
        let signature = keypair.sign(message);
        let public_key = keypair.public_key();
        
        assert!(public_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let message = b"Test message";
        
        let signature = keypair1.sign(message);
        let public_key2 = keypair2.public_key();
        
        assert!(public_key2.verify(message, &signature).is_err());
    }

    #[test]
    fn test_content_hash() {
        let data = b"Some content to hash";
        let hash1 = hash_content(data);
        let hash2 = hash_content(data);
        
        assert_eq!(hash1, hash2);
        
        let different_data = b"Different content";
        let hash3 = hash_content(different_data);
        
        assert_ne!(hash1, hash3);
    }
}
