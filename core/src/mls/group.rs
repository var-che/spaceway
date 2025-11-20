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
    
    /// Signer keypair for this node
    signer: SignatureKeyPair,
    
    /// Member roles (UserId -> Role mapping)
    /// Stored locally and synced via MLS application messages
    member_roles: HashMap<UserId, Role>,
}

impl MlsGroup {
    /// Create a new MLS group for a Space (founder)
    pub fn create(
        space_id: SpaceId,
        creator_id: UserId,
        signer: SignatureKeyPair,
        config: MlsGroupConfig,
        provider: &DescordProvider,
    ) -> Result<Self> {
        // Create credential for creator
        let credential = BasicCredential::new(signer.public().to_vec());
        
        // Create MLS group configuration
        let mls_group_create_config = MlsGroupCreateConfig::builder()
            .ciphersuite(config.ciphersuite)
            .build();
        
        // Create the group
        let group = openmls::group::MlsGroup::new(
            provider,
            &signer,
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

    /// Remove member (removes role mapping)
    pub fn remove_member(&mut self, user_id: &UserId) {
        self.member_roles.remove(user_id);
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
    use uuid::Uuid;

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
        let space_id = SpaceId(Uuid::new_v4());
        let user_id = create_test_user_id();
        let keypair = create_test_keypair();
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
        let space_id = SpaceId(Uuid::new_v4());
        let user_id = create_test_user_id();
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, user_id, keypair, config, &provider).unwrap();
        assert_eq!(group.epoch().0, 0);
    }

    #[test]
    fn test_creator_is_admin() {
        let provider = create_provider();
        let space_id = SpaceId(Uuid::new_v4());
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
        let space_id = SpaceId(Uuid::new_v4());
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
        let space_id = SpaceId(Uuid::new_v4());
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
        let space_id = SpaceId(Uuid::new_v4());
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
        let space_id = SpaceId(Uuid::new_v4());
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
}
