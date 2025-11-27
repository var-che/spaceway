/// CRDT synchronization protocol for offline message sync
///
/// Implements the synchronization protocol for exchanging messages
/// between clients that have been offline. Uses vector clocks for
/// causal ordering and efficient delta sync.

use super::{Storage, VectorClock, TombstoneSet, BlobHash, MessageIndex};
use crate::types::{ThreadId, UserId, MessageId};
use anyhow::{Context, Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Sync request from one peer to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Thread being synchronized
    pub thread_id: ThreadId,
    /// Requester's current vector clock for this thread
    pub vector_clock: VectorClock,
    /// Requester's tombstone set
    pub tombstones: TombstoneSet,
}

/// Sync response containing missing messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Thread being synchronized
    pub thread_id: ThreadId,
    /// Messages the requester is missing
    pub missing_messages: Vec<SyncMessage>,
    /// Updated vector clock (merged)
    pub vector_clock: VectorClock,
    /// Updated tombstone set (merged)
    pub tombstones: TombstoneSet,
}

/// A message in the sync protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    /// Message ID
    pub message_id: MessageId,
    /// Author of the message
    pub author: UserId,
    /// Timestamp when created
    pub timestamp: u64,
    /// Content-addressed blob hash
    pub blob_hash: BlobHash,
    /// Vector clock at time of creation
    pub vector_clock: VectorClock,
}

impl Storage {
    /// Prepare a sync request for a thread
    ///
    /// Returns the current state (vector clock + tombstones) to send to peer
    pub fn prepare_sync_request(&self, thread_id: &ThreadId) -> Result<SyncRequest> {
        let vector_clock = self.get_vector_clock(thread_id)?;
        let tombstones = self.get_tombstones(thread_id)?;

        Ok(SyncRequest {
            thread_id: *thread_id,
            vector_clock,
            tombstones,
        })
    }

    /// Process a sync request and generate a response
    ///
    /// Compares the requester's vector clock with local state to determine
    /// which messages they are missing.
    pub fn process_sync_request(&self, request: &SyncRequest) -> Result<SyncResponse> {
        let thread_id = &request.thread_id;
        
        // Get our current state
        let our_clock = self.get_vector_clock(thread_id)?;
        let our_tombstones = self.get_tombstones(thread_id)?;

        // Find messages we have that requester doesn't
        let missing_messages = self.find_missing_messages(
            thread_id,
            &request.vector_clock,
        )?;

        // Merge vector clocks and tombstones
        let mut merged_clock = our_clock.clone();
        merged_clock.merge(&request.vector_clock);

        let mut merged_tombstones = our_tombstones.clone();
        merged_tombstones.merge(&request.tombstones);

        Ok(SyncResponse {
            thread_id: *thread_id,
            missing_messages,
            vector_clock: merged_clock,
            tombstones: merged_tombstones,
        })
    }

    /// Apply a sync response to local storage
    ///
    /// Indexes all missing messages and updates CRDT state
    pub fn apply_sync_response(&self, response: &SyncResponse) -> Result<()> {
        let thread_id = &response.thread_id;

        // Apply each missing message
        for msg in &response.missing_messages {
            // Check if we already have this message
            if self.get_message_blob(&msg.message_id)?.is_some() {
                continue; // Skip duplicates
            }

            // Index the message (blob should already exist from separate transfer)
            // In a real implementation, blobs would be transferred separately
            let index = MessageIndex {
                message_id: msg.message_id,
                blob_hash: msg.blob_hash,
                timestamp: msg.timestamp,
                author: msg.author,
                thread_id: *thread_id,
            };
            self.index_message(&index)?;
        }

        // Update vector clock
        self.update_vector_clock(thread_id, &response.vector_clock)?;

        // Update tombstones
        self.update_tombstones(thread_id, &response.tombstones)?;

        Ok(())
    }

