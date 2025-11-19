//! Channel management
//!
//! A Channel is a text communication container within a Space.
//! Channels can have Threads (multi-message discussions).

use crate::types::*;
use crate::crdt::{CrdtOp, OpType, OpPayload, Hlc, HoldbackQueue, OpValidator, ValidationResult};
use crate::{Error, Result};
use std::collections::HashMap;

/// A Channel (text communication container)
#[derive(Debug, Clone)]
pub struct Channel {
    /// Unique identifier
    pub id: ChannelId,
    
    /// Parent Space
    pub space_id: SpaceId,
    
    /// Display name
    pub name: String,
    
    /// Optional description/topic
    pub description: Option<String>,
    
    /// Creator
    pub creator: UserId,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Whether the channel is archived
    pub archived: bool,
}

impl Channel {
    /// Create a new Channel
    pub fn new(
        id: ChannelId,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        creator: UserId,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            space_id,
            name,
            description,
            creator,
            created_at,
            archived: false,
        }
    }
    
    /// Archive the channel
    pub fn archive(&mut self) {
        self.archived = true;
    }
    
    /// Unarchive the channel
    pub fn unarchive(&mut self) {
        self.archived = false;
    }
    
    /// Update the channel name
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
    
    /// Update the channel description
    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
    }
}

/// Manages Channel state and operations
pub struct ChannelManager {
    /// All channels indexed by ID
    channels: HashMap<ChannelId, Channel>,
    
    /// Channels by Space
    space_channels: HashMap<SpaceId, Vec<ChannelId>>,
    
    /// CRDT operation validator
    validator: OpValidator,
    
    /// Holdback queue for out-of-order operations
    holdback: HoldbackQueue,
    
    /// HLC generator
    hlc: Hlc,
    
