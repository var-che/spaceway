//! Space management
//!
//! A Space is the top-level container in Descord, similar to a Discord server.
//! Each Space has its own MLS group for E2E encryption.

use crate::types::*;
use crate::crdt::{CrdtOp, OpType, OpPayload, Hlc, HoldbackQueue, OpValidator, ValidationResult};
use crate::mls::{MlsGroup, MlsGroupConfig};
use crate::mls::provider::DescordProvider;
use crate::{Error, Result};
use std::collections::HashMap;

/// A Space (top-level forum container)
#[derive(Debug, Clone)]
pub struct Space {
    /// Unique identifier
    pub id: SpaceId,
    
    /// Display name
    pub name: String,
    
    /// Optional description
    pub description: Option<String>,
    
    /// Creator/owner
    pub owner: UserId,
    
    /// Current members (user_id -> role)
    pub members: HashMap<UserId, Role>,
    
    /// Visibility and discoverability settings
    pub visibility: SpaceVisibility,
    
    /// Current MLS epoch
    pub epoch: EpochId,
    
    /// Creation timestamp
    pub created_at: u64,
}

impl Space {
    /// Create a new Space
    pub fn new(
        id: SpaceId,
        name: String,
        description: Option<String>,
        owner: UserId,
        created_at: u64,
    ) -> Self {
        let mut members = HashMap::new();
        members.insert(owner, Role::Admin);
        
        Self {
            id,
            name,
            description,
            owner,
            members,
            visibility: SpaceVisibility::default(),
            epoch: EpochId(0),
            created_at,
        }
    }
    
    /// Create a new Space with specific visibility
    pub fn new_with_visibility(
        id: SpaceId,
        name: String,
        description: Option<String>,
        owner: UserId,
        visibility: SpaceVisibility,
        created_at: u64,
    ) -> Self {
        let mut members = HashMap::new();
        members.insert(owner, Role::Admin);
        
        Self {
            id,
            name,
            description,
            owner,
            members,
            visibility,
            epoch: EpochId(0),
            created_at,
        }
    }
    
    /// Update space visibility
    pub fn set_visibility(&mut self, visibility: SpaceVisibility) {
        self.visibility = visibility;
    }
    
    /// Add a member to the Space
    pub fn add_member(&mut self, user_id: UserId, role: Role) {
        self.members.insert(user_id, role);
    }
    
    /// Remove a member from the Space
    pub fn remove_member(&mut self, user_id: &UserId) -> Option<Role> {
        self.members.remove(user_id)
    }
    
    /// Update a member's role
    pub fn update_role(&mut self, user_id: &UserId, new_role: Role) -> Result<()> {
        if let Some(role) = self.members.get_mut(user_id) {
            *role = new_role;
            Ok(())
        } else {
            Err(Error::NotFound(format!("User {:?} not in Space", user_id)))
        }
    }
    
    /// Check if a user is a member
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains_key(user_id)
    }
    
    /// Get a user's role
    pub fn get_role(&self, user_id: &UserId) -> Option<&Role> {
        self.members.get(user_id)
    }
    
    /// Advance to next epoch
    pub fn advance_epoch(&mut self) {
        self.epoch.0 += 1;
    }
}

/// Manages Space state and operations
pub struct SpaceManager {
    /// All spaces this node knows about
    spaces: HashMap<SpaceId, Space>,
    
    /// MLS groups for each space
    mls_groups: HashMap<SpaceId, MlsGroup>,
    
    /// CRDT operation validator
    validator: OpValidator,
    
    /// Holdback queue for out-of-order operations
    holdback: HoldbackQueue,
    
    /// HLC generator for causal ordering
    hlc: Hlc,
    
    /// All operations we've seen (for persistence)
    operations: HashMap<OpId, CrdtOp>,
}

impl SpaceManager {
    pub fn new() -> Self {
        Self {
            spaces: HashMap::new(),
            mls_groups: HashMap::new(),
            validator: OpValidator::new(),
            holdback: HoldbackQueue::new(),
            hlc: Hlc::now(),
            operations: HashMap::new(),
        }
    }
    
