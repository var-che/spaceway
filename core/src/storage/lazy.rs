///! Lazy loading and pagination for efficient message retrieval
///! 
///! Phase 3 Feature: Fetch blobs on demand rather than syncing everything eagerly

use anyhow::{Context, Result};
use super::{Storage, BlobHash};
use crate::types::{ThreadId, MessageId, UserId};

/// Thread preview with first N messages
#[derive(Debug, Clone)]
pub struct ThreadPreview {
    pub thread_id: ThreadId,
    pub message_count: usize,
    pub preview_messages: Vec<MessageId>,
    pub latest_timestamp: u64,
}

/// Pagination cursor for iterating through large threads
#[derive(Debug, Clone)]
pub struct MessageCursor {
    pub thread_id: ThreadId,
    pub last_timestamp: Option<u64>,
    pub last_message_id: Option<MessageId>,
    pub page_size: usize,
}

impl MessageCursor {
    pub fn new(thread_id: ThreadId, page_size: usize) -> Self {
        Self {
            thread_id,
            last_timestamp: None,
            last_message_id: None,
            page_size,
        }
    }
}

/// Page of messages returned from paginated query
#[derive(Debug, Clone)]
pub struct MessagePage {
    pub messages: Vec<(MessageId, BlobHash, u64)>,
    pub has_more: bool,
    pub cursor: Option<MessageCursor>,
}

impl Storage {
    /// Get thread preview (first N messages without loading blobs)
    ///
    /// This is much faster than loading all messages since it only reads indices.
    pub fn get_thread_preview(&self, thread_id: &ThreadId, limit: usize) -> Result<ThreadPreview> {
        let all_messages = self.get_thread_messages(thread_id)?;
        let messages: Vec<_> = all_messages.into_iter().take(limit).collect();
        
        let message_count = messages.len();
        let preview_messages: Vec<MessageId> = messages.iter().map(|(id, _, _)| *id).collect();
        let latest_timestamp = messages.last().map(|(_, _, ts)| *ts).unwrap_or(0);
        
        Ok(ThreadPreview {
            thread_id: *thread_id,
            message_count,
            preview_messages,
            latest_timestamp,
        })
    }
    
    /// Get a page of messages using cursor-based pagination
    ///
    /// This allows efficient iteration through large threads without loading everything.
    pub fn get_messages_page(&self, mut cursor: MessageCursor) -> Result<MessagePage> {
        let cf = self.db.cf_handle(Self::CF_THREAD_MESSAGES)
            .context("Missing thread_messages column family")?;
        
        let prefix = format!("{}:", cursor.thread_id.to_hex());
        let mut messages = Vec::new();
        let mut count = 0;
        let mut skip_until_cursor = cursor.last_timestamp.is_some();
        
        // Start iteration from the beginning or from cursor position
        let start_key = if let Some(ts) = cursor.last_timestamp {
            format!("{}:{}:", cursor.thread_id.to_hex(), ts)
        } else {
            prefix.clone()
        };
        
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item.context("Iterator error")?;
            let key_str = String::from_utf8_lossy(&key);
            
            // Check if still in our thread's prefix
            if !key_str.starts_with(&prefix) {
                break;
            }
            
            // Parse key: "<thread_id_hex>:<timestamp>:<message_id_hex>"
            let parts: Vec<&str> = key_str.split(':').collect();
            if parts.len() != 3 {
                continue;
            }
            
            let timestamp: u64 = parts[1].parse()
                .context("Invalid timestamp in key")?;
            let message_id = MessageId::from_hex(parts[2])
                .context("Invalid message ID in key")?;
            
            // Skip messages we've already seen (cursor position)
            if skip_until_cursor {
                if let Some(last_msg_id) = &cursor.last_message_id {
                    if message_id == *last_msg_id {
                        skip_until_cursor = false;
                        continue; // Skip the cursor message itself
                    }
                }
                continue;
            }
            
            let blob_hash: BlobHash = bincode::deserialize(&value)
                .context("Failed to deserialize blob hash")?;
            
            messages.push((message_id, blob_hash, timestamp));
            count += 1;
            
            // Update cursor for next page
            cursor.last_timestamp = Some(timestamp);
            cursor.last_message_id = Some(message_id);
            
            // Stop if we've reached page size
            if count >= cursor.page_size {
                break;
            }
        }
        
