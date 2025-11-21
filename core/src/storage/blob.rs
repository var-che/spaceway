/// Blob storage with encryption and content addressing
/// 
/// Blobs are encrypted with AES-256-GCM and stored using SHA256-based
/// content addressing for deduplication and integrity verification.

use super::{BlobHash, Storage};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::{Read, Write};
use zeroize::Zeroizing;
use serde::{Serialize, Deserialize};

/// Encrypted blob structure
/// 
/// Layout on disk: [nonce (12 bytes)][ciphertext (variable)][tag (16 bytes)]
/// The tag is included in the ciphertext by AES-GCM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// AES-GCM nonce (96 bits)
    pub nonce: [u8; 12],
    /// Encrypted data (includes authentication tag)
    pub ciphertext: Vec<u8>,
}

impl EncryptedBlob {
    /// Encrypt plaintext data
    pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<Self> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Invalid encryption key")?;
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("Encryption failed: {}", e))?;
        
        Ok(Self {
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypt to plaintext
    pub fn decrypt(&self, key: &[u8; 32]) -> Result<Zeroizing<Vec<u8>>> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .context("Invalid decryption key")?;
        
        let nonce = Nonce::from_slice(&self.nonce);
        
        // Decrypt
        let plaintext = cipher.decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| anyhow!("Decryption failed: {}", e))?;
        
        Ok(Zeroizing::new(plaintext))
    }

    /// Serialize to bytes for disk storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .context("Failed to serialize blob")
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .context("Failed to deserialize blob")
    }
}

impl Storage {
    /// Store a blob (encrypts and writes to disk)
    /// 
    /// Returns the content-addressed hash of the plaintext.
    /// If a blob with the same hash already exists, it is not written again.
    pub fn store_blob(&self, plaintext: &[u8], key: &[u8; 32]) -> Result<BlobHash> {
        // Compute hash of plaintext (content addressing)
        let hash = BlobHash::hash(plaintext);
        let blob_path = self.blob_dir.join(hash.to_hex());
        
        // Check if blob already exists (deduplication)
        if blob_path.exists() {
            tracing::debug!(
                hash = %hash.to_hex(),
                "Blob already exists, skipping write"
            );
            return Ok(hash);
        }

        // Encrypt
        let encrypted = EncryptedBlob::encrypt(plaintext, key)?;
        let blob_bytes = encrypted.to_bytes()?;

        // Write to disk atomically (write to temp file, then rename)
        let temp_path = blob_path.with_extension("tmp");
        let mut file = fs::File::create(&temp_path)
            .context("Failed to create blob temp file")?;
        file.write_all(&blob_bytes)
            .context("Failed to write blob")?;
        file.sync_all()
            .context("Failed to sync blob file")?;
        
        // Atomic rename
        fs::rename(&temp_path, &blob_path)
            .context("Failed to rename blob file")?;

        tracing::info!(
            hash = %hash.to_hex(),
            size = blob_bytes.len(),
            "Stored encrypted blob"
        );

        Ok(hash)
    }

    /// Load a blob from disk and decrypt
    pub fn load_blob(&self, hash: &BlobHash, key: &[u8; 32]) -> Result<Zeroizing<Vec<u8>>> {
        let blob_path = self.blob_dir.join(hash.to_hex());
        
        // Read encrypted blob
        let mut file = fs::File::open(&blob_path)
            .context("Blob not found")?;
        let mut blob_bytes = Vec::new();
        file.read_to_end(&mut blob_bytes)
            .context("Failed to read blob")?;

        // Deserialize and decrypt
        let encrypted = EncryptedBlob::from_bytes(&blob_bytes)?;
        let plaintext = encrypted.decrypt(key)?;

        // Verify hash integrity
        let computed_hash = BlobHash::hash(&plaintext);
        if computed_hash != *hash {
            return Err(anyhow!(
                "Blob hash mismatch! Expected {}, got {}. Blob may be corrupted.",
                hash.to_hex(),
                computed_hash.to_hex()
            ));
        }

        tracing::debug!(
            hash = %hash.to_hex(),
            size = plaintext.len(),
            "Loaded and decrypted blob"
        );

        Ok(plaintext)
    }

