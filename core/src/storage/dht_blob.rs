//! DHT blob storage for offline availability
//! 
//! Blobs (encrypted messages, attachments) are replicated to the DHT
//! so users can fetch them even when the original author is offline.

use crate::storage::{BlobHash, EncryptedBlob};
use crate::types::SpaceId;
use crate::{Error, Result};
use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use minicbor::{Encode, Decode};

/// Encrypted blob for DHT storage
/// 
/// Blobs are already encrypted once (for local storage), but we encrypt
/// them again with a Space-derived key for DHT storage. This allows
/// Space members to discover and decrypt blobs without knowing the
/// original encryption key.
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct DhtBlob {
    /// Space ID (used to derive decryption key)
    #[n(0)]
    pub space_id: SpaceId,
    
    /// Content hash (for verification)
    #[n(1)]
    pub content_hash: BlobHash,
    
    /// AES-GCM nonce (96 bits)
    #[n(2)]
    pub nonce: [u8; 12],
    
    /// Encrypted blob data (contains the locally-encrypted blob)
    #[b(3)]
    pub ciphertext: Vec<u8>,
}

impl DhtBlob {
    /// Encrypt a locally-encrypted blob for DHT storage
    /// 
    /// Takes the EncryptedBlob (already encrypted for local storage)
    /// and encrypts it again with the Space-derived key.
    pub fn encrypt(space_id: &SpaceId, blob_hash: &BlobHash, local_blob: &EncryptedBlob) -> Result<Self> {
        // Serialize the locally-encrypted blob
        let plaintext = local_blob.to_bytes()?;
        
        // Derive encryption key from Space ID
        let key = Self::derive_key(space_id);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Crypto(format!("Failed to create cipher: {}", e)))?;
        
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to encrypt blob for DHT: {}", e)))?;
        
        Ok(Self {
            space_id: *space_id,
            content_hash: *blob_hash,
            nonce: nonce_bytes,
            ciphertext,
        })
    }
    
    /// Decrypt a DHT blob to get the locally-encrypted blob
    pub fn decrypt(&self) -> Result<EncryptedBlob> {
        // Derive decryption key from Space ID
        let key = Self::derive_key(&self.space_id);
        
        // Decrypt
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Crypto(format!("Failed to create cipher: {}", e)))?;
        
        let nonce = Nonce::from_slice(&self.nonce);
        
        let plaintext = cipher.decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to decrypt DHT blob: {}", e)))?;
        
        // Deserialize the locally-encrypted blob
        EncryptedBlob::from_bytes(&plaintext)
            .map_err(|e| Error::Serialization(format!("Failed to deserialize blob: {}", e)))
    }
    
    /// Derive encryption key from Space ID
    /// 
    /// Format: SHA-256(b"DESCORD_BLOB_ENCRYPTION_KEY:" + space_id)
    fn derive_key(space_id: &SpaceId) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_BLOB_ENCRYPTION_KEY:");
        hasher.update(space_id.as_bytes());
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }
    
    /// Compute DHT key for this blob
    /// 
    /// Format: SHA-256(b"DESCORD_BLOB:" + space_id + content_hash)
    pub fn dht_key(&self) -> Vec<u8> {
        Self::compute_dht_key(&self.space_id, &self.content_hash)
    }
    
    /// Compute DHT key for a specific space and blob hash
    pub fn compute_dht_key(space_id: &SpaceId, blob_hash: &BlobHash) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_BLOB:");
        hasher.update(space_id.as_bytes());
        hasher.update(blob_hash.as_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Serialize to bytes for DHT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        minicbor::to_vec(self)
            .map_err(|e| Error::Serialization(format!("Failed to serialize DHT blob: {}", e)))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to deserialize DHT blob: {}", e)))
    }
}

/// Index of blobs available in the DHT for a Space
/// 
/// This allows efficient discovery of all blobs without scanning.
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct BlobIndex {
    /// Space ID
    #[n(0)]
    pub space_id: SpaceId,
    
    /// List of blob hashes available in DHT
    #[b(1)]
    pub blob_hashes: Vec<BlobHash>,
    
    /// Total size of all blobs (in bytes)
    #[n(2)]
    pub total_size: u64,
    
    /// Last updated timestamp (Unix epoch seconds)
    #[n(3)]
    pub last_updated: u64,
}

