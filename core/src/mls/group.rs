//! MLS group management for Descord Spaces
//!
//! Each Space is backed by an MLS group that provides:
//! - End-to-end encryption for all operations
//! - Epoch-based key rotation on membership changes
//! - Forward secrecy and post-compromise security
//! - Authenticated group membership

use crate::types::*;
use crate::mls::provider::DescordProvider;
use crate::permissions::Permissions;
use crate::{Error, Result};

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for MLS group creation
#[derive(Debug, Clone)]
pub struct MlsGroupConfig {
    /// Ciphersuite to use (defaults to MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519)
    pub ciphersuite: Ciphersuite,
}

impl Default for MlsGroupConfig {
    fn default() -> Self {
        Self {
            ciphersuite: Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519,
        }
    }
}

/// MLS group wrapper for a Descord Space
pub struct MlsGroup {
    /// The underlying OpenMLS group
    group: openmls::group::MlsGroup,
    
    /// Space ID this group corresponds to
    space_id: SpaceId,
    
    /// Current epoch
    current_epoch: EpochId,
    
    /// Signer keypair for this node (wrapped in Arc for sharing)
    signer: Arc<SignatureKeyPair>,
    
    /// Member roles (UserId -> Role mapping)
    /// Stored locally and synced via MLS application messages
    member_roles: HashMap<UserId, Role>,
}

impl MlsGroup {
    /// Create a new MLS group for a Space (founder)
    pub fn create(
        space_id: SpaceId,
        creator_id: UserId,
        signer: Arc<SignatureKeyPair>,
        config: MlsGroupConfig,
        provider: &DescordProvider,
    ) -> Result<Self> {
        // Create credential for creator using user_id (for member lookup)
        let credential = BasicCredential::new(creator_id.0.to_vec());
        
        // Create MLS group configuration
        // Enable RATCHET_TREE extension to include ratchet tree in Welcome messages
        let mls_group_create_config = MlsGroupCreateConfig::builder()
            .ciphersuite(config.ciphersuite)
            .use_ratchet_tree_extension(true)  // Enable ratchet tree in Welcome
            .build();
        
        // Create the group
        let group = openmls::group::MlsGroup::new(
            provider,
            &*signer,  // Deref the Arc to get &SignatureKeyPair
            &mls_group_create_config,
            CredentialWithKey {
                credential: credential.into(),
                signature_key: signer.public().into(),
            },
        )
        .map_err(|e| Error::Crypto(format!("Failed to create MLS group: {:?}", e)))?;

        // Creator starts as Admin
        let mut member_roles = HashMap::new();
        member_roles.insert(creator_id, Role::Admin);

        Ok(Self {
            group,
            space_id,
            current_epoch: EpochId(0),
            signer,
            member_roles,
        })
    }

    /// Get current epoch
    pub fn epoch(&self) -> EpochId {
        self.current_epoch
    }

    /// Get space ID
    pub fn space_id(&self) -> SpaceId {
        self.space_id
    }

    /// Get role for a user
    pub fn get_role(&self, user_id: &UserId) -> Option<Role> {
        self.member_roles.get(user_id).copied()
    }

    /// Get permissions for a user based on their role
    pub fn get_permissions(&self, user_id: &UserId) -> Permissions {
        self.get_role(user_id)
            .map(Permissions::from_role)
            .unwrap_or(Permissions::NONE)
    }

    /// Set role for a user (admin only)
    pub fn set_role(&mut self, admin_id: &UserId, target_id: UserId, new_role: Role) -> Result<()> {
        // Check if caller is admin
        let admin_perms = self.get_permissions(admin_id);
        if !admin_perms.can_manage_roles() {
            return Err(Error::Permission(
                "Only administrators can change roles".to_string()
            ));
        }

        // Update role
        self.member_roles.insert(target_id, new_role);
        
        // TODO: Sync role change via MLS application message
        
        Ok(())
    }

    /// Add member with role (for new joins)
    pub fn add_member_with_role(&mut self, user_id: UserId, role: Role) {
        self.member_roles.insert(user_id, role);
    }