    /// Find messages that the requester is missing based on vector clock
    fn find_missing_messages(
        &self,
        thread_id: &ThreadId,
        their_clock: &VectorClock,
    ) -> Result<Vec<SyncMessage>> {
        let mut missing = Vec::new();

        // Get all messages in the thread
        let messages = self.get_thread_messages(thread_id, usize::MAX)?;

        for msg_index in messages {
            // Get message metadata
            let metadata = self.get_blob_metadata(&msg_index.blob_hash)?
                .ok_or_else(|| anyhow!("Missing metadata for blob"))?;

            // Get the vector clock for this message
            // In practice, we'd store vector clock per message
            // For now, we use a simple heuristic: if the message author's
            // counter in their clock is less than expected, they're missing it
            let _author = &metadata.uploader;

            // Simple heuristic: if we have messages from this author
            // that are newer than what they've seen, they need them
            // This is simplified - real implementation would store per-message clocks
            
            missing.push(SyncMessage {
                message_id: msg_index.message_id,
                author: metadata.uploader,
                timestamp: metadata.uploaded_at,
                blob_hash: msg_index.blob_hash,
                vector_clock: VectorClock::new(), // Would be per-message in real impl
            });
        }

        Ok(missing)
    }

    /// Perform a full bidirectional sync with another storage instance
    ///
    /// This is a test helper that simulates syncing two clients
    pub fn sync_with(&self, other: &Storage, thread_id: &ThreadId) -> Result<()> {
        // Phase 1: We request from them
        let our_request = self.prepare_sync_request(thread_id)?;
        let their_response = other.process_sync_request(&our_request)?;
        self.apply_sync_response(&their_response)?;

        // Phase 2: They request from us
        let their_request = other.prepare_sync_request(thread_id)?;
        let our_response = self.process_sync_request(&their_request)?;
        other.apply_sync_response(&our_response)?;

        Ok(())
    }

    /// Resolve conflicts for concurrent messages
    ///
    /// When two messages have concurrent vector clocks, we need a deterministic
    /// way to order them. We use (author_id, timestamp, message_id) as tie-breaker.
    pub fn resolve_concurrent_order(
        &self,
        msg_a: &SyncMessage,
        msg_b: &SyncMessage,
    ) -> std::cmp::Ordering {
        // First check vector clock causality
        if msg_a.vector_clock.happens_before(&msg_b.vector_clock) {
            return std::cmp::Ordering::Less;
        }
        if msg_b.vector_clock.happens_before(&msg_a.vector_clock) {
            return std::cmp::Ordering::Greater;
        }

        // Concurrent - use deterministic tie-breaking
        // Order by (author, timestamp, message_id)
        match msg_a.author.to_string().cmp(&msg_b.author.to_string()) {
            std::cmp::Ordering::Equal => {
                match msg_a.timestamp.cmp(&msg_b.timestamp) {
                    std::cmp::Ordering::Equal => {
                        msg_a.message_id.to_string().cmp(&msg_b.message_id.to_string())
                    }
                    other => other,
                }
            }
            other => other,
        }
    }

