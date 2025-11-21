/// Message indexing and retrieval
/// 
/// Manages RocksDB indices for fast message lookup by:
/// - Thread ID (chronological order)
/// - User ID (user's message history)
/// - Message ID (direct lookup)

use super::{BlobHash, Storage};
use crate::types::{MessageId, ThreadId, UserId};
use anyhow::{Context, Result};
use rocksdb::{IteratorMode, Direction};
use serde::{Serialize, Deserialize};

/// Metadata about a stored blob
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// Content-addressed hash of the blob
    pub hash: BlobHash,
    /// Original size in bytes
    pub size: u64,
    /// MIME type (e.g., "image/png", "text/plain")
    pub mime_type: Option<String>,
    /// Original filename
    pub filename: Option<String>,
    /// Unix timestamp when uploaded
    pub uploaded_at: u64,
    /// User who uploaded this blob
    pub uploader: UserId,
    /// Thread this blob belongs to (optional)
    pub thread_id: Option<ThreadId>,
}

impl Storage {
    /// Index a message (maps message_id -> blob_hash)
    pub fn index_message(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        author: UserId,
        timestamp: u64,
        blob_hash: BlobHash,
    ) -> Result<()> {
        let cf_thread = self.db.cf_handle(Self::CF_THREAD_MESSAGES)
            .context("Missing thread_messages column family")?;
        let cf_user = self.db.cf_handle(Self::CF_USER_MESSAGES)
            .context("Missing user_messages column family")?;
        let cf_refs = self.db.cf_handle(Self::CF_MESSAGE_REFS)
            .context("Missing message_refs column family")?;
        let cf_meta = self.db.cf_handle(Self::CF_BLOB_METADATA)
            .context("Missing blob_metadata column family")?;

        // Key formats:
        // thread_messages: "<thread_id_hex>:<timestamp>:<message_id_hex>" -> blob_hash
        // user_messages: "<user_id_hex>:<timestamp>:<message_id_hex>" -> blob_hash
        // message_refs: "<message_id_hex>" -> blob_hash
        // blob_metadata: "<blob_hash_hex>" -> BlobMetadata

        let thread_key = format!("{}:{}:{}", 
            thread_id.to_hex(), timestamp, message_id.to_hex());
        let user_key = format!("{}:{}:{}", 
            author.to_hex(), timestamp, message_id.to_hex());
        let msg_key = message_id.to_hex();
        let blob_key = blob_hash.to_hex();

        let hash_bytes = bincode::serialize(&blob_hash)
            .context("Failed to serialize blob hash")?;

        // Store metadata
        let metadata = BlobMetadata {
            hash: blob_hash,
            thread_id: Some(thread_id),
            uploader: author,
            uploaded_at: timestamp,
            size: 0, // Size can be updated later
            mime_type: None,
            filename: None,
        };
        let meta_bytes = bincode::serialize(&metadata)
            .context("Failed to serialize metadata")?;

        // Write all indices atomically
        let mut batch = rocksdb::WriteBatch::default();
        batch.put_cf(&cf_thread, thread_key.as_bytes(), &hash_bytes);
        batch.put_cf(&cf_user, user_key.as_bytes(), &hash_bytes);
        batch.put_cf(&cf_refs, msg_key.as_bytes(), &hash_bytes);
        batch.put_cf(&cf_meta, blob_key.as_bytes(), &meta_bytes);

        self.db.write(batch)
            .context("Failed to write message indices")?;

        tracing::debug!(
            message_id = %message_id,
            thread_id = %thread_id,
            author = %author,
            "Indexed message"
        );

        Ok(())
    }