    /// Add a new member to the MLS group using their KeyPackage
    /// 
    /// This adds the member to the MLS group, which:
    /// 1. Adds them to the member list
    /// 2. Triggers key rotation (new epoch)
    /// 3. Generates a Welcome message for the new member
    /// 4. Generates group info for existing members
    /// 
    /// # Arguments
    /// * `user_id` - The user to add
    /// * `role` - The role to assign to the new member
    /// * `key_package` - The KeyPackage fetched from DHT
    /// * `admin_id` - The user performing the add (must have permission)
    /// * `provider` - Crypto provider
    /// 
    /// # Returns
    /// A tuple of (MlsMessageOut, MlsMessageOut) that must be distributed:
    /// - First MlsMessageOut: Send to existing group members (the Commit)
    /// - Second MlsMessageOut: Send to the new member (the Welcome)
    pub fn add_member_with_key_package(
        &mut self,
        user_id: UserId,
        role: Role,
        key_package: openmls::prelude::KeyPackage,
        admin_id: &UserId,
        provider: &DescordProvider,
    ) -> Result<(openmls::framing::MlsMessageOut, openmls::framing::MlsMessageOut)> {
        // Check if caller has permission to add members
        let admin_perms = self.get_permissions(admin_id);
        if !admin_perms.can_manage_roles() && !admin_perms.is_administrator() {
            return Err(Error::Permission(
                "Only administrators and moderators can add members".to_string()
            ));
        }

        // Add the member to the MLS group
        // This creates a Commit that adds the member and rotates keys
        let (mls_message, welcome_msg, _group_info) = self.group
            .add_members(provider, &*self.signer, &[key_package])
            .map_err(|e| Error::Crypto(format!("Failed to add member to MLS group: {:?}", e)))?;
        
        // Merge the pending commit to update the group state
        self.group.merge_pending_commit(provider)
            .map_err(|e| Error::Crypto(format!("Failed to merge pending commit: {:?}", e)))?;
        
        // Increment epoch
        self.current_epoch = EpochId(self.current_epoch.0 + 1);

        // Add to local role mapping
        self.member_roles.insert(user_id, role);
        
        println!("✓ Added member {} to MLS group (epoch {})", user_id, self.current_epoch.0);
        
        // Return the commit message and welcome message
        // Note: welcome_msg might be an MlsMessageOut, need to extract Welcome
        Ok((mls_message, welcome_msg))
    }

    /// Remove member from MLS group and rotate keys
    /// 
    /// This removes a member from the MLS group, which:
    /// 1. Removes them from the member list
    /// 2. Triggers key rotation (new epoch)
    /// 3. Ensures removed member can't decrypt future messages
    /// 
    /// # Returns
    /// The Commit message that must be broadcast to remaining members
    /// 
    /// # Security Properties
    /// - Forward secrecy: Removed member can't decrypt new messages
    /// - Post-compromise security: New epoch keys generated
    /// - Authentication: Group membership is cryptographically verified
    pub fn remove_member_with_key_rotation(
        &mut self,
        user_id: &UserId,
        admin_id: &UserId,
        provider: &DescordProvider,
    ) -> Result<openmls::framing::MlsMessageOut> {
        // Check if caller is admin or moderator
        let admin_perms = self.get_permissions(admin_id);
        if !admin_perms.can_kick_members() {
            return Err(Error::Permission(
                "Only administrators and moderators can remove members".to_string()
            ));
        }

        // Find the member's leaf index in the MLS group
        let mut member_index: Option<LeafNodeIndex> = None;
        
        for member in self.group.members() {
            // Extract the credential bytes
            let credential = member.credential.serialized_content();
            
            // Check if this credential matches the user_id
            if credential == user_id.0.as_slice() {
                member_index = Some(member.index);
                break;
            }
        }

        let member_index = member_index.ok_or_else(|| {
            Error::NotFound(format!("Member {} not found in MLS group", user_id))
        })?;

        // Create and commit the Remove proposal
        // This generates a new epoch and new encryption keys
        let (mls_message, _welcome, _group_info) = self.group
            .remove_members(provider, &*self.signer, &[member_index])
            .map_err(|e| Error::Crypto(format!("Failed to remove member from MLS group: {:?}", e)))?;
        
        // Merge the pending commit to update our own group state
        self.group.merge_pending_commit(provider)
            .map_err(|e| Error::Crypto(format!("Failed to merge pending commit: {:?}", e)))?;

        // Increment epoch
        self.current_epoch = EpochId(self.group.epoch().as_u64());

        // Remove from local role mapping
        self.member_roles.remove(user_id);
        
        println!("✓ Removed member {} from MLS group (epoch {})", user_id, self.current_epoch.0);
        
        // Return the Commit message that must be broadcast to remaining members
        Ok(mls_message)
    }

