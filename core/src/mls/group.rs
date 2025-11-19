//! MLS group management for Descord Spaces
//!
//! Each Space is backed by an MLS group that provides:
//! - End-to-end encryption for all operations
//! - Epoch-based key rotation on membership changes
//! - Forward secrecy and post-compromise security
//! - Authenticated group membership

use crate::types::*;
use crate::mls::provider::DescordProvider;
use crate::{Error, Result};

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;

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
}

impl MlsGroup {
    /// Create a new MLS group for a Space (founder)
    pub fn create(
        space_id: SpaceId,
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

        Ok(Self {
            group,
            space_id,
            current_epoch: EpochId(0),
            signer,
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

    #[test]
    fn test_create_group() {
        let provider = create_provider();
        let space_id = SpaceId(Uuid::new_v4());
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, keypair, config, &provider);
        assert!(group.is_ok());

        let group = group.unwrap();
        assert_eq!(group.space_id(), space_id);
        assert_eq!(group.epoch(), EpochId(0));
    }

    #[test]
    fn test_epoch_starts_at_zero() {
        let provider = create_provider();
        let space_id = SpaceId(Uuid::new_v4());
        let keypair = create_test_keypair();
        let config = MlsGroupConfig::default();

        let group = MlsGroup::create(space_id, keypair, config, &provider).unwrap();
        assert_eq!(group.epoch().0, 0);
    }
}
