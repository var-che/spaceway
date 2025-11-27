//! Channel management
//!
//! A Channel is a text communication container within a Space.
//! Channels can have Threads (multi-message discussions).

use crate::types::*;
use crate::crdt::{CrdtOp, OpType, OpPayload, Hlc, HoldbackQueue, OpValidator, ValidationResult};
use crate::mls::{MlsGroup, MlsGroupConfig};
use crate::mls::provider::DescordProvider;
use crate::{Error, Result};
use std::collections::HashMap;
use openmls::prelude::OpenMlsProvider;

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
    
    /// Membership mode (determines if channel has its own MLS group)
    /// NOTE: Channels ALWAYS use MLS mode for E2EE (unlike Spaces which can be lightweight)
    pub membership_mode: SpaceMembershipMode,
    
    /// Current MLS epoch (for this channel's MLS group)
    pub epoch: EpochId,
    
    /// Channel members (for channel-level access control)
    pub members: HashMap<UserId, Role>,
    
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
        let mut members = HashMap::new();
        members.insert(creator, Role::Admin); // Creator is admin of the channel
        
        Self {
            id,
            space_id,
            name,
            description,
            creator,
            membership_mode: SpaceMembershipMode::MLS, // Channels ALWAYS use MLS for E2EE
            epoch: EpochId(0),
            members,
            created_at,
            archived: false,
        }
    }
    
    /// Add a member to the channel
    pub fn add_member(&mut self, user_id: UserId, role: Role) {
        self.members.insert(user_id, role);
    }
    
    /// Remove a member from the channel
    pub fn remove_member(&mut self, user_id: &UserId) -> Option<Role> {
        self.members.remove(user_id)
    }
    
    /// Check if a user is a member of this channel
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains_key(user_id)
    }
    
    /// Get a user's role in this channel
    pub fn get_role(&self, user_id: &UserId) -> Option<&Role> {
        self.members.get(user_id)
    }
    
    /// Advance to next epoch (for MLS key rotation)
    pub fn advance_epoch(&mut self) {
        self.epoch.0 += 1;
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
    
    /// MLS groups for each channel (channel-level encryption)
    mls_groups: HashMap<ChannelId, MlsGroup>,
    
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
            mls_groups: HashMap::new(),
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
        // Channels always use MLS for E2EE
        self.create_channel_with_mls(
            channel_id,
            space_id,
            name,
            description,
            creator,
            creator_keypair,
            epoch,
            true, // create_mls_group = true
            None, // provider will be passed when needed
        )
    }
    
    /// Create a new Channel with optional MLS group
    /// 
    /// If create_mls_group is true and provider is Some, creates a channel-level MLS group.
    /// This enables true channel-level encryption isolation.
    pub fn create_channel_with_mls(
        &mut self,
        channel_id: ChannelId,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        epoch: EpochId,
        create_mls_group: bool,
        provider: Option<&DescordProvider>,
    ) -> Result<CrdtOp> {
        // Check if channel already exists
        if self.channels.contains_key(&channel_id) {
            return Err(Error::AlreadyExists(format!("Channel {:?} already exists", channel_id)));
        }
        
        // Create MLS group for this channel if requested
        let mls_group = if create_mls_group {
            if let Some(prov) = provider {
                let mls_config = MlsGroupConfig::default();
                let signer = openmls_basic_credential::SignatureKeyPair::new(
                    mls_config.ciphersuite.signature_algorithm()
                ).map_err(|e| Error::Crypto(format!("Failed to create signer: {:?}", e)))?;
                let signer = std::sync::Arc::new(signer);
                
                // Use channel_id as the group identifier
                Some(MlsGroup::create(
                    SpaceId(channel_id.0), // Convert ChannelId to SpaceId for MlsGroup API
                    creator,
                    signer,
                    mls_config,
                    prov,
                )?)
            } else {
                None // No provider, can't create MLS group
            }
        } else {
            None // MLS group not requested
        };
        
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
        
        // Store MLS group if created
        if let Some(group) = mls_group {
            self.mls_groups.insert(channel_id, group);
            println!("ℹ️  Created channel-level MLS group for channel: {}", hex::encode(&channel_id.0[..8]));
        }
        
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
    
    /// Get MLS group for a Channel
    pub fn get_mls_group(&self, channel_id: &ChannelId) -> Option<&MlsGroup> {
        self.mls_groups.get(channel_id)
    }
    
    /// Get mutable MLS group for a Channel (for encryption/decryption)
    pub fn get_mls_group_mut(&mut self, channel_id: &ChannelId) -> Option<&mut MlsGroup> {
        self.mls_groups.get_mut(channel_id)
    }
    
    /// Store an MLS group for a Channel (e.g., after processing a Welcome message)
    pub fn store_mls_group(&mut self, channel_id: ChannelId, mls_group: MlsGroup) {
        self.mls_groups.insert(channel_id, mls_group);
    }
    
    /// Get mutable iterator over all channel MLS groups (for processing Commits)
    pub fn mls_groups_mut(&mut self) -> impl Iterator<Item = (&ChannelId, &mut MlsGroup)> {
        self.mls_groups.iter_mut()
    }
    
    /// Add a member to a channel's MLS group
    pub fn add_member_with_mls(
        &mut self,
        channel_id: &ChannelId,
        user_id: UserId,
        role: Role,
        key_package_bytes: &[u8],
        admin_id: &UserId,  // The user who is adding the member (must have permissions)
        provider: &DescordProvider,
    ) -> Result<Vec<u8>> {
        use openmls::prelude::{KeyPackageIn, ProtocolVersion};
        use openmls::prelude::tls_codec::Deserialize;
        
        // Add to channel's member list
        if let Some(channel) = self.channels.get_mut(channel_id) {
            channel.add_member(user_id, role);
        } else {
            return Err(Error::NotFound(format!("Channel {:?} not found", channel_id)));
        }
        
        // Deserialize KeyPackage from bytes
        let key_package_in = KeyPackageIn::tls_deserialize(&mut &key_package_bytes[..])
            .map_err(|e| Error::Crypto(format!("Failed to deserialize KeyPackage: {:?}", e)))?;
        
        let key_package = key_package_in.validate(provider.crypto(), ProtocolVersion::Mls10)
            .map_err(|e| Error::Crypto(format!("Failed to validate KeyPackage: {:?}", e)))?;
        
        // Add to channel's MLS group
        if let Some(mls_group) = self.mls_groups.get_mut(channel_id) {
            // Use the provided admin_id (the caller who is adding the member)
            let (_commit_msg, welcome_msg) = mls_group.add_member_with_key_package(
                user_id,
                role,
                key_package,
                admin_id,  // Use the actual admin_id parameter
                provider,
            )?;
            
            // Serialize Welcome message to bytes
            use openmls::prelude::tls_codec::Serialize;
            let welcome_bytes = welcome_msg.tls_serialize_detached()
                .map_err(|e| Error::Serialization(format!("Failed to serialize Welcome: {}", e)))?;
            Ok(welcome_bytes)
        } else {
            Err(Error::NotFound(format!("Channel {:?} MLS group not found", channel_id)))
        }
    }
    
    /// Remove a member from a channel's MLS group only (not from space)
    pub fn remove_member_with_mls(
        &mut self,
        channel_id: &ChannelId,
        user_id: &UserId,
        admin_id: &UserId,
        provider: &DescordProvider,
    ) -> Result<Vec<u8>> {
        // Remove from channel's member list
        if let Some(channel) = self.channels.get_mut(channel_id) {
            channel.remove_member(user_id);
        } else {
            return Err(Error::NotFound(format!("Channel {:?} not found", channel_id)));
        }
        
        // Remove from channel's MLS group with key rotation
        if let Some(mls_group) = self.mls_groups.get_mut(channel_id) {
            use openmls::prelude::tls_codec::Serialize;
            let commit = mls_group.remove_member_with_key_rotation(user_id, admin_id, provider)?;
            // Serialize the MlsMessageOut to bytes
            let commit_bytes = commit.tls_serialize_detached()
                .map_err(|e| Error::Serialization(format!("Failed to serialize commit: {}", e)))?;
            Ok(commit_bytes)
        } else {
            Err(Error::NotFound(format!("Channel {:?} MLS group not found", channel_id)))
        }
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
        let space_id = SpaceId::new();
        let channel_id = ChannelId::new();
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
        let space_id = SpaceId::new();
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        let channel1 = ChannelId::new();
        let channel2 = ChannelId::new();
        
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
        let space_id = SpaceId::new();
        let channel_id = ChannelId::new();
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