    /// Create a new Space (as founder)
    pub fn create_space(
        &mut self,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        provider: &DescordProvider,
    ) -> Result<CrdtOp> {
        // Check if space already exists
        if self.spaces.contains_key(&space_id) {
            return Err(Error::AlreadyExists(format!("Space {:?} already exists", space_id)));
        }
        
        // Create MLS group for this space
        let mls_config = MlsGroupConfig::default();
        let signer = openmls_basic_credential::SignatureKeyPair::new(
            mls_config.ciphersuite.signature_algorithm()
        ).map_err(|e| Error::Crypto(format!("Failed to create signer: {:?}", e)))?;
        
        let mls_group = MlsGroup::create(
            space_id,
            signer,
            mls_config,
            provider,
        )?;
        
        // Create Space
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let space = Space::new(
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
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateSpace(OpPayload::CreateSpace {
                name,
                description,
            }),
            prev_ops: vec![],
            author: creator,
            epoch: EpochId(0),
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(creator_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        self.spaces.insert(space_id, space);
        self.mls_groups.insert(space_id, mls_group);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }

    /// Create a new Space with specific visibility
    pub fn create_space_with_visibility(
        &mut self,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        provider: &DescordProvider,
    ) -> Result<CrdtOp> {
        // Check if space already exists
        if self.spaces.contains_key(&space_id) {
            return Err(Error::AlreadyExists(format!("Space {:?} already exists", space_id)));
        }
        
        // Create MLS group for this space
        let mls_config = MlsGroupConfig::default();
        let signer = openmls_basic_credential::SignatureKeyPair::new(
            mls_config.ciphersuite.signature_algorithm()
        ).map_err(|e| Error::Crypto(format!("Failed to create signer: {:?}", e)))?;
        
        let mls_group = MlsGroup::create(
            space_id,
            signer,
            mls_config,
            provider,
        )?;
        
        // Create Space
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let space = Space::new_with_visibility(
            space_id,
            name.clone(),
            description.clone(),
            creator,
            visibility,
            current_time,
        );
        
        // Create CRDT operation
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateSpace(OpPayload::CreateSpace {
                name,
                description,
            }),
            prev_ops: vec![],
            author: creator,
            epoch: EpochId(0),
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(creator_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        self.spaces.insert(space_id, space);
        self.mls_groups.insert(space_id, mls_group);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }

    /// Update a Space's visibility (admins only)
    pub fn update_space_visibility(
        &mut self,
        space_id: SpaceId,
        visibility: SpaceVisibility,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
    ) -> Result<CrdtOp> {
        // Check space exists
        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Check author has permission (Admin only)
        let author_role = space.get_role(&author)
            .ok_or_else(|| Error::Permission("Author not in Space".to_string()))?;
        
        if !author_role.is_admin() {
            return Err(Error::Permission("Only admins can change space visibility".to_string()));
        }
        
        // Create operation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::UpdateSpaceVisibility(OpPayload::UpdateSpaceVisibility {
                visibility,
            }),
            prev_ops: vec![],
            author,
            epoch: space.epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(author_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        space.set_visibility(visibility);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }

    /// Process an incoming UpdateSpaceVisibility operation
    pub fn process_update_space_visibility(&mut self, op: &CrdtOp) -> Result<()> {
        // Validate the operation
        match self.validator.validate(op, &self.operations) {
            ValidationResult::Accept => {
                if let OpType::UpdateSpaceVisibility(OpPayload::UpdateSpaceVisibility { visibility }) = &op.op_type {
                    if let Some(space) = self.spaces.get_mut(&op.space_id) {
                        // Verify author is admin
                        if let Some(role) = space.get_role(&op.author) {
                            if role.is_admin() {
                                space.set_visibility(*visibility);
                                self.operations.insert(op.op_id, op.clone());
                                self.validator.apply_op(op);
                                self.hlc.update(op.hlc);
                                return Ok(());
                            }
                        }
                        return Err(Error::Permission("Only admins can change space visibility".to_string()));
                    }
                    return Err(Error::NotFound(format!("Space {:?} not found", op.space_id)));
                } else {
                    return Err(Error::InvalidOperation("Expected UpdateSpaceVisibility operation".to_string()));
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
    
    /// Process an incoming CreateSpace operation
    pub fn process_create_space(&mut self, op: &CrdtOp) -> Result<()> {
        // Validate the operation
        match self.validator.validate(op, &self.operations) {
            ValidationResult::Accept => {
                // Extract space details
                if let OpType::CreateSpace(OpPayload::CreateSpace { name, description }) = &op.op_type {
                    let space = Space::new(
                        op.space_id,
                        name.clone(),
                        description.clone(),
                        op.author,
                        op.timestamp,
                    );
                    
                    self.spaces.insert(op.space_id, space);
                    self.operations.insert(op.op_id, op.clone());
                    self.validator.apply_op(op);
                    self.hlc.update(op.hlc);
                    
                    Ok(())
                } else {
                    Err(Error::InvalidOperation("Expected CreateSpace operation".to_string()))
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
    
    /// Add a member to a Space
    pub fn add_member(
        &mut self,
        space_id: SpaceId,
        user_id: UserId,
        role: Role,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
    ) -> Result<CrdtOp> {
        // Check space exists
        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Check author has permission (Admin only)
        let author_role = space.get_role(&author)
            .ok_or_else(|| Error::Permission("Author not in Space".to_string()))?;
        
        if !matches!(author_role, Role::Admin) {
            return Err(Error::Permission("Only admins can add members".to_string()));
        }
        
        // Create operation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::AddMember(OpPayload::AddMember {
                user_id,
                role,
            }),
            prev_ops: vec![], // TODO: Add causal dependencies
            author,
            epoch: space.epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(author_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        space.add_member(user_id, role);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Get a Space by ID
    pub fn get_space(&self, space_id: &SpaceId) -> Option<&Space> {
        self.spaces.get(space_id)
    }
    
    /// Get all Spaces
    pub fn list_spaces(&self) -> Vec<&Space> {
        self.spaces.values().collect()
    }
    
    /// Get MLS group for a Space
    pub fn get_mls_group(&self, space_id: &SpaceId) -> Option<&MlsGroup> {
        self.mls_groups.get(space_id)
    }
}

impl Default for SpaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mls::provider::create_provider;
    
    #[test]
    fn test_create_space() {
        let mut manager = SpaceManager::new();
        let provider = create_provider();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        let result = manager.create_space(
            space_id,
            "Test Space".to_string(),
            Some("A test space".to_string()),
            creator,
            &creator_keypair,
            &provider,
        );
        
        assert!(result.is_ok());
        
        let space = manager.get_space(&space_id);
        assert!(space.is_some());
        
        let space = space.unwrap();
        assert_eq!(space.name, "Test Space");
        assert_eq!(space.owner, creator);
        assert!(space.is_member(&creator));
        assert_eq!(space.get_role(&creator), Some(&Role::Admin));
    }
    
    #[test]
    fn test_add_member() {
        let mut manager = SpaceManager::new();
        let provider = create_provider();
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let creator_keypair = crate::crypto::signing::Keypair::generate();
        let creator = creator_keypair.user_id();
        
        manager.create_space(
            space_id,
            "Test Space".to_string(),
            None,
            creator,
            &creator_keypair,
            &provider,
        ).unwrap();
        
        let new_member = crate::crypto::signing::Keypair::generate().user_id();
        
        let result = manager.add_member(
            space_id,
            new_member,
            Role::Member,
            creator,
            &creator_keypair,
        );
        
        assert!(result.is_ok());
        
        let space = manager.get_space(&space_id).unwrap();
        assert!(space.is_member(&new_member));
        assert_eq!(space.get_role(&new_member), Some(&Role::Member));
    }
    
    #[test]
    fn test_space_epoch() {
        let space_id = SpaceId(uuid::Uuid::new_v4());
        let owner = crate::crypto::signing::Keypair::generate().user_id();
        let mut space = Space::new(
            space_id,
            "Test".to_string(),
            None,
            owner,
            1000,
        );
        
        assert_eq!(space.epoch.0, 0);
        space.advance_epoch();
        assert_eq!(space.epoch.0, 1);
    }
}
