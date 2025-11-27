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
    
    /// Custom roles defined for this space
    pub roles: HashMap<RoleId, SpaceRole>,
    
    /// Member role assignments (user_id -> role_id)
    pub member_roles: HashMap<UserId, RoleId>,
    
    /// Default role for new members (like Discord's @everyone)
    pub default_role: RoleId,
    
    /// DEPRECATED: Old members HashMap (kept for backward compatibility)
    /// TODO: Remove after migration
    #[deprecated(note = "Use member_roles instead")]
    pub members: HashMap<UserId, Role>,
    
    /// Visibility and discoverability settings
    pub visibility: SpaceVisibility,
    
    /// Membership mode (lightweight vs MLS encrypted)
    pub membership_mode: SpaceMembershipMode,
    
    /// Active invites (invite_id -> invite)
    pub invites: HashMap<InviteId, Invite>,
    
    /// Invite permissions for this space
    pub invite_permissions: InvitePermissions,
    
    /// Current MLS epoch (only used if membership_mode is MLS)
    pub epoch: EpochId,
    
    /// Creation timestamp
    pub created_at: u64,
}

impl Space {
    /// Create default roles for a new space (Admin, Moderator, Member)
    fn create_default_roles(owner: UserId) -> (HashMap<RoleId, SpaceRole>, HashMap<UserId, RoleId>, RoleId) {
        let admin_role = SpaceRole::admin();
        let mod_role = SpaceRole::moderator();
        let member_role = SpaceRole::member();
        
        let admin_role_id = admin_role.id;
        let mod_role_id = mod_role.id;
        let member_role_id = member_role.id;
        
        let mut roles = HashMap::new();
        roles.insert(admin_role_id, admin_role);
        roles.insert(mod_role_id, mod_role);
        roles.insert(member_role_id, member_role);
        
        // Owner gets Admin role
        let mut member_roles = HashMap::new();
        member_roles.insert(owner, admin_role_id);
        
        (roles, member_roles, member_role_id)
    }
    