    /// Process a Welcome message to join an existing MLS group
    /// 
    /// This method is called when a user receives a Welcome message after being added
    /// to a Space. It creates a new MlsGroup from the Welcome message.
    /// 
    /// # Returns
    /// A new MlsGroup instance that is synced with the existing group
    pub fn from_welcome(
        welcome_bytes: Vec<u8>,
        user_id: UserId,
        signer: Arc<SignatureKeyPair>,
        provider: &DescordProvider,
    ) -> Result<Self> {
        // Deserialize the MlsMessageIn (which wraps the Welcome)
        use tls_codec::Deserialize;
        let mls_message_in = openmls::framing::MlsMessageIn::tls_deserialize(&mut welcome_bytes.as_slice())
            .map_err(|e| Error::Serialization(format!("Failed to deserialize MlsMessageIn: {:?}", e)))?;
        
        // Extract the Welcome from the MlsMessageIn
        let welcome = match mls_message_in.extract() {
            openmls::framing::MlsMessageBodyIn::Welcome(w) => w,
            _ => return Err(Error::Serialization("Expected Welcome message, got something else".to_string())),
        };
        
        let group_config = MlsGroupJoinConfig::default();
        
        let mls_group = StagedWelcome::new_from_welcome(
            provider,
            &group_config,
            welcome,
            None, // No ratchet tree extension
        )
        .map_err(|e| Error::Crypto(format!("Failed to stage Welcome: {:?}", e)))?
        .into_group(provider)
        .map_err(|e| Error::Crypto(format!("Failed to create group from Welcome: {:?}", e)))?;
        
        // Extract space_id from group context
        // For now, we'll use a placeholder since we need to get it from the Welcome message context
        // In production, the space_id should be included in the group context or sent separately
        let space_id = SpaceId([0u8; 32]); // Placeholder - should be extracted from context
        
        let current_epoch = EpochId(mls_group.epoch().as_u64());
        
        // Initialize with the joining user's role (will be updated from CRDT state)
        let mut member_roles = HashMap::new();
        member_roles.insert(user_id, Role::Member);
        
        Ok(Self {
            group: mls_group,
            space_id,
            current_epoch,
            signer,
            member_roles,
        })
    }

    /// Encrypt application message data using MLS
    /// 
    /// # Arguments
    /// * `plaintext` - The message content to encrypt
    /// * `provider` - Crypto provider
    /// 
    /// # Returns
    /// Encrypted MlsMessageOut that can be sent to group members
    pub fn encrypt_application_message(
        &mut self,
        plaintext: &[u8],
        provider: &DescordProvider,
    ) -> Result<openmls::framing::MlsMessageOut> {
        let mls_message = self.group
            .create_message(provider, &*self.signer, plaintext)
            .map_err(|e| Error::Crypto(format!("Failed to encrypt application message: {:?}", e)))?;
        
        Ok(mls_message)
    }

