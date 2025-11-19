//! Thread management
//!
//! A Thread is a multi-message discussion within a Channel.
//! Threads contain Messages and support replies.

use crate::types::*;
use crate::crdt::{CrdtOp, OpType, OpPayload, Hlc, HoldbackQueue, OpValidator, ValidationResult};
use crate::{Error, Result};
use std::collections::HashMap;

/// A Thread (multi-message discussion)
#[derive(Debug, Clone)]
pub struct Thread {
    /// Unique identifier
    pub id: ThreadId,
    
    /// Parent Space
    pub space_id: SpaceId,
    
    /// Parent Channel
    pub channel_id: ChannelId,
    
    /// Optional title
    pub title: Option<String>,
    
    /// First message ID (thread starter)
    pub first_message_id: MessageId,
    
    /// Creator
    pub creator: UserId,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Whether the thread is resolved/closed
    pub resolved: bool,
    
    /// Number of messages (cached)
    pub message_count: u64,
}

impl Thread {
    /// Create a new Thread
    pub fn new(
        id: ThreadId,
        space_id: SpaceId,
        channel_id: ChannelId,
        title: Option<String>,
        first_message_id: MessageId,
        creator: UserId,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            space_id,
            channel_id,
            title,
            first_message_id,
            creator,
            created_at,
            resolved: false,
            message_count: 1, // Includes first message
        }
    }
    
    /// Mark thread as resolved
    pub fn resolve(&mut self) {
        self.resolved = true;
    }
    
    /// Mark thread as unresolved
    pub fn unresolve(&mut self) {
        self.resolved = false;
    }
    
    /// Update thread title
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }
    
    /// Increment message count
    pub fn add_message(&mut self) {
        self.message_count += 1;
    }
}

/// A Message within a Thread
#[derive(Debug, Clone)]
pub struct Message {
    /// Unique identifier
    pub id: MessageId,
    
    /// Parent Thread
    pub thread_id: ThreadId,
    
    /// Message content (plain text for now)
    pub content: String,
    
    /// Author
    pub author: UserId,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Last edit timestamp
    pub edited_at: Option<u64>,
    
    /// Whether the message is deleted
    pub deleted: bool,
}

impl Message {
    /// Create a new Message
    pub fn new(
        id: MessageId,
        thread_id: ThreadId,
        content: String,
        author: UserId,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            thread_id,
            content,
            author,
            created_at,
            edited_at: None,
            deleted: false,
        }
    }
    
    /// Edit the message content
    pub fn edit(&mut self, new_content: String, timestamp: u64) {
        self.content = new_content;
        self.edited_at = Some(timestamp);
    }
    
    /// Mark message as deleted
    pub fn delete(&mut self) {
        self.deleted = true;
    }
}

/// Manages Thread and Message state and operations
pub struct ThreadManager {
    /// All threads indexed by ID
    threads: HashMap<ThreadId, Thread>,
    
    /// Threads by Channel
    channel_threads: HashMap<ChannelId, Vec<ThreadId>>,
    
    /// All messages indexed by ID
    messages: HashMap<MessageId, Message>,
    
    /// Messages by Thread
    thread_messages: HashMap<ThreadId, Vec<MessageId>>,
    
    /// CRDT operation validator
    validator: OpValidator,
    
    /// Holdback queue for out-of-order operations
    holdback: HoldbackQueue,
    
    /// HLC generator
    hlc: Hlc,
    