        // Check if there are more messages after this page
        let has_more = count == cursor.page_size;
        
        let next_cursor = if has_more {
            Some(cursor)
        } else {
            None
        };
        
        Ok(MessagePage {
            messages,
            has_more,
            cursor: next_cursor,
        })
    }
    
    /// Get total message count for a thread (without loading blobs)
    pub fn get_thread_message_count(&self, thread_id: &ThreadId) -> Result<usize> {
        let messages = self.get_thread_messages(thread_id)?;
        Ok(messages.len())
    }
    
    /// Get recent messages from a user (with pagination)
    pub fn get_user_messages_page(&self, user_id: &UserId, page_size: usize, offset: usize) -> Result<Vec<(MessageId, BlobHash, u64)>> {
        let all_messages = self.get_user_messages(user_id, usize::MAX)?;
        
        let start = offset.min(all_messages.len());
        let end = (offset + page_size).min(all_messages.len());
        
        Ok(all_messages[start..end].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::types::UserId;
    
    #[test]
    fn test_thread_preview() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let thread_id = ThreadId::new();
        let user_id = UserId::new();
        
        // Index 10 messages
        for i in 0..10 {
            let message_id = MessageId::new();
            let blob_hash = BlobHash::hash(format!("message {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user_id, 1000 + i, blob_hash)?;
        }
        
        // Get preview of first 5
        let preview = storage.get_thread_preview(&thread_id, 5)?;
        
        assert_eq!(preview.thread_id, thread_id);
        assert_eq!(preview.message_count, 5);
        assert_eq!(preview.preview_messages.len(), 5);
        assert!(preview.latest_timestamp >= 1000);
        
        Ok(())
    }
    
    #[test]
    fn test_pagination() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let thread_id = ThreadId::new();
        let user_id = UserId::new();
        
        // Index 25 messages
        for i in 0..25 {
            let message_id = MessageId::new();
            let blob_hash = BlobHash::hash(format!("message {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user_id, 1000 + i, blob_hash)?;
        }
        
        // Page through 10 at a time
        let mut cursor = MessageCursor::new(thread_id, 10);
        let mut total_messages = 0;
        let mut page_count = 0;
        
        loop {
            let page = storage.get_messages_page(cursor.clone())?;
            total_messages += page.messages.len();
            page_count += 1;
            
            if !page.has_more {
                break;
            }
            
            cursor = page.cursor.expect("Should have cursor for next page");
        }
        
        assert_eq!(total_messages, 25, "Should retrieve all messages across pages");
        assert_eq!(page_count, 3, "Should have 3 pages (10, 10, 5)");
        
        Ok(())
    }
    
    #[test]
    fn test_message_count() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let thread_id = ThreadId::new();
        let user_id = UserId::new();
        
        // Initially 0
        let count = storage.get_thread_message_count(&thread_id)?;
        assert_eq!(count, 0);
        
        // Add 5 messages
        for i in 0..5 {
            let message_id = MessageId::new();
            let blob_hash = BlobHash::hash(format!("message {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user_id, 1000 + i, blob_hash)?;
        }
        
        let count = storage.get_thread_message_count(&thread_id)?;
        assert_eq!(count, 5);
        
        Ok(())
    }
    
    #[test]
    fn test_user_messages_pagination() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let thread_id = ThreadId::new();
        let user_id = UserId::new();
        
        // Index 20 messages
        for i in 0..20 {
            let message_id = MessageId::new();
            let blob_hash = BlobHash::hash(format!("user message {}", i).as_bytes());
            storage.index_message(thread_id, message_id, user_id, 1000 + i, blob_hash)?;
        }
        
        // Get page 1 (messages 0-9)
        let page1 = storage.get_user_messages_page(&user_id, 10, 0)?;
        assert_eq!(page1.len(), 10);
        
        // Get page 2 (messages 10-19)
        let page2 = storage.get_user_messages_page(&user_id, 10, 10)?;
        assert_eq!(page2.len(), 10);
        
        // Get page 3 (no more messages)
        let page3 = storage.get_user_messages_page(&user_id, 10, 20)?;
        assert_eq!(page3.len(), 0);
        
        Ok(())
    }
}