    /// Decrypt application message received from the group
    /// 
    /// # Arguments
    /// * `encrypted_bytes` - The serialized encrypted MlsMessageIn
    /// * `provider` - Crypto provider
    /// 
    /// # Returns
    /// The decrypted plaintext message content
    pub fn decrypt_application_message(
        &mut self,
        encrypted_bytes: &[u8],
        provider: &DescordProvider,
    ) -> Result<Vec<u8>> {
        use tls_codec::Deserialize;
        
        // Deserialize the MlsMessageIn
        let mls_message_in = openmls::framing::MlsMessageIn::tls_deserialize(&mut &encrypted_bytes[..])
            .map_err(|e| Error::Serialization(format!("Failed to deserialize MLS message: {:?}", e)))?;
        
        // Convert to ProtocolMessage (extract from the MlsMessageIn wrapper)
        let protocol_message = mls_message_in.try_into_protocol_message()
            .map_err(|e| Error::Crypto(format!("Invalid protocol message: {:?}", e)))?;
        
        // Process the message (this verifies signature and decrypts)
        let processed_message = self.group
            .process_message(provider, protocol_message)
            .map_err(|e| {
                eprintln!("  🔍 MLS DECRYPTION DEBUG:");
                eprintln!("     Current epoch: {}", self.current_epoch.0);
                eprintln!("     Group members: {}", self.member_roles.len());
                eprintln!("     Error details: {:?}", e);
                Error::Crypto(format!("Failed to process MLS message: {:?}", e))
            })?;
        
        // Extract the application message
        match processed_message.into_content() {
            ProcessedMessageContent::ApplicationMessage(app_msg) => {
                Ok(app_msg.into_bytes())
            }
            ProcessedMessageContent::ProposalMessage(_) => {
                Err(Error::Crypto("Received proposal instead of application message".to_string()))
            }
            ProcessedMessageContent::ExternalJoinProposalMessage(_) => {
                Err(Error::Crypto("Received external join proposal instead of application message".to_string()))
            }
            ProcessedMessageContent::StagedCommitMessage(_) => {
                // This is a commit message (membership change) - need to merge it
                Err(Error::Crypto("Received commit message - should be handled separately".to_string()))
            }
        }
    }

    /// Process a Commit message to update group state (epoch change)
    /// 
    /// When a new member is added or removed, the group creator sends a Commit message
    /// to all existing members. This function processes that Commit and updates the
    /// local epoch to stay in sync with the group.
    pub fn process_commit_message(
        &mut self,
        commit_bytes: &[u8],
        provider: &DescordProvider,
    ) -> Result<()> {
        use tls_codec::Deserialize;
        
        // Deserialize the MlsMessageIn
        let mls_message_in = openmls::framing::MlsMessageIn::tls_deserialize(&mut &commit_bytes[..])
            .map_err(|e| Error::Serialization(format!("Failed to deserialize Commit: {:?}", e)))?;
        
        // Convert to ProtocolMessage
        let protocol_message = mls_message_in.try_into_protocol_message()
            .map_err(|e| Error::Crypto(format!("Invalid Commit protocol message: {:?}", e)))?;
        
        // Process the message
        let processed_message = self.group
            .process_message(provider, protocol_message)
            .map_err(|e| Error::Crypto(format!("Failed to process Commit: {:?}", e)))?;
        
        // Extract and merge the staged commit
        match processed_message.into_content() {
            ProcessedMessageContent::StagedCommitMessage(staged_commit) => {
                // Merge the commit to update our group state
                self.group.merge_staged_commit(provider, *staged_commit)
                    .map_err(|e| Error::Crypto(format!("Failed to merge Commit: {:?}", e)))?;
                
                // Update our local epoch
                self.current_epoch = EpochId(self.group.epoch().as_u64());
                
                println!("  ✓ Processed Commit - updated to epoch {}", self.current_epoch.0);
                Ok(())
            }
            _ => {
                Err(Error::Crypto("Expected Commit message but got different message type".to_string()))
            }
        }
    }

    /// Remove member (legacy method - only removes role mapping)
    /// 
    /// **Warning:** This does NOT rotate MLS keys. For security, use
    /// `remove_member_with_key_rotation()` instead.
    pub fn remove_member(&mut self, user_id: &UserId) {
        self.member_roles.remove(user_id);
    }

    /// Get the current epoch of the MLS group
    pub fn current_epoch(&self) -> EpochId {
        self.current_epoch
    }

