//! DHT storage for CRDT operations
//!
//! This module handles storing and retrieving CRDT operations from the DHT.
//! Operations are batched by Space and encrypted before storage.

use crate::crdt::CrdtOp;
use crate::types::SpaceId;
use crate::{Error, Result};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};

/// A batch of CRDT operations for a Space
/// 
/// Operations are batched to reduce DHT storage overhead.
/// Each batch contains operations from a specific epoch range.
#[derive(Clone, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct OperationBatch {
    /// Space ID these operations belong to
    #[n(0)]
    pub space_id: SpaceId,
    
    /// List of operations in this batch
    #[n(1)]
    pub operations: Vec<CrdtOp>,
    
    /// Earliest operation timestamp in this batch
    #[n(2)]
    pub start_time: u64,
    
    /// Latest operation timestamp in this batch
    #[n(3)]
    pub end_time: u64,
    
    /// Total number of operations
    #[n(4)]
    pub count: u32,
    
    /// Batch sequence number (for ordering)
    #[n(5)]
    pub sequence: u32,
}

impl OperationBatch {
    /// Create a new operation batch
    pub fn new(space_id: SpaceId, operations: Vec<CrdtOp>, sequence: u32) -> Self {
        let start_time = operations.iter().map(|op| op.timestamp).min().unwrap_or(0);
        let end_time = operations.iter().map(|op| op.timestamp).max().unwrap_or(0);
        let count = operations.len() as u32;
        
        Self {
            space_id,
            operations,
            start_time,
            end_time,
            count,
            sequence,
        }
    }
    
    /// Serialize to bytes (CBOR)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        minicbor::encode(self, &mut buf)
            .map_err(|e| Error::Serialization(format!("Failed to encode operation batch: {}", e)))?;
        Ok(buf)
    }
    
    /// Deserialize from bytes (CBOR)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to decode operation batch: {}", e)))
    }
}

/// Encrypted operation batch for DHT storage
/// 
/// Operations contain sensitive metadata (who did what, when).
/// We encrypt them with a key derived from the Space ID.
#[derive(Clone, Debug)]
pub struct EncryptedOperationBatch {
    /// Space ID (used to derive decryption key)
    pub space_id: SpaceId,
    
    /// Batch sequence number
    pub sequence: u32,
    
    /// AES-GCM nonce (96 bits)
    pub nonce: [u8; 12],
    
    /// Encrypted batch data
    pub ciphertext: Vec<u8>,
}

impl EncryptedOperationBatch {
    /// Encrypt an operation batch
    pub fn encrypt(batch: &OperationBatch) -> Result<Self> {
        // Serialize batch
        let plaintext = batch.to_bytes()?;
        
        // Derive encryption key from Space ID
        let key = Self::derive_key(&batch.space_id);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::RngCore;
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Crypto(format!("Failed to create cipher: {}", e)))?;
        
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to encrypt batch: {}", e)))?;
        
        Ok(Self {
            space_id: batch.space_id,
            sequence: batch.sequence,
            nonce: nonce_bytes,
            ciphertext,
        })
    }
    
    /// Decrypt an operation batch
    pub fn decrypt(&self) -> Result<OperationBatch> {
        // Derive decryption key from Space ID
        let key = Self::derive_key(&self.space_id);
        
        // Decrypt
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| Error::Crypto(format!("Failed to create cipher: {}", e)))?;
        
        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher.decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to decrypt batch: {}", e)))?;
        
        // Deserialize
        OperationBatch::from_bytes(&plaintext)
    }
    
    /// Derive 256-bit encryption key from Space ID
    /// 
    /// Uses SHA-256 to derive a deterministic key from the Space ID.
    /// Anyone who knows the Space ID can decrypt the operations.
    fn derive_key(space_id: &SpaceId) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_OPS_ENCRYPTION_KEY:");
        hasher.update(space_id.as_bytes());
        hasher.finalize().into()
    }
    
    /// Get DHT storage key for this batch
    /// 
    /// Format: SHA-256(b"DESCORD_OPS_BATCH:" + space_id + sequence)
    pub fn dht_key(&self) -> Vec<u8> {
        Self::compute_dht_key(&self.space_id, self.sequence)
    }
    
    /// Compute DHT key for a specific space and sequence
    /// 
    /// Format: SHA-256(b"DESCORD_OPS_BATCH:" + space_id + sequence)
    pub fn compute_dht_key(space_id: &SpaceId, sequence: u32) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_OPS_BATCH:");
        hasher.update(space_id.as_bytes());
        hasher.update(&sequence.to_le_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Serialize to bytes for DHT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        
        // Write space_id (32 bytes)
        buf.extend_from_slice(self.space_id.as_bytes());
        
        // Write sequence (4 bytes)
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        
        // Write nonce (12 bytes)
        buf.extend_from_slice(&self.nonce);
        
        // Write ciphertext length (4 bytes) + ciphertext
        buf.extend_from_slice(&(self.ciphertext.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.ciphertext);
        
        Ok(buf)
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 52 {
            return Err(Error::Serialization("Encrypted batch too short".to_string()));
        }
        
        // Read space_id (32 bytes)
        let mut space_id_bytes = [0u8; 32];
        space_id_bytes.copy_from_slice(&bytes[0..32]);
        let space_id = SpaceId(space_id_bytes);
        
        // Read sequence (4 bytes)
        let sequence = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        
        // Read nonce (12 bytes)
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[36..48]);
        
        // Read ciphertext length (4 bytes)
        let ciphertext_len = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]) as usize;
        
        // Read ciphertext
        if bytes.len() < 52 + ciphertext_len {
            return Err(Error::Serialization("Ciphertext truncated".to_string()));
        }
        let ciphertext = bytes[52..52 + ciphertext_len].to_vec();
        
        Ok(Self {
            space_id,
            sequence,
            nonce,
            ciphertext,
        })
    }
}