impl BlobIndex {
    /// Create a new empty index
    pub fn new(space_id: SpaceId) -> Self {
        Self {
            space_id,
            blob_hashes: Vec::new(),
            total_size: 0,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// Add a blob to the index
    pub fn add_blob(&mut self, blob_hash: BlobHash, size: u64) {
        if !self.blob_hashes.contains(&blob_hash) {
            self.blob_hashes.push(blob_hash);
        }
        self.total_size += size;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// Compute DHT key for the blob index
    pub fn dht_key(&self) -> Vec<u8> {
        Self::compute_dht_key(&self.space_id)
    }
    
    /// Compute DHT key for a Space's blob index
    pub fn compute_dht_key(space_id: &SpaceId) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_BLOB_INDEX:");
        hasher.update(space_id.as_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        minicbor::to_vec(self)
            .map_err(|e| Error::Serialization(format!("Failed to serialize blob index: {}", e)))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to deserialize blob index: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::EncryptedBlob;
    
    #[test]
    fn test_dht_blob_encryption() {
        let space_id = SpaceId([1u8; 32]);
        let plaintext = b"Hello, DHT!";
        let local_key = [2u8; 32];
        
        // Create locally-encrypted blob
        let local_blob = EncryptedBlob::encrypt(plaintext, &local_key).unwrap();
        let blob_hash = BlobHash::hash(plaintext);
        
        // Encrypt for DHT
        let dht_blob = DhtBlob::encrypt(&space_id, &blob_hash, &local_blob).unwrap();
        
        // Verify space_id and hash
        assert_eq!(dht_blob.space_id, space_id);
        assert_eq!(dht_blob.content_hash, blob_hash);
        
        // Decrypt from DHT
        let decrypted_local_blob = dht_blob.decrypt().unwrap();
        
        // Decrypt local blob
        let decrypted_plaintext = decrypted_local_blob.decrypt(&local_key).unwrap();
        assert_eq!(&decrypted_plaintext[..], plaintext);
    }
    
    #[test]
    fn test_dht_blob_serialization() {
        let space_id = SpaceId([3u8; 32]);
        let plaintext = b"Test data";
        let local_key = [4u8; 32];
        
        let local_blob = EncryptedBlob::encrypt(plaintext, &local_key).unwrap();
        let blob_hash = BlobHash::hash(plaintext);
        
        let dht_blob = DhtBlob::encrypt(&space_id, &blob_hash, &local_blob).unwrap();
        
        // Serialize
        let bytes = dht_blob.to_bytes().unwrap();
        
        // Deserialize
        let deserialized = DhtBlob::from_bytes(&bytes).unwrap();
        
        assert_eq!(deserialized.space_id, space_id);
        assert_eq!(deserialized.content_hash, blob_hash);
        assert_eq!(deserialized.nonce, dht_blob.nonce);
    }
    
    #[test]
    fn test_blob_index() {
        let space_id = SpaceId([5u8; 32]);
        let mut index = BlobIndex::new(space_id);
        
        // Add some blobs
        let hash1 = BlobHash([1u8; 32]);
        let hash2 = BlobHash([2u8; 32]);
        
        index.add_blob(hash1, 100);
        index.add_blob(hash2, 200);
        
        assert_eq!(index.blob_hashes.len(), 2);
        assert_eq!(index.total_size, 300);
        
        // Serialize and deserialize
        let bytes = index.to_bytes().unwrap();
        let deserialized = BlobIndex::from_bytes(&bytes).unwrap();
        
        assert_eq!(deserialized.space_id, space_id);
        assert_eq!(deserialized.blob_hashes.len(), 2);
        assert_eq!(deserialized.total_size, 300);
    }
    
    #[test]
    fn test_dht_key_computation() {
        let space_id = SpaceId([6u8; 32]);
        let blob_hash = BlobHash([7u8; 32]);
        
        let key1 = DhtBlob::compute_dht_key(&space_id, &blob_hash);
        let key2 = DhtBlob::compute_dht_key(&space_id, &blob_hash);
        
        // Keys should be deterministic
        assert_eq!(key1, key2);
        
        // Keys should be 32 bytes (SHA-256)
        assert_eq!(key1.len(), 32);
    }
}