    /// Get blob hash for a message ID
    pub fn get_message_blob(&self, message_id: &MessageId) -> Result<Option<BlobHash>> {
        let cf = self.db.cf_handle(Self::CF_MESSAGE_REFS)
            .context("Missing message_refs column family")?;

        let key = message_id.to_hex();
        let value = self.db.get_cf(&cf, key.as_bytes())
            .context("Failed to read message ref")?;

        match value {
            Some(bytes) => {
                let hash = bincode::deserialize(&bytes)
                    .context("Failed to deserialize blob hash")?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Get all message blob hashes in a thread (chronological order)
    pub fn get_thread_messages(&self, thread_id: &ThreadId) -> Result<Vec<(MessageId, BlobHash, u64)>> {
        let cf = self.db.cf_handle(Self::CF_THREAD_MESSAGES)
            .context("Missing thread_messages column family")?;

        let prefix = format!("{}:", thread_id.to_hex());
        let mut messages = Vec::new();

        // Iterate over all keys with this thread_id prefix
        let iter = self.db.iterator_cf(&cf, IteratorMode::From(prefix.as_bytes(), Direction::Forward));
        
        for item in iter {
            let (key, value) = item.context("Iterator error")?;
            let key_str = String::from_utf8_lossy(&key);
            
            // Check if still in our thread's prefix
            if !key_str.starts_with(&prefix) {
                break;
            }

            // Parse key: "<thread_id>:<timestamp>:<message_id>"
            let parts: Vec<&str> = key_str.split(':').collect();
            if parts.len() != 3 {
                continue;
            }

            let timestamp: u64 = parts[1].parse()
                .context("Invalid timestamp in key")?;
            let message_id = MessageId::from_hex(parts[2])
                .context("Invalid message ID in key")?;;

            let blob_hash: BlobHash = bincode::deserialize(&value)
                .context("Failed to deserialize blob hash")?;

            messages.push((message_id, blob_hash, timestamp));
        }

        Ok(messages)
    }

    /// Get all message blob hashes by a user (chronological order)
    pub fn get_user_messages(&self, user_id: &UserId, limit: usize) -> Result<Vec<(MessageId, BlobHash, u64)>> {
        let cf = self.db.cf_handle(Self::CF_USER_MESSAGES)
            .context("Missing user_messages column family")?;

        let prefix = format!("{}:", user_id.to_hex());
        let mut messages = Vec::new();

        let iter = self.db.iterator_cf(&cf, IteratorMode::From(prefix.as_bytes(), Direction::Forward));
        
        for item in iter.take(limit) {
            let (key, value) = item.context("Iterator error")?;
            let key_str = String::from_utf8_lossy(&key);
            
            if !key_str.starts_with(&prefix) {
                break;
            }

            let parts: Vec<&str> = key_str.split(':').collect();
            if parts.len() != 3 {
                continue;
            }

            let timestamp: u64 = parts[1].parse()
                .context("Invalid timestamp in key")?;
            let message_id = MessageId::from_hex(parts[2])
                .context("Invalid message ID in key")?;

            let blob_hash: BlobHash = bincode::deserialize(&value)
                .context("Failed to deserialize blob hash")?;

            messages.push((message_id, blob_hash, timestamp));
        }

        Ok(messages)
    }

    /// Get metadata for a blob
    pub fn get_blob_metadata(&self, blob_hash: &BlobHash) -> Result<Option<BlobMetadata>> {
        let cf = self.db.cf_handle(Self::CF_BLOB_METADATA)
            .context("Missing blob_metadata column family")?;

        let key = blob_hash.to_hex();
        let value = self.db.get_cf(&cf, key.as_bytes())
            .context("Failed to read blob metadata")?;

        match value {
            Some(bytes) => {
                let metadata = bincode::deserialize(&bytes)
                    .context("Failed to deserialize metadata")?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }
    
    /// Store or update blob metadata
    pub fn store_blob_metadata(&self, blob_hash: &BlobHash, metadata: &BlobMetadata) -> Result<()> {
        let cf = self.db.cf_handle(Self::CF_BLOB_METADATA)
            .context("Missing blob_metadata column family")?;
        
        let key = blob_hash.to_hex();
        let value = bincode::serialize(metadata)
            .context("Failed to serialize blob metadata")?;
        
        self.db.put_cf(&cf, key.as_bytes(), &value)
            .context("Failed to write blob metadata")?;
        
        tracing::debug!(
            blob_hash = %blob_hash.to_hex(),
            size = metadata.size,
            "Stored blob metadata"
        );
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_index_and_retrieve_message() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let message_id = MessageId::new();
        let author = UserId([1u8; 32]);
        let timestamp = 1234567890;
        let blob_hash = BlobHash::hash(b"test message");

        // Index message
        storage.index_message(thread_id, message_id, author, timestamp, blob_hash)?;

        // Retrieve by message ID
        let retrieved = storage.get_message_blob(&message_id)?;
        assert_eq!(retrieved, Some(blob_hash));

        Ok(())
    }

    #[test]
    fn test_get_thread_messages() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let author = UserId([1u8; 32]);

        // Add 3 messages to thread
        for i in 0..3 {
            let message_id = MessageId::new();
            let timestamp = 1000 + i;
            let blob_hash = BlobHash::hash(format!("message {}", i).as_bytes());

            storage.index_message(thread_id, message_id, author, timestamp, blob_hash)?;
        }

        // Retrieve all messages
        let messages = storage.get_thread_messages(&thread_id)?;
        assert_eq!(messages.len(), 3);

        // Verify chronological order
        for i in 0..2 {
            assert!(messages[i].2 <= messages[i + 1].2);
        }

        Ok(())
    }

    #[test]
    fn test_get_user_messages() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let user1 = UserId([1u8; 32]);
        let user2 = UserId([2u8; 32]);

        // User1 posts 5 messages
        for i in 0..5 {
            let message_id = MessageId::new();
            let timestamp = 2000 + i;
            let blob_hash = BlobHash::hash(format!("user1 msg {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user1, timestamp, blob_hash)?;
        }

        // User2 posts 2 messages
        for i in 0..2 {
            let message_id = MessageId::new();
            let timestamp = 3000 + i;
            let blob_hash = BlobHash::hash(format!("user2 msg {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user2, timestamp, blob_hash)?;
        }

        // Get user1's messages (limit 3)
        let user1_msgs = storage.get_user_messages(&user1, 3)?;
        assert_eq!(user1_msgs.len(), 3);

        // Get user2's messages (all)
        let user2_msgs = storage.get_user_messages(&user2, 100)?;
        assert_eq!(user2_msgs.len(), 2);

        Ok(())
    }

    #[test]
    fn test_blob_metadata() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let message_id = MessageId::new();
        let author = UserId([42u8; 32]);
        let timestamp = 9999;
        let blob_hash = BlobHash::hash(b"metadata test");

        // Index message
        storage.index_message(thread_id, message_id, author, timestamp, blob_hash)?;

        // Retrieve metadata
        let metadata = storage.get_blob_metadata(&blob_hash)?;
        assert!(metadata.is_some());

        let meta = metadata.unwrap();
        assert_eq!(meta.thread_id, Some(thread_id));
        assert_eq!(meta.uploader, author);
        assert_eq!(meta.uploaded_at, timestamp);

        Ok(())
    }
}