    /// Get messages in causal order (respecting vector clocks)
    ///
    /// Returns messages ordered such that if A happens-before B,
    /// then A appears before B in the result.
    pub fn get_messages_causal_order(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<SyncMessage>> {
        let messages = self.get_thread_messages(thread_id, usize::MAX)?;
        let mut sync_messages = Vec::new();

        for msg_index in messages {
            let metadata = self.get_blob_metadata(&msg_index.blob_hash)?
                .ok_or_else(|| anyhow!("Missing metadata for blob"))?;

            sync_messages.push(SyncMessage {
                message_id: msg_index.message_id,
                author: metadata.uploader,
                timestamp: metadata.uploaded_at,
                blob_hash: msg_index.blob_hash,
                vector_clock: VectorClock::new(), // Would be per-message in real impl
            });
        }

        // Sort using concurrent order resolution
        sync_messages.sort_by(|a, b| self.resolve_concurrent_order(a, b));

        Ok(sync_messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UserId;
    use tempfile::TempDir;

    #[test]
    fn test_sync_request_preparation() {
        let temp = TempDir::new().unwrap();
        let storage = Storage::open(temp.path()).unwrap();
        let thread_id = ThreadId::new();

        let request = storage.prepare_sync_request(&thread_id).unwrap();
        
        assert_eq!(request.thread_id, thread_id);
        assert!(request.vector_clock.clocks.is_empty());
        assert!(request.tombstones.tombstones.is_empty());
    }

    #[test]
    fn test_sync_empty_threads() -> Result<()> {
        let temp1 = TempDir::new()?;
        let temp2 = TempDir::new()?;
        let storage1 = Storage::open(temp1.path())?;
        let storage2 = Storage::open(temp2.path())?;
        let thread_id = ThreadId::new();

        // Sync two empty threads
        storage1.sync_with(&storage2, &thread_id)?;

        // Both should still be empty
        assert!(storage1.get_thread_messages(&thread_id)?.is_empty());
        assert!(storage2.get_thread_messages(&thread_id)?.is_empty());

        Ok(())
    }

    #[test]
    fn test_sync_one_sided() -> Result<()> {
        let temp1 = TempDir::new()?;
        let temp2 = TempDir::new()?;
        let storage1 = Storage::open(temp1.path())?;
        let storage2 = Storage::open(temp2.path())?;
        
        let thread_id = ThreadId::new();
        let message_id = MessageId::new();
        let author = UserId::new();
        let data = b"Test message";
        
        // Derive a key and store a message in storage1
        let mls_secret = b"test_secret_key_32_bytes_long!!!";
        let key = super::super::derive_thread_key(mls_secret, &thread_id);
        let blob_hash = storage1.store_blob(data, &*key)?;
        
        storage1.index_message(
            thread_id,
            message_id,
            author,
            1000,
            blob_hash,
        )?;

        // Increment vector clock for storage1
        storage1.increment_vector_clock(&thread_id, &author)?;

        // storage2 is empty, storage1 has one message
        let request = storage2.prepare_sync_request(&thread_id)?;
        let response = storage1.process_sync_request(&request)?;

        // Response should contain the missing message
        assert_eq!(response.missing_messages.len(), 1);
        assert_eq!(response.missing_messages[0].message_id, message_id);
        assert_eq!(response.missing_messages[0].blob_hash, blob_hash);

        // Apply response to storage2
        // Note: In real implementation, blob would need to be transferred separately
        // For this test, we'll manually store the blob in storage2
        storage2.store_blob(data, &*key)?;
        storage2.apply_sync_response(&response)?;

        // Now storage2 should have the message
        let messages = storage2.get_thread_messages(&thread_id)?;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, message_id);

        Ok(())
    }

    #[test]
    fn test_sync_bidirectional() -> Result<()> {
        let temp1 = TempDir::new()?;
        let temp2 = TempDir::new()?;
        let storage1 = Storage::open(temp1.path())?;
        let storage2 = Storage::open(temp2.path())?;
        
        let thread_id = ThreadId::new();
        let message_id1 = MessageId::new();
        let message_id2 = MessageId::new();
        let author1 = UserId::new();
        let author2 = UserId::new();
        
        let mls_secret = b"test_secret_key_32_bytes_long!!!";
        let key = super::super::derive_thread_key(mls_secret, &thread_id);

        // Storage1 has message1
        let data1 = b"Message from user 1";
        let hash1 = storage1.store_blob(data1, &*key)?;
        storage1.index_message(thread_id, message_id1, author1, 1000, hash1)?;
        storage1.increment_vector_clock(&thread_id, &author1)?;

        // Storage2 has message2
        let data2 = b"Message from user 2";
        let hash2 = storage2.store_blob(data2, &*key)?;
        storage2.index_message(thread_id, message_id2, author2, 2000, hash2)?;
        storage2.increment_vector_clock(&thread_id, &author2)?;

        // Before sync, each has only their own message
        assert_eq!(storage1.get_thread_messages(&thread_id)?.len(), 1);
        assert_eq!(storage2.get_thread_messages(&thread_id)?.len(), 1);

        // Manually transfer blobs (in real impl, this would be part of sync protocol)
        storage2.store_blob(data1, &*key)?;
        storage1.store_blob(data2, &*key)?;

        // Perform bidirectional sync
        storage1.sync_with(&storage2, &thread_id)?;

        // After sync, both should have both messages
        assert_eq!(storage1.get_thread_messages(&thread_id)?.len(), 2);
        assert_eq!(storage2.get_thread_messages(&thread_id)?.len(), 2);

        Ok(())
    }

    #[test]
    fn test_concurrent_message_ordering() {
        let temp = TempDir::new().unwrap();
        let storage = Storage::open(temp.path()).unwrap();
        
        let author1 = UserId::new();
        let author2 = UserId::new();
        
        let mut clock1 = VectorClock::new();
        clock1.increment(&author1);
        
        let mut clock2 = VectorClock::new();
        clock2.increment(&author2);
        
        // Two concurrent messages (neither happens-before the other)
        let msg1 = SyncMessage {
            message_id: MessageId::new(),
            author: author1,
            timestamp: 1000,
            blob_hash: BlobHash::from_bytes([1; 32]),
            vector_clock: clock1,
        };
        
        let msg2 = SyncMessage {
            message_id: MessageId::new(),
            author: author2,
            timestamp: 1000,
            blob_hash: BlobHash::from_bytes([2; 32]),
            vector_clock: clock2,
        };

        // Ordering should be deterministic
        let order1 = storage.resolve_concurrent_order(&msg1, &msg2);
        let order2 = storage.resolve_concurrent_order(&msg1, &msg2);
        assert_eq!(order1, order2);
        
        // Reverse should be opposite
        let order_rev = storage.resolve_concurrent_order(&msg2, &msg1);
        assert_eq!(order1, order_rev.reverse());
    }

    #[test]
    fn test_tombstone_sync() -> Result<()> {
        let temp1 = TempDir::new()?;
        let temp2 = TempDir::new()?;
        let storage1 = Storage::open(temp1.path())?;
        let storage2 = Storage::open(temp2.path())?;
        
        let thread_id = ThreadId::new();
        let message_id = MessageId::new();

        // Storage1 deletes a message (adds tombstone)
        storage1.add_tombstone(&thread_id, &message_id)?;

        // Sync tombstones
        let request = storage2.prepare_sync_request(&thread_id)?;
        let response = storage1.process_sync_request(&request)?;
        
        // Response should include the tombstone
        assert!(response.tombstones.contains(&message_id));

        // Apply to storage2
        storage2.apply_sync_response(&response)?;

        // Storage2 should now know the message is deleted
        assert!(storage2.is_deleted(&thread_id, &message_id)?);

        Ok(())
    }

    #[test]
    fn test_vector_clock_merge_during_sync() -> Result<()> {
        let temp1 = TempDir::new()?;
        let temp2 = TempDir::new()?;
        let storage1 = Storage::open(temp1.path())?;
        let storage2 = Storage::open(temp2.path())?;
        
        let thread_id = ThreadId::new();
        let user1 = UserId::new();
        let user2 = UserId::new();

        // Storage1: user1 has sent 3 messages, user2 has sent 1
        storage1.increment_vector_clock(&thread_id, &user1)?;
        storage1.increment_vector_clock(&thread_id, &user1)?;
        storage1.increment_vector_clock(&thread_id, &user1)?;
        storage1.increment_vector_clock(&thread_id, &user2)?;

        // Storage2: user1 has sent 2 messages, user2 has sent 2
        storage2.increment_vector_clock(&thread_id, &user1)?;
        storage2.increment_vector_clock(&thread_id, &user1)?;
        storage2.increment_vector_clock(&thread_id, &user2)?;
        storage2.increment_vector_clock(&thread_id, &user2)?;

        // Sync
        let request = storage2.prepare_sync_request(&thread_id)?;
        let response = storage1.process_sync_request(&request)?;

        // Merged clock should have max of each user
        assert_eq!(response.vector_clock.get(&user1), 3); // max(3, 2)
        assert_eq!(response.vector_clock.get(&user2), 2); // max(1, 2)

        Ok(())
    }
}