    /// Check if user has permission to perform an action
    pub fn check_permission<F>(&self, user_id: &UserId, check: F) -> Result<()>
    where
        F: Fn(&Permissions) -> bool,
    {
        let perms = self.get_permissions(user_id);
        if check(&perms) {
            Ok(())
        } else {
            Err(Error::Permission(
                format!("User {} lacks required permission", user_id)
            ))
        }
    }

    /// List all members with their roles
    pub fn members_with_roles(&self) -> &HashMap<UserId, Role> {
        &self.member_roles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mls::provider::create_provider;

    fn create_test_keypair() -> SignatureKeyPair {
        SignatureKeyPair::new(
            Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519.signature_algorithm()
        ).unwrap()
    }

    fn create_test_user_id() -> UserId {
        UserId([1u8; 32])
    }

    #[test]
    fn test_create_group() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let user_id = create_test_user_id();
        let keypair = Arc::new(create_test_keypair());
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, user_id, keypair, config, &provider);
        assert!(group.is_ok());

        let group = group.unwrap();
        assert_eq!(group.space_id(), space_id);
        assert_eq!(group.epoch(), EpochId(0));
        
        // Creator should be admin
        assert_eq!(group.get_role(&user_id), Some(Role::Admin));
    }

    #[test]
    fn test_epoch_starts_at_zero() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let user_id = create_test_user_id();
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, user_id, keypair, config, &provider).unwrap();
        assert_eq!(group.epoch().0, 0);
    }

    #[test]
    fn test_creator_is_admin() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let user_id = create_test_user_id();
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, user_id, keypair, config, &provider).unwrap();
        
        // Creator should have admin role
        assert_eq!(group.get_role(&user_id), Some(Role::Admin));
        
        // Creator should have admin permissions
        let perms = group.get_permissions(&user_id);
        assert!(perms.is_administrator());
        assert!(perms.can_manage_roles());
        assert!(perms.can_manage_channels());
    }

    #[test]
    fn test_role_management() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let user_id = UserId([2u8; 32]);
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let mut group = MlsGroup::create(space_id, admin_id, keypair, config, &provider).unwrap();
        
        // Add new member as Member
        group.add_member_with_role(user_id, Role::Member);
        assert_eq!(group.get_role(&user_id), Some(Role::Member));
        
        // Admin promotes user to Moderator
        let result = group.set_role(&admin_id, user_id, Role::Moderator);
        assert!(result.is_ok());
        assert_eq!(group.get_role(&user_id), Some(Role::Moderator));
        
        // Check moderator permissions
        let perms = group.get_permissions(&user_id);
        assert!(perms.can_kick_members());
        assert!(!perms.can_manage_roles());
    }

    #[test]
    fn test_permission_enforcement() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let member_id = UserId([2u8; 32]);
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let mut group = MlsGroup::create(space_id, admin_id, keypair, config, &provider).unwrap();
        group.add_member_with_role(member_id, Role::Member);
        
        // Member tries to change roles (should fail)
        let result = group.set_role(&member_id, admin_id, Role::Member);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Permission(_)));
        
        // Admin can change roles (should succeed)
        let result = group.set_role(&admin_id, member_id, Role::Moderator);
        assert!(result.is_ok());
    }

    #[test]
    fn test_permission_checks() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let member_id = UserId([2u8; 32]);
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let mut group = MlsGroup::create(space_id, admin_id, keypair, config, &provider).unwrap();
        group.add_member_with_role(member_id, Role::Member);
        
        // Admin can kick
        let result = group.check_permission(&admin_id, |p| p.can_kick_members());
        assert!(result.is_ok());
        
        // Member cannot kick
        let result = group.check_permission(&member_id, |p| p.can_kick_members());
        assert!(result.is_err());
        
        // Member can send messages
        let result = group.check_permission(&member_id, |p| p.can_send_messages());
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_member() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let user_id = UserId([2u8; 32]);
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let mut group = MlsGroup::create(space_id, admin_id, keypair, config, &provider).unwrap();
        group.add_member_with_role(user_id, Role::Member);
        
        assert_eq!(group.get_role(&user_id), Some(Role::Member));
        
        // Remove member
        group.remove_member(&user_id);
        
        // Should have no role now
        assert_eq!(group.get_role(&user_id), None);
        
        // Should have no permissions
        let perms = group.get_permissions(&user_id);
        assert_eq!(perms, Permissions::NONE);
    }

    #[test]
    fn test_add_member_with_key_package() {
        use crate::mls::KeyPackageStore;
        
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let new_member_id = UserId([2u8; 32]);
        let admin_keypair = create_test_keypair();
        let config = MlsGroupConfig::default();
        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

        // Create MLS group as admin
        let mut admin_group = MlsGroup::create(space_id, admin_id, admin_keypair, config, &provider).unwrap();
        
        // Generate KeyPackage for new member
        let member_signer = SignatureKeyPair::new(ciphersuite.signature_algorithm()).unwrap();
        let mut kp_store = KeyPackageStore::new(new_member_id, member_signer, ciphersuite);
        let key_packages = kp_store.generate_key_packages(1, &provider).unwrap();
        let key_package_bundle = &key_packages[0];
        
        // Deserialize the KeyPackage
        let key_package = KeyPackageStore::deserialize_key_package(key_package_bundle, &provider).unwrap();
        
        // Admin adds new member to MLS group
        let result = admin_group.add_member_with_key_package(
            new_member_id,
            Role::Member,
            key_package,
            &admin_id,
            &provider
        );
        
        assert!(result.is_ok());
        let (commit_msg, welcome_msg) = result.unwrap();
        
        // Verify commit and welcome messages were created
        let commit_bytes = commit_msg.to_bytes().unwrap();
        let welcome_bytes = welcome_msg.to_bytes().unwrap();
        assert!(!commit_bytes.is_empty());
        assert!(!welcome_bytes.is_empty());
        
        // Verify member was added to role mapping
        assert_eq!(admin_group.get_role(&new_member_id), Some(Role::Member));
        
        // Verify epoch incremented
        assert_eq!(admin_group.current_epoch().0, 1);
        
        // Note: Processing the Welcome message requires the init key that was
        // generated with the KeyPackage. In production, the KeyPackageStore would
        // maintain a mapping of KeyPackages to their init keys. For this test,
        // we verify that:
        // 1. KeyPackage generation works ✓
        // 2. add_member_with_key_package succeeds ✓  
        // 3. Commit and Welcome messages are created ✓
        // 4. Member is added to the group ✓
        // 5. Epoch increments ✓
        //
        // The Welcome message processing would require the KeyPackageStore to
        // track init keys, which is a future enhancement.
        
        println!("✅ MLS member addition flow test passed!");
        println!("   - KeyPackage generated and serialized");
        println!("   - Member added to MLS group");
        println!("   - Commit message created ({} bytes)", commit_bytes.len());
        println!("   - Welcome message created ({} bytes)", welcome_bytes.len());
        println!("   - Epoch incremented to {}", admin_group.current_epoch().0);
    }

    #[test]
    fn test_remove_member_with_key_rotation() {
        let provider = create_provider();
        let space_id = SpaceId::new();
        let admin_id = create_test_user_id();
        let member_id = UserId([2u8; 32]);
        let admin_keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let mut group = MlsGroup::create(space_id, admin_id, admin_keypair, config, &provider).unwrap();
        
        // Add member first (using legacy method since we need them in the MLS group)
        group.add_member_with_role(member_id, Role::Member);
        
        // Note: In production, member would be added via add_member_with_key_package
        // For this test, we're just verifying the removal logic
        
        // Remove member with key rotation
        // This will fail because member isn't actually in the OpenMLS group
        // (they were only added to role mapping)
        let result = group.remove_member_with_key_rotation(&member_id, &admin_id, &provider);
        
        // Should fail with NotFound because member isn't in actual MLS group
        assert!(result.is_err());
        match result {
            Err(Error::NotFound(_)) => {
                println!("✅ Correctly detected member not in MLS group");
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