    /// Create a new Space
    pub fn new(
        id: SpaceId,
        name: String,
        description: Option<String>,
        owner: UserId,
        created_at: u64,
    ) -> Self {
        let (roles, member_roles, default_role) = Self::create_default_roles(owner);
        
        // Create deprecated members HashMap for backward compatibility
        let mut members = HashMap::new();
        members.insert(owner, Role::Admin);
        
        Self {
            id,
            name,
            description,
            owner,
            roles,
            member_roles,
            default_role,
            members,
            visibility: SpaceVisibility::default(),
            membership_mode: SpaceMembershipMode::default(),
            invites: HashMap::new(),
            invite_permissions: InvitePermissions::default(),
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
        let (roles, member_roles, default_role) = Self::create_default_roles(owner);
        
        let mut members = HashMap::new();
        members.insert(owner, Role::Admin);
        
        Self {
            id,
            name,
            description,
            owner,
            roles,
            member_roles,
            default_role,
            members,
            visibility,
            membership_mode: SpaceMembershipMode::default(),
            invites: HashMap::new(),
            invite_permissions: InvitePermissions::default(),
            epoch: EpochId(0),
            created_at,
        }
    }
    
    /// Create a new Space with specific visibility and membership mode
    pub fn new_with_mode(
        id: SpaceId,
        name: String,
        description: Option<String>,
        owner: UserId,
        visibility: SpaceVisibility,
        membership_mode: SpaceMembershipMode,
        created_at: u64,
    ) -> Self {
        let (roles, member_roles, default_role) = Self::create_default_roles(owner);
        
        let mut members = HashMap::new();
        members.insert(owner, Role::Admin);
        
        Self {
            id,
            name,
            description,
            owner,
            roles,
            member_roles,
            default_role,
            members,
            visibility,
            membership_mode,
            invites: HashMap::new(),
            invite_permissions: InvitePermissions::default(),
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
    
    // ========================================================================
    // Permission System Methods
    // ========================================================================
    
    /// Check if user has a specific permission
    pub fn has_permission(&self, user_id: &UserId, check: impl Fn(&SpacePermissions) -> bool) -> bool {
        // Owner always has all permissions
        if *user_id == self.owner {
            return true;
        }
        
        // Check user's role
        if let Some(role_id) = self.member_roles.get(user_id) {
            if let Some(role) = self.roles.get(role_id) {
                return check(&role.permissions);
            }
        }
        
        // Fall back to default role
        if let Some(role) = self.roles.get(&self.default_role) {
            return check(&role.permissions);
        }
        
        false
    }
    
    /// Check if user can create channels
    pub fn can_create_channels(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::CREATE_CHANNELS))
    }
    
    /// Check if user can delete channels
    pub fn can_delete_channels(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::DELETE_CHANNELS))
    }
    
    /// Check if user can manage channels
    pub fn can_manage_channels(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::MANAGE_CHANNELS))
    }
    
    /// Check if user can kick members
    pub fn can_kick_members(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::KICK_MEMBERS))
    }
    
    /// Check if user can manage roles
    pub fn can_manage_roles(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::MANAGE_ROLES))
    }
    
    /// Check if user can delete messages
    pub fn can_delete_messages(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::DELETE_MESSAGES))
    }
    
    /// Check if user can invite members
    pub fn can_invite_members(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.has(SpacePermissions::INVITE_MEMBERS))
    }
    
    /// Check if user can assign a specific role (hierarchy check)
    pub fn can_assign_role(&self, assigner: &UserId, target_role_id: &RoleId) -> bool {
        // Owner can assign any role
        if *assigner == self.owner {
            return true;
        }
        
        // Must have MANAGE_ROLES permission
        if !self.can_manage_roles(assigner) {
            return false;
        }
        
        // Get assigner's role position
        let assigner_position = self.member_roles.get(assigner)
            .and_then(|rid| self.roles.get(rid))
            .map(|r| r.position)
            .unwrap_or(0);
        
        // Get target role position
        let target_position = self.roles.get(target_role_id)
            .map(|r| r.position)
            .unwrap_or(0);
        
        // Can't assign role equal or higher than your own
        assigner_position > target_position
    }
    
    /// Assign a role to a user
    pub fn assign_role(&mut self, user_id: UserId, role_id: RoleId) -> Result<()> {
        // Check role exists
        if !self.roles.contains_key(&role_id) {
            return Err(Error::NotFound(format!("Role {:?} not found", role_id)));
        }
        
        // Update role assignment
        self.member_roles.insert(user_id, role_id);
        
        // Also update deprecated members HashMap for backward compatibility
        let role = if let Some(r) = self.roles.get(&role_id) {
            if r.permissions.is_admin() {
                Role::Admin
            } else if r.permissions.has(SpacePermissions::KICK_MEMBERS) {
                Role::Moderator
            } else {
                Role::Member
            }
        } else {
            Role::Member
        };
        self.members.insert(user_id, role);
        
        Ok(())
    }
    
    /// Get user's role
    pub fn get_user_role(&self, user_id: &UserId) -> Option<&SpaceRole> {
        self.member_roles.get(user_id)
            .and_then(|role_id| self.roles.get(role_id))
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
        let signer = std::sync::Arc::new(signer); // Wrap in Arc
        
        let mls_group = MlsGroup::create(
            space_id,
            creator,
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
        // Use default membership mode (MLS) for backwards compatibility
        self.create_space_with_mode(
            space_id,
            name,
            description,
            visibility,
            SpaceMembershipMode::default(),
            creator,
            creator_keypair,
            provider,
        )
    }

    /// Create a new Space with specific visibility and membership mode
    pub fn create_space_with_mode(
        &mut self,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
        membership_mode: SpaceMembershipMode,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        provider: &DescordProvider,
    ) -> Result<CrdtOp> {
        // Check if space already exists
        if self.spaces.contains_key(&space_id) {
            return Err(Error::AlreadyExists(format!("Space {:?} already exists", space_id)));
        }
        
        // Conditionally create MLS group based on membership mode
        let mls_group = if membership_mode.uses_space_mls() {
            let mls_config = MlsGroupConfig::default();
            let signer = openmls_basic_credential::SignatureKeyPair::new(
                mls_config.ciphersuite.signature_algorithm()
            ).map_err(|e| Error::Crypto(format!("Failed to create signer: {:?}", e)))?;
            let signer = std::sync::Arc::new(signer);
            
            Some(MlsGroup::create(
                space_id,
                creator,
                signer,
                mls_config,
                provider,
            )?)
        } else {
            None  // Lightweight mode: no space-level MLS group
        };
        
        // Create Space
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let space = Space::new_with_mode(
            space_id,
            name.clone(),
            description.clone(),
            creator,
            visibility,
            membership_mode,
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
        
        // Only insert MLS group if one was created
        if let Some(group) = mls_group {
            self.mls_groups.insert(space_id, group);
        }
        
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
    
    /// Add a member to a Space with MLS group integration
    /// 
    /// This adds the member to both the Space and the MLS group,
    /// triggering key rotation and returning messages to distribute.
    pub fn add_member_with_mls(
        &mut self,
        space_id: &SpaceId,
        user_id: UserId,
        role: Role,
        key_package: openmls::prelude::KeyPackage,
        admin_id: &UserId,
        provider: &crate::mls::DescordProvider,
    ) -> Result<(openmls::framing::MlsMessageOut, openmls::framing::MlsMessageOut)> {
        // Get the MLS group for this Space
        let mls_group = self.mls_groups.get_mut(space_id)
            .ok_or_else(|| Error::NotFound(format!("MLS group for Space {:?} not found", space_id)))?;
        
        // Add member to MLS group and get messages
        let (commit_msg, welcome_msg) = mls_group.add_member_with_key_package(
            user_id,
            role,
            key_package,
            admin_id,
            provider,
        )?;
        
        println!("✓ MLS group updated - new epoch: {}", mls_group.epoch().0);
        
        Ok((commit_msg, welcome_msg))
    }
    
    /// Remove a member from a Space (kick)
    /// 
    /// Returns the Commit message that must be broadcast to remaining members
    pub fn remove_member(
        &mut self,
        space_id: SpaceId,
        user_id: UserId,
        author: UserId,
        author_keypair: &crate::crypto::signing::Keypair,
        provider: &DescordProvider,
    ) -> Result<(CrdtOp, Option<openmls::framing::MlsMessageOut>)> {
        // Check space exists
        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Check author has permission (Admin or Moderator)
        let author_role = space.get_role(&author)
            .ok_or_else(|| Error::Permission("Author not in Space".to_string()))?;
        
        if !matches!(author_role, Role::Admin | Role::Moderator) {
            return Err(Error::Permission("Only admins and moderators can remove members".to_string()));
        }
        
        // Can't remove yourself
        if user_id == author {
            return Err(Error::Permission("Cannot remove yourself from Space".to_string()));
        }
        
        // Check target user is actually a member
        if !space.members.contains_key(&user_id) {
            return Err(Error::NotFound(format!("User {:?} not a member of Space", user_id)));
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
            op_type: OpType::RemoveMember(OpPayload::RemoveMember {
                user_id,
                reason: None, // Optional reason for removal
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
        
        // Apply locally - remove from Space
        space.remove_member(&user_id);
        
        // MLS key rotation: Remove member from MLS group
        let commit_msg = if let Some(mls_group) = self.mls_groups.get_mut(&space_id) {
            // Remove member and rotate keys
            match mls_group.remove_member_with_key_rotation(&user_id, &author, provider) {
                Ok(commit) => {
                    println!("✓ MLS keys rotated - removed member can't decrypt future messages");
                    Some(commit)
                }
                Err(e) => {
                    eprintln!("⚠ Warning: MLS key rotation failed: {}", e);
                    eprintln!("  Removed member may still be able to decrypt new messages");
                    // Continue anyway - the member is still removed from the Space
                    None
                }
            }
        } else {
            eprintln!("⚠ Warning: No MLS group found for Space");
            eprintln!("  This Space may not have E2E encryption enabled");
            None
        };
        
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok((op, commit_msg))
    }
    
    /// Get a Space by ID
    pub fn get_space(&self, space_id: &SpaceId) -> Option<&Space> {
        self.spaces.get(space_id)
    }
    
    /// Add a Space from DHT metadata (without MLS group)
    /// 
    /// This is used when joining a Space while the creator is offline.
    /// The Space metadata is fetched from DHT, but we don't have MLS keys yet.
    /// The user won't be able to decrypt messages until an admin adds them to the MLS group.
    pub fn add_space_from_dht(&mut self, space: Space) {
        let space_id = space.id;
        
        // Only add if it doesn't exist
        if !self.spaces.contains_key(&space_id) {
            self.spaces.insert(space_id, space);
            // Note: No MLS group yet - will be initialized when we receive Welcome message
        }
    }
    
    /// Get all Spaces
    pub fn list_spaces(&self) -> Vec<&Space> {
        self.spaces.values().collect()
    }
    
    /// Get MLS group for a Space
    pub fn get_mls_group(&self, space_id: &SpaceId) -> Option<&MlsGroup> {
        self.mls_groups.get(space_id)
    }
    
    /// Get mutable MLS group for a Space (for encryption/decryption)
    pub fn get_mls_group_mut(&mut self, space_id: &SpaceId) -> Option<&mut MlsGroup> {
        self.mls_groups.get_mut(space_id)
    }
    
    /// Store an MLS group for a Space (e.g., after processing a Welcome message)
    pub fn store_mls_group(&mut self, space_id: SpaceId, mls_group: MlsGroup) {
        self.mls_groups.insert(space_id, mls_group);
    }
    
    /// Get mutable iterator over all MLS groups (for processing Commits)
    pub fn mls_groups_mut(&mut self) -> impl Iterator<Item = (&SpaceId, &mut MlsGroup)> {
        self.mls_groups.iter_mut()
    }
    
    /// Generate a random invite code (8 characters, alphanumeric)
    fn generate_invite_code() -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = rand::thread_rng();
        
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Create a new invite for a space
    pub fn create_invite(
        &mut self,
        space_id: SpaceId,
        creator: UserId,
        creator_keypair: &crate::crypto::signing::Keypair,
        max_uses: Option<u32>,
        max_age_hours: Option<u32>,
    ) -> Result<CrdtOp> {
        println!("🎫 [CREATE_INVITE] START");
        println!("   Space: {}", hex::encode(&space_id.0[..8]));
        println!("   Creator: {}", hex::encode(&creator.as_bytes()[..8]));
        
        let space = self.spaces.get(&space_id)
            .ok_or_else(|| {
                println!("✗ [CREATE_INVITE] Space not found: {}", hex::encode(&space_id.0[..8]));
                Error::NotFound(format!("Space {:?} not found", space_id))
            })?;
        
        println!("✓ [CREATE_INVITE] Space found: {}", space.name);
        
        // Check permissions
        let creator_role = space.get_role(&creator)
            .ok_or_else(|| {
                println!("✗ [CREATE_INVITE] User not a member of space");
                Error::Rejected("Not a member of the space".to_string())
            })?;
        
        println!("✓ [CREATE_INVITE] User role: {:?}", creator_role);
        
        let can_create = Invite::can_create(*creator_role, &space.invite_permissions);
        println!("   Permission check: can_create={}", can_create);
        
        if !can_create {
            println!("✗ [CREATE_INVITE] Permission denied");
            return Err(Error::Rejected(
                "Insufficient permissions to create invites".to_string()
            ));
        }
        
        println!("✓ [CREATE_INVITE] Permission granted");
        
        // Create invite
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let expires_at = max_age_hours.map(|hours| {
            current_time + (hours as u64 * 3600)
        });
        
        let invite = Invite {
            id: InviteId(uuid::Uuid::new_v4()),
            space_id,
            creator,
            code: Self::generate_invite_code(),
            max_uses,
            expires_at,
            uses: 0,
            created_at: current_time,
            revoked: false,
        };
        
        // Create CRDT operation
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateInvite(OpPayload::CreateInvite {
                invite: invite.clone(),
            }),
            prev_ops: vec![],
            author: creator,
            epoch: space.epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(creator_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        let space = self.spaces.get_mut(&space_id).unwrap();
        space.invites.insert(invite.id, invite.clone());
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        println!("✓ [CREATE_INVITE] Invite created successfully");
        println!("   Invite code: {}", invite.code);
        println!("   Invite ID: {}", invite.id.0);
        
        Ok(op)
    }
    
    /// Revoke an invite
    pub fn revoke_invite(
        &mut self,
        space_id: SpaceId,
        invite_id: InviteId,
        revoker: UserId,
        revoker_keypair: &crate::crypto::signing::Keypair,
    ) -> Result<CrdtOp> {
        let space = self.spaces.get(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Check permissions (admins and invite creator can revoke)
        let revoker_role = space.get_role(&revoker)
            .ok_or_else(|| Error::Rejected("Not a member of the space".to_string()))?;
        
        let invite = space.invites.get(&invite_id)
            .ok_or_else(|| Error::NotFound(format!("Invite {:?} not found", invite_id)))?;
        
        if !revoker_role.can_moderate() && invite.creator != revoker {
            return Err(Error::Rejected(
                "Only admins/moderators or invite creator can revoke invites".to_string()
            ));
        }
        
        // Create CRDT operation
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::RevokeInvite(OpPayload::RevokeInvite {
                invite_id,
            }),
            prev_ops: vec![],
            author: revoker,
            epoch: space.epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(revoker_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        let space = self.spaces.get_mut(&space_id).unwrap();
        if let Some(invite) = space.invites.get_mut(&invite_id) {
            invite.revoked = true;
        }
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Use an invite to join a space
    pub fn use_invite(
        &mut self,
        space_id: SpaceId,
        code: String,
        joiner: UserId,
        joiner_keypair: &crate::crypto::signing::Keypair,
    ) -> Result<CrdtOp> {
        let space = self.spaces.get(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Find invite by code
        let invite = space.invites.values()
            .find(|inv| inv.code == code && inv.space_id == space_id)
            .ok_or_else(|| Error::NotFound("Invalid invite code".to_string()))?;
        
        // Validate invite
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if !invite.is_valid(current_time) {
            return Err(Error::Rejected("Invite is no longer valid".to_string()));
        }
        
        // Check if already a member
        if space.is_member(&joiner) {
            return Err(Error::AlreadyExists("Already a member of this space".to_string()));
        }
        
        let invite_id = invite.id;
        
        // Create CRDT operation for using the invite
        let mut op = CrdtOp {
            op_id: OpId(uuid::Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::UseInvite(OpPayload::UseInvite {
                invite_id,
                code: code.clone(),
            }),
            prev_ops: vec![],
            author: joiner,
            epoch: space.epoch,
            hlc: self.hlc.tick(),
            timestamp: current_time,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(joiner_keypair.sign(&signing_bytes).0);
        
        // Apply locally
        let space = self.spaces.get_mut(&space_id).unwrap();
        // Increment invite use count
        if let Some(invite) = space.invites.get_mut(&invite_id) {
            invite.uses += 1;
        }
        // Add member with default role
        space.add_member(joiner, Role::Member);
        self.operations.insert(op.op_id, op.clone());
        self.validator.apply_op(&op);
        
        Ok(op)
    }
    
    /// Get all invites for a space
    pub fn list_invites(&self, space_id: &SpaceId) -> Vec<&Invite> {
        if let Some(space) = self.spaces.get(space_id) {
            space.invites.values().collect()
        } else {
            vec![]
        }
    }
    
    /// Get a specific invite by ID
    pub fn get_invite(&self, space_id: &SpaceId, invite_id: &InviteId) -> Option<&Invite> {
        self.spaces.get(space_id)
            .and_then(|space| space.invites.get(invite_id))
    }
    
    /// Process a remote CreateInvite operation
    pub fn process_create_invite(&mut self, op: &CrdtOp) -> Result<()> {
        if let OpType::CreateInvite(OpPayload::CreateInvite { invite }) = &op.op_type {
            // Validate the operation
            match self.validator.validate(op, &self.operations) {
                ValidationResult::Accept => {
                    // Apply the operation
                    if let Some(space) = self.spaces.get_mut(&op.space_id) {
                        space.invites.insert(invite.id, invite.clone());
                        self.operations.insert(op.op_id, op.clone());
                        self.validator.apply_op(op);
                    }
                    Ok(())
                }
                ValidationResult::Buffered(_) => {
                    // TODO: Properly handle buffering with missing_deps
                    Ok(())
                }
                ValidationResult::Reject(_) => {
                    Err(Error::Rejected("Operation validation failed".to_string()))
                }
            }
        } else {
            Err(Error::Crdt("Invalid operation type for process_create_invite".to_string()))
        }
    }
    
    /// Process a remote RevokeInvite operation
    pub fn process_revoke_invite(&mut self, op: &CrdtOp) -> Result<()> {
        if let OpType::RevokeInvite(OpPayload::RevokeInvite { invite_id }) = &op.op_type {
            // Validate the operation
            match self.validator.validate(op, &self.operations) {
                ValidationResult::Accept => {
                    // Apply the operation
                    if let Some(space) = self.spaces.get_mut(&op.space_id) {
                        if let Some(invite) = space.invites.get_mut(invite_id) {
                            invite.revoked = true;
                        }
                        self.operations.insert(op.op_id, op.clone());
                        self.validator.apply_op(op);
                    }
                    Ok(())
                }
                ValidationResult::Buffered(_) => {
                    // TODO: Properly handle buffering with missing_deps
                    Ok(())
                }
                ValidationResult::Reject(_) => {
                    Err(Error::Rejected("Operation validation failed".to_string()))
                }
            }
        } else {
            Err(Error::Crdt("Invalid operation type for process_revoke_invite".to_string()))
        }
    }
    
    /// Process a remote UseInvite operation
    pub fn process_use_invite(&mut self, op: &CrdtOp) -> Result<()> {
        if let OpType::UseInvite(OpPayload::UseInvite { invite_id, .. }) = &op.op_type {
            // Validate the operation
            match self.validator.validate(op, &self.operations) {
                ValidationResult::Accept => {
                    // Apply the operation
                    if let Some(space) = self.spaces.get_mut(&op.space_id) {
                        // Increment invite use count
                        if let Some(invite) = space.invites.get_mut(invite_id) {
                            invite.uses += 1;
                        }
                        // Add member
                        space.add_member(op.author, Role::Member);
                        self.operations.insert(op.op_id, op.clone());
                        self.validator.apply_op(op);
                    }
                    Ok(())
                }
                ValidationResult::Buffered(_) => {
                    // TODO: Properly handle buffering with missing_deps
                    Ok(())
                }
                ValidationResult::Reject(_) => {
                    Err(Error::Rejected("Operation validation failed".to_string()))
                }
            }
        } else {
            Err(Error::Crdt("Invalid operation type for process_use_invite".to_string()))
        }
    }
    
    /// Process a RemoveMember operation from the network
    pub fn process_remove_member(&mut self, op: &CrdtOp) -> Result<()> {
        if let OpType::RemoveMember(OpPayload::RemoveMember { user_id, .. }) = &op.op_type {
            // Validate the operation
            match self.validator.validate(op, &self.operations) {
                ValidationResult::Accept => {
                    // Apply the operation
                    if let Some(space) = self.spaces.get_mut(&op.space_id) {
                        // Remove member
                        space.remove_member(user_id);
                        self.operations.insert(op.op_id, op.clone());
                        self.validator.apply_op(op);
                    }
                    Ok(())
                }
                ValidationResult::Buffered(_) => {
                    // TODO: Properly handle buffering with missing_deps
                    Ok(())
                }
                ValidationResult::Reject(_) => {
                    Err(Error::Rejected("Operation validation failed".to_string()))
                }
            }
        } else {
            Err(Error::Crdt("Invalid operation type for process_remove_member".to_string()))
        }
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
        let space_id = SpaceId::new();
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
        let space_id = SpaceId::new();
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
        let space_id = SpaceId::new();
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
