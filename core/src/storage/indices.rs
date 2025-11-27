//! Message and blob indices
//!
//! Provides metadata indexing for messages and blobs.

use super::BlobHash;
use crate::types::{UserId, ThreadId, MessageId};
use serde::{Serialize, Deserialize};

/// Metadata for a stored blob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// Content hash
    pub hash: BlobHash,
    
    /// Size in bytes
    pub size: u64,
    
    /// MIME type (e.g., "image/png", "text/plain")
    pub mime_type: Option<String>,
    
    /// Original filename
    pub filename: Option<String>,
    
    /// Upload timestamp (Unix seconds)
    pub uploaded_at: u64,
    
    /// User who uploaded this blob
    pub uploader: UserId,
    
    /// Thread this blob belongs to (if it's a message attachment)
    pub thread_id: Option<ThreadId>,
}

impl BlobMetadata {
    /// Create new blob metadata
    pub fn new(
        hash: BlobHash,
        size: u64,
        mime_type: Option<String>,
        filename: Option<String>,
        uploader: UserId,
        thread_id: Option<ThreadId>,
    ) -> Self {
        Self {
            hash,
            size,
            mime_type,
            filename,
            uploaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uploader,
            thread_id,
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize BlobMetadata: {}", e))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize BlobMetadata: {}", e))
    }
}

/// Message index entry (for thread/user message lists)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageIndex {
    /// Message ID
    pub message_id: MessageId,
    
    /// Blob hash (where the message content is stored)
    pub blob_hash: BlobHash,
    
    /// Message timestamp
    pub timestamp: u64,
    
    /// Author user ID
    pub author: UserId,
    
    /// Thread ID
    pub thread_id: ThreadId,
}

impl MessageIndex {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize MessageIndex: {}", e))
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize MessageIndex: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blob_metadata_roundtrip() -> anyhow::Result<()> {
        let hash = BlobHash::hash(b"test data");
        let uploader = UserId([1u8; 16]);
        
        let metadata = BlobMetadata::new(
            hash,
            1024,
            Some("text/plain".to_string()),
            Some("test.txt".to_string()),
            uploader,
            None,
        );
        
        let bytes = metadata.to_bytes()?;
        let deserialized = BlobMetadata::from_bytes(&bytes)?;
        
        assert_eq!(metadata.hash, deserialized.hash);
        assert_eq!(metadata.size, deserialized.size);
        assert_eq!(metadata.mime_type, deserialized.mime_type);
        
        Ok(())
    }
}