    /// Check if a blob exists
    pub fn blob_exists(&self, hash: &BlobHash) -> bool {
        self.blob_dir.join(hash.to_hex()).exists()
    }

    /// Delete a blob from disk
    pub fn delete_blob(&self, hash: &BlobHash) -> Result<()> {
        let blob_path = self.blob_dir.join(hash.to_hex());
        if blob_path.exists() {
            fs::remove_file(&blob_path)
                .context("Failed to delete blob")?;
            tracing::info!(hash = %hash.to_hex(), "Deleted blob");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_encrypted_blob_roundtrip() -> Result<()> {
        let plaintext = b"Secret message content";
        let key = [42u8; 32];

        // Encrypt
        let encrypted = EncryptedBlob::encrypt(plaintext, &key)?;
        
        // Decrypt
        let decrypted = encrypted.decrypt(&key)?;
        
        assert_eq!(&**decrypted, plaintext);
        Ok(())
    }

    #[test]
    fn test_encrypted_blob_wrong_key() -> Result<()> {
        let plaintext = b"Secret message";
        let key1 = [1u8; 32];
        let key2 = [2u8; 32];

        let encrypted = EncryptedBlob::encrypt(plaintext, &key1)?;
        
        // Decryption with wrong key should fail
        let result = encrypted.decrypt(&key2);
        assert!(result.is_err());
        
        Ok(())
    }

    #[test]
    fn test_blob_serialization() -> Result<()> {
        let plaintext = b"Test data";
        let key = [123u8; 32];

        let encrypted = EncryptedBlob::encrypt(plaintext, &key)?;
        
        // Serialize and deserialize
        let bytes = encrypted.to_bytes()?;
        let deserialized = EncryptedBlob::from_bytes(&bytes)?;
        
        // Should decrypt to same plaintext
        let decrypted = deserialized.decrypt(&key)?;
        assert_eq!(&**decrypted, plaintext);
        
        Ok(())
    }

    #[test]
    fn test_store_and_load_blob() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let plaintext = b"Hello, blob storage!";
        let key = [99u8; 32];

        // Store blob
        let hash = storage.store_blob(plaintext, &key)?;
        
        // Verify it exists
        assert!(storage.blob_exists(&hash));
        
        // Load blob
        let loaded = storage.load_blob(&hash, &key)?;
        assert_eq!(&**loaded, plaintext);
        
        Ok(())
    }

    #[test]
    fn test_blob_deduplication() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let plaintext = b"Duplicate content";
        let key = [77u8; 32];

        // Store same content twice
        let hash1 = storage.store_blob(plaintext, &key)?;
        let hash2 = storage.store_blob(plaintext, &key)?;
        
        // Should have same hash
        assert_eq!(hash1, hash2);
        
        // Should only exist once on disk
        let blob_path = storage.blob_dir().join(hash1.to_hex());
        assert!(blob_path.exists());
        
        // Count files in blob directory
        let blob_count = std::fs::read_dir(storage.blob_dir())?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(blob_count, 1);
        
        Ok(())
    }

    #[test]
    fn test_blob_integrity_check() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let plaintext = b"Integrity test";
        let key = [88u8; 32];

        let hash = storage.store_blob(plaintext, &key)?;
        
        // Corrupt the blob by changing a byte
        let blob_path = storage.blob_dir().join(hash.to_hex());
        let mut blob_data = std::fs::read(&blob_path)?;
        if !blob_data.is_empty() {
            let mid = blob_data.len() / 2;
            blob_data[mid] ^= 0xFF; // Flip some bits
            std::fs::write(&blob_path, &blob_data)?;
        }
        
        // Loading should fail due to authentication tag or hash mismatch
        let result = storage.load_blob(&hash, &key);
        assert!(result.is_err());
        
        Ok(())
    }

    #[test]
    fn test_delete_blob() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let plaintext = b"To be deleted";
        let key = [55u8; 32];

        let hash = storage.store_blob(plaintext, &key)?;
        assert!(storage.blob_exists(&hash));
        
        // Delete blob
        storage.delete_blob(&hash)?;
        assert!(!storage.blob_exists(&hash));
        
        Ok(())
    }
}