/// Metadata about available operation batches for a Space
/// 
/// Stored in DHT to allow discovery of all batches.
#[derive(Clone, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct OperationBatchIndex {
    /// Space ID
    #[n(0)]
    pub space_id: SpaceId,
    
    /// List of available batch sequence numbers
    #[n(1)]
    pub batch_sequences: Vec<u32>,
    
    /// Total number of operations across all batches
    #[n(2)]
    pub total_operations: u64,
    
    /// Last updated timestamp
    #[n(3)]
    pub last_updated: u64,
}

impl OperationBatchIndex {
    /// Create new index
    pub fn new(space_id: SpaceId) -> Self {
        Self {
            space_id,
            batch_sequences: Vec::new(),
            total_operations: 0,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// Add a batch to the index
    pub fn add_batch(&mut self, sequence: u32, op_count: u32) {
        if !self.batch_sequences.contains(&sequence) {
            self.batch_sequences.push(sequence);
            self.batch_sequences.sort();
        }
        self.total_operations += op_count as u64;
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
    
    /// Get DHT storage key for this index
    pub fn dht_key(&self) -> Vec<u8> {
        Self::compute_dht_key(&self.space_id)
    }
    
    /// Compute DHT key for a specific Space's operation index
    pub fn compute_dht_key(space_id: &SpaceId) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_OPS_INDEX:");
        hasher.update(space_id.as_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        minicbor::encode(self, &mut buf)
            .map_err(|e| Error::Serialization(format!("Failed to encode index: {}", e)))?;
        Ok(buf)
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to decode index: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpId, UserId, EpochId, Signature};
    use crate::crdt::{OpType, OpPayload, Hlc};
    
    fn create_test_op(timestamp: u64) -> CrdtOp {
        CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id: SpaceId::new(),
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateSpace(OpPayload::CreateSpace {
                name: "Test".to_string(),
                description: None,
            }),
            prev_ops: vec![],
            author: UserId([0u8; 32]),
            epoch: EpochId(0),
            hlc: Hlc { wall_time: timestamp, logical: 0 },
            timestamp,
            signature: Signature([0u8; 64]),
        }
    }
    
    #[test]
    fn test_operation_batch_serialization() {
        let space_id = SpaceId::new();
        let ops = vec![create_test_op(1000), create_test_op(2000)];
        let batch = OperationBatch::new(space_id, ops, 0);
        
        let bytes = batch.to_bytes().unwrap();
        let decoded = OperationBatch::from_bytes(&bytes).unwrap();
        
        assert_eq!(decoded.space_id, batch.space_id);
        assert_eq!(decoded.operations.len(), 2);
        assert_eq!(decoded.start_time, 1000);
        assert_eq!(decoded.end_time, 2000);
    }
    
    #[test]
    fn test_encrypted_batch_round_trip() {
        let space_id = SpaceId::new();
        let ops = vec![create_test_op(1000), create_test_op(2000)];
        let batch = OperationBatch::new(space_id, ops, 0);
        
        let encrypted = EncryptedOperationBatch::encrypt(&batch).unwrap();
        let decrypted = encrypted.decrypt().unwrap();
        
        assert_eq!(decrypted.space_id, batch.space_id);
        assert_eq!(decrypted.operations.len(), batch.operations.len());
    }
    
    #[test]
    fn test_encrypted_batch_serialization() {
        let space_id = SpaceId::new();
        let ops = vec![create_test_op(1000)];
        let batch = OperationBatch::new(space_id, ops, 0);
        let encrypted = EncryptedOperationBatch::encrypt(&batch).unwrap();
        
        let bytes = encrypted.to_bytes().unwrap();
        let decoded = EncryptedOperationBatch::from_bytes(&bytes).unwrap();
        
        assert_eq!(decoded.space_id, encrypted.space_id);
        assert_eq!(decoded.sequence, encrypted.sequence);
        assert_eq!(decoded.nonce, encrypted.nonce);
        assert_eq!(decoded.ciphertext, encrypted.ciphertext);
    }
    
    #[test]
    fn test_batch_index() {
        let space_id = SpaceId::new();
        let mut index = OperationBatchIndex::new(space_id);
        
        index.add_batch(0, 10);
        index.add_batch(1, 20);
        
        assert_eq!(index.batch_sequences, vec![0, 1]);
        assert_eq!(index.total_operations, 30);
        
        let bytes = index.to_bytes().unwrap();
        let decoded = OperationBatchIndex::from_bytes(&bytes).unwrap();
        
        assert_eq!(decoded.batch_sequences, index.batch_sequences);
        assert_eq!(decoded.total_operations, index.total_operations);
    }
}