    /// All operations (for persistence)
    operations: HashMap<OpId, CrdtOp>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            space_channels: HashMap::new(),
            validator: OpValidator::new(),
            holdback: HoldbackQueue::new(),
            hlc: Hlc::now(),
            operations: HashMap::new(),
        }
    }
    
    /// Create a new Channel
    pub fn create_channel(
        &mut self,
        channel_id: ChannelId,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        // Check if channel already exists
        if self.channels.contains_key(&channel_id) {
            return Err(Error::AlreadyExists(format!("Channel {:?} already exists", channel_id)));
        }
        
        // Create Channel
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let channel = Channel::new(
            channel_id,
            space_id,
            name.clone(),
            description.clone(),
            creator,
            current_time,
        );
        
        // Create CRDT operation
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: Some(channel_id),
            thread_id: None,
            op_type: OpType::CreateChannel(OpPayload::CreateChannel {
                name,
                description,
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
        self.channels.insert(channel_id, channel);
        self.space_channels
            .entry(space_id)
            .or_insert_with(Vec::new)
            .push(channel_id);
        
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Process an incoming CreateChannel operation
    pub fn process_create_channel(&mut self, op: &CrdtOp) -> Result<()> {
        // Validate the operation
        match self.validator.validate(op, &self.operations) {
            ValidationResult::Accept => {
                // Extract channel details
                if let OpType::CreateChannel(OpPayload::CreateChannel { name, description }) = &op.op_type {
                    let channel_id = op.channel_id
                        .ok_or_else(|| Error::InvalidOperation("Missing channel_id".to_string()))?;
                    
                    let channel = Channel::new(
                        channel_id,
                        op.space_id,
                        name.clone(),
                        description.clone(),
                        op.author,
                        op.timestamp,
                    );
                    
                    self.channels.insert(channel_id, channel);
                    self.space_channels
                        .entry(op.space_id)
                        .or_insert_with(Vec::new)
                        .push(channel_id);
                    
                    self.operations.insert(op.op_id, op.clone());
                    self.validator.apply_op(op);
                    self.hlc.update(op.hlc);
                    
                    Ok(())
                } else {
                    Err(Error::InvalidOperation("Expected CreateChannel operation".to_string()))
                }
            }
            ValidationResult::Buffered(deps) => {
                // Buffer in holdback queue
                self.holdback.buffer(op.clone(), deps, op.timestamp)
                    .map_err(|e| Error::Storage(e))?;
                Ok(())
            }
            ValidationResult::Reject(reason) => {
                Err(Error::InvalidOperation(format!("Operation rejected: {:?}", reason)))
            }
        }
    }
    
    /// Update a channel's name
    pub fn update_name(
        &mut self,
        channel_id: ChannelId,
        new_name: String,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        // Check channel exists
        let channel = self.channels.get_mut(&channel_id)
            .ok_or_else(|| Error::NotFound(format!("Channel {:?} not found", channel_id)))?;
        
        let space_id = channel.space_id;
        
        // Create operation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: Some(channel_id),
            thread_id: None,
            op_type: OpType::UpdateChannel(OpPayload::UpdateChannel {
                name: Some(new_name.clone()),
                description: None,
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
        channel.set_name(new_name);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Archive a channel
    pub fn archive_channel(
        &mut self,
        channel_id: ChannelId,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
    ) -> Result<CrdtOp> {
        let channel = self.channels.get_mut(&channel_id)
            .ok_or_else(|| Error::NotFound(format!("Channel {:?} not found", channel_id)))?;
        
        let space_id = channel.space_id;
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: Some(channel_id),
            thread_id: None,
            op_type: OpType::ArchiveChannel,
            prev_ops: vec![],
            author,
            epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(author_keypair.sign(&signing_bytes).0);
        
        channel.archive();
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Get a Channel by ID
    pub fn get_channel(&self, channel_id: &ChannelId) -> Option<&Channel> {
        self.channels.get(channel_id)
    }
    
    /// Get all Channels in a Space
    pub fn list_channels(&self, space_id: &SpaceId) -> Vec<&Channel> {
        self.space_channels
            .get(space_id)
            .map(|ids| ids.iter().filter_map(|id| self.channels.get(id)).collect())
            .unwrap_or_default()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_channel() {
        let mut manager = ChannelManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        let result = manager.create_channel(
            channel_id,
            space_id,
            "general".to_string(),
            Some("General discussion".to_string()),
            creator,
            &creator_keypair,
            EpochId(0),
        );
        
        assert!(result.is_ok());
        
        let channel = manager.get_channel(&channel_id);
        assert!(channel.is_some());
        
        let channel = channel.unwrap();
        assert_eq!(channel.name, "general");
        assert_eq!(channel.space_id, space_id);
        assert_eq!(channel.creator, creator);
        assert!(!channel.archived);
    }
    
    #[test]
    fn test_list_channels() {
        let mut manager = ChannelManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        let channel1 = ChannelId(uuid::Uuid::new_v4());
        let channel2 = ChannelId(uuid::Uuid::new_v4());
        
        manager.create_channel(
            channel1,
            space_id,
            "general".to_string(),
            None,
            creator,
            &creator_keypair,
            EpochId(0),
        ).unwrap();
        
        manager.create_channel(
            channel2,
            space_id,
            "random".to_string(),
            None,
            creator,
            &creator_keypair,
            EpochId(0),
        ).unwrap();
        
        let channels = manager.list_channels(&space_id);
        assert_eq!(channels.len(), 2);
    }
    
    #[test]
    fn test_archive_channel() {
        let mut manager = ChannelManager::new();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        manager.create_channel(
            channel_id,
            space_id,
            "old-channel".to_string(),
            None,
            creator,
            &creator_keypair,
            EpochId(0),
        ).unwrap();
        
        let result = manager.archive_channel(
            channel_id,
            creator,
            &creator_keypair,
            EpochId(0),
        );
        
        assert!(result.is_ok());
        
        let channel = manager.get_channel(&channel_id).unwrap();
        assert!(channel.archived);
    }
}
