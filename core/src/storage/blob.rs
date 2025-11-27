//! Encrypted blob storage
//!
//! Provides AES-256-GCM encryption for message blobs and attachments.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result, anyhow};
use super::BlobHash;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

/// Encrypted blob (AES-256-GCM encrypted data + nonce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// Random nonce (96 bits / 12 bytes for GCM)
    pub nonce: [u8; 12],
    /// Encrypted data (ciphertext + authentication tag)
    pub ciphertext: Vec<u8>,
}

impl EncryptedBlob {
    /// Encrypt data with the given key
    pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<Self> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Initialize cipher
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Failed to create cipher")?;
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {:?}", e))?;
        
        Ok(Self {
            nonce: nonce_bytes,
            ciphertext,
        })
    }
    
    /// Decrypt the blob with the given key
    pub fn decrypt(&self, key: &[u8; 32]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(&self.nonce);
        
        // Initialize cipher
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Failed to create cipher")?;
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {:?}", e))?;
        
        Ok(plaintext)
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .context("Failed to serialize EncryptedBlob")
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .context("Failed to deserialize EncryptedBlob")
    }
    
    /// Write encrypted blob to filesystem
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let bytes = self.to_bytes()?;
        fs::write(path, bytes)
            .context("Failed to write encrypted blob to file")
    }
    
    /// Read encrypted blob from filesystem
    pub fn read_from_file(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .context("Failed to read encrypted blob from file")?;
        Self::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encrypt_decrypt() -> Result<()> {
        let data = b"Hello, encrypted world!";
        let key = [42u8; 32];
        
        let blob = EncryptedBlob::encrypt(data, &key)?;
        let decrypted = blob.decrypt(&key)?;
        
        assert_eq!(data, &decrypted[..]);
        Ok(())
    }
    
    #[test]
    fn test_wrong_key_fails() {
        let data = b"Secret message";
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        
        let blob = EncryptedBlob::encrypt(data, &key1).unwrap();
        let result = blob.decrypt(&key2);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_serialization() -> Result<()> {
        let data = b"Test data";
        let key = [7u8; 32];
        
        let blob = EncryptedBlob::encrypt(data, &key)?;
        let bytes = blob.to_bytes()?;
        let deserialized = EncryptedBlob::from_bytes(&bytes)?;
        
        let decrypted = deserialized.decrypt(&key)?;
        assert_eq!(data, &decrypted[..]);
        
        Ok(())
    }
}