    /// All operations (for persistence)
    operations: HashMap<OpId, CrdtOp>,
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            threads: HashMap::new(),
            channel_threads: HashMap::new(),
            messages: HashMap::new(),
            thread_messages: HashMap::new(),
            validator: OpValidator::new(),
            holdback: HoldbackQueue::new(),
            hlc: Hlc::now(),
            operations: HashMap::new(),
        }
    }
    
    /// Create a new Thread
    pub fn create_thread(
        &mut self,
        thread_id: ThreadId,
        space_id: SpaceId,
        channel_id: ChannelId,
        title: Option<String>,
        first_message_content: String,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        // Check if thread already exists
        if self.threads.contains_key(&thread_id) {
            return Err(Error::AlreadyExists(format!("Thread {:?} already exists", thread_id)));
        }
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Generate first message ID
        let first_message_id = MessageId(uuid::Uuid::new_v4());
        
        // Create Thread
        let thread = Thread::new(
            thread_id,
            space_id,
            channel_id,
            title.clone(),
            first_message_id,
            creator,
            current_time,
        );
        
        // Create first Message
        let message = Message::new(
            first_message_id,
            thread_id,
            first_message_content.clone(),
            creator,
            current_time,
        );
        
        // Create CRDT operation
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: Some(channel_id),
            thread_id: Some(thread_id),
            op_type: OpType::CreateThread(OpPayload::CreateThread {
                title,
                first_message: first_message_content,
            }),
            prev_ops: vec![],
            author: creator,
            epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(creator_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        self.threads.insert(thread_id, thread);
        self.channel_threads
            .entry(channel_id)
            .or_insert_with(Vec::new)
            .push(thread_id);
        
        self.messages.insert(first_message_id, message);
        self.thread_messages
            .entry(thread_id)
            .or_insert_with(Vec::new)
            .push(first_message_id);
        
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Process an incoming CreateThread operation
    pub fn process_create_thread(&mut self, op: &CrdtOp) -> Result<()> {
        match self.validator.validate(op, &self.operations) {
            ValidationResult::Accept => {
                if let OpType::CreateThread(OpPayload::CreateThread { title, first_message }) = &op.op_type {
                    let thread_id = op.thread_id
                        .ok_or_else(|| Error::InvalidOperation("Missing thread_id".to_string()))?;
                    let channel_id = op.channel_id
                        .ok_or_else(|| Error::InvalidOperation("Missing channel_id".to_string()))?;
                    
                    let first_message_id = MessageId(uuid::Uuid::new_v4());
                    
                    let thread = Thread::new(
                        thread_id,
                        op.space_id,
                        channel_id,
                        title.clone(),
                        first_message_id,
                        op.author,
                        op.timestamp,
                    );
                    
                    let message = Message::new(
                        first_message_id,
                        thread_id,
                        first_message.clone(),
                        op.author,
                        op.timestamp,
                    );
                    
                    self.threads.insert(thread_id, thread);
                    self.channel_threads
                        .entry(channel_id)
                        .or_insert_with(Vec::new)
                        .push(thread_id);
                    
                    self.messages.insert(first_message_id, message);
                    self.thread_messages
                        .entry(thread_id)
                        .or_insert_with(Vec::new)
                        .push(first_message_id);
                    
                    self.operations.insert(op.op_id, op.clone());
                    self.validator.apply_op(op);
                    self.hlc.update(op.hlc);
                    
                    Ok(())
                } else {
                    Err(Error::InvalidOperation("Expected CreateThread operation".to_string()))
                }
            }
            ValidationResult::Buffered(deps) => {
                self.holdback.buffer(op.clone(), deps, op.timestamp)
                    .map_err(|e| Error::Storage(e))?;
                Ok(())
            }
            ValidationResult::Reject(reason) => {
                Err(Error::InvalidOperation(format!("Operation rejected: {:?}", reason)))
            }
        }
    }
    
    /// Post a message to a Thread
    pub fn post_message(
        &mut self,
        message_id: MessageId,
        thread_id: ThreadId,
        content: String,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        // Check thread exists
        let thread = self.threads.get_mut(&thread_id)
            .ok_or_else(|| Error::NotFound(format!("Thread {:?} not found", thread_id)))?;
        
        let space_id = thread.space_id;
        let channel_id = thread.channel_id;
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Create Message
        let message = Message::new(
            message_id,
            thread_id,
            content.clone(),
            author,
            current_time,
        );
        
        // Create CRDT operation
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: Some(channel_id),
            thread_id: Some(thread_id),
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id,
                content,
            }),
            prev_ops: vec![],
            author,
            epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(author_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        self.messages.insert(message_id, message);
        self.thread_messages
            .entry(thread_id)
            .or_insert_with(Vec::new)
            .push(message_id);
        thread.add_message();
        
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Edit a message
    pub fn edit_message(
        &mut self,
        message_id: MessageId,
        new_content: String,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        let message = self.messages.get_mut(&message_id)
            .ok_or_else(|| Error::NotFound(format!("Message {:?} not found", message_id)))?;
        
        // Check author matches
        if message.author != author {
            return Err(Error::Permission("Only author can edit message".to_string()));
        }
        
        let thread = self.threads.get(&message.thread_id)
            .ok_or_else(|| Error::NotFound(format!("Thread {:?} not found", message.thread_id)))?;
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id: thread.space_id,
            channel_id: Some(thread.channel_id),
            thread_id: Some(message.thread_id),
            op_type: OpType::EditMessage(OpPayload::EditMessage {
                message_id,
                new_content: new_content.clone(),
            }),
            prev_ops: vec![],
            author,
            epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(author_keypair.sign(&signing_bytes).0);
        
        message.edit(new_content, current_time);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Get a Thread by ID
    pub fn get_thread(&self, thread_id: &ThreadId) -> Option<&Thread> {
        self.threads.get(thread_id)
    }
    
    /// Get all Threads in a Channel
    pub fn list_threads(&self, channel_id: &ChannelId) -> Vec<&Thread> {
        self.channel_threads
            .get(channel_id)
            .map(|ids| ids.iter().filter_map(|id| self.threads.get(id)).collect())
            .unwrap_or_default()
    }
    
    /// Get a Message by ID
    pub fn get_message(&self, message_id: &MessageId) -> Option<&Message> {
        self.messages.get(message_id)
    }
    
    /// Get all Messages in a Thread
    pub fn list_messages(&self, thread_id: &ThreadId) -> Vec<&Message> {
        self.thread_messages
            .get(thread_id)
            .map(|ids| ids.iter().filter_map(|id| self.messages.get(id)).collect())
            .unwrap_or_default()
    }
}

impl Default for ThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_thread() {
        let mut manager = ThreadManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        let thread_id = ThreadId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        let result = manager.create_thread(
            thread_id,
            space_id,
            channel_id,
            Some("Bug Report".to_string()),
            "Found a bug in the UI".to_string(),
            creator,
            &creator_keypair,
            EpochId(0),
        );
        
        assert!(result.is_ok());
        
        let thread = manager.get_thread(&thread_id);
        assert!(thread.is_some());
        
        let thread = thread.unwrap();
        assert_eq!(thread.title, Some("Bug Report".to_string()));
        assert_eq!(thread.creator, creator);
        assert_eq!(thread.message_count, 1);
        
        let messages = manager.list_messages(&thread_id);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Found a bug in the UI");
    }
    
    #[test]
    fn test_post_message() {
        let mut manager = ThreadManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        let thread_id = ThreadId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        manager.create_thread(
            thread_id,
            space_id,
            channel_id,
            None,
            "First message".to_string(),
            creator,
            &creator_keypair,
            EpochId(0),
        ).unwrap();
        
        let message_id = MessageId(uuid::Uuid::new_v4());
        let result = manager.post_message(
            message_id,
            thread_id,
            "Second message".to_string(),
            creator,
            &creator_keypair,
            EpochId(0),
        );
        
        assert!(result.is_ok());
        
        let thread = manager.get_thread(&thread_id).unwrap();
        assert_eq!(thread.message_count, 2);
        
        let messages = manager.list_messages(&thread_id);
        assert_eq!(messages.len(), 2);
    }
    
    #[test]
    fn test_edit_message() {
        let mut manager = ThreadManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        let thread_id = ThreadId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        manager.create_thread(
            thread_id,
            space_id,
            channel_id,
            None,
            "Original content".to_string(),
            creator,
            &creator_keypair,
            EpochId(0),
        ).unwrap();
        
        let messages = manager.list_messages(&thread_id);
        let message_id = messages[0].id;
        
        let result = manager.edit_message(
            message_id,
            "Edited content".to_string(),
            creator,
            &creator_keypair,
            EpochId(0),
        );
        
        assert!(result.is_ok());
        
        let message = manager.get_message(&message_id).unwrap();
        assert_eq!(message.content, "Edited content");
        assert!(message.edited_at.is_some());
    }
}
