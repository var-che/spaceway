//! KeyPackage management for MLS member addition
//!
//! KeyPackages are pre-generated cryptographic bundles that allow:
//! - Adding new members to MLS groups
//! - Establishing shared secrets without online coordination
//! - Enabling asynchronous group operations
//!
//! Flow:
//! 1. Each client generates KeyPackages on startup
//! 2. KeyPackages are published to DHT (keyed by UserId)
//! 3. When adding member, fetch their KeyPackage from DHT
//! 4. Use KeyPackage to add them to MLS group
//! 5. Distribute Welcome message to new member

use crate::types::*;
use crate::mls::provider::DescordProvider;
use crate::{Error, Result};

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use serde::{Deserialize, Serialize};

/// A KeyPackage bundle with metadata
#[derive(Clone, Serialize, Deserialize)]
pub struct KeyPackageBundle {
    /// The user this KeyPackage belongs to
    pub user_id: UserId,
    
    /// Serialized OpenMLS KeyPackage
    pub key_package_bytes: Vec<u8>,
    
    /// Timestamp when this KeyPackage was created
    pub created_at: u64,
    
    /// Signature over the KeyPackage (for verification)
    pub signature: Vec<u8>,
}

/// Manages KeyPackage generation and storage
pub struct KeyPackageStore {
    /// User ID for this client
    user_id: UserId,
    
    /// Signer keypair
    signer: SignatureKeyPair,
    
    /// Ciphersuite to use
    ciphersuite: Ciphersuite,
    
    /// Generated KeyPackages waiting to be used
    available_packages: Vec<KeyPackage>,
}

impl KeyPackageStore {
    /// Create a new KeyPackage store
    pub fn new(
        user_id: UserId,
        signer: SignatureKeyPair,
        ciphersuite: Ciphersuite,
    ) -> Self {
        Self {
            user_id,
            signer,
            ciphersuite,
            available_packages: Vec::new(),
        }
    }

    /// Generate a batch of KeyPackages
    ///
    /// # Arguments
    /// * `count` - Number of KeyPackages to generate
    /// * `provider` - Crypto provider
    ///
    /// # Returns
    /// Vector of serialized KeyPackage bundles ready for DHT storage
    pub fn generate_key_packages(
        &mut self,
        count: usize,
        provider: &DescordProvider,
    ) -> Result<Vec<KeyPackageBundle>> {
        let mut bundles = Vec::new();
        
        // Create credential
        let credential = BasicCredential::new(self.signer.public().to_vec());
        let credential_with_key = CredentialWithKey {
            credential: credential.into(),
            signature_key: self.signer.public().into(),
        };

        for _ in 0..count {
            // Generate a KeyPackage
            let key_package_bundle = KeyPackage::builder()
                .build(
                    self.ciphersuite,
                    provider,
                    &self.signer,
                    credential_with_key.clone(),
                )
                .map_err(|e| Error::Crypto(format!("Failed to create KeyPackage: {:?}", e)))?;

            // Extract the KeyPackage from the bundle
            let key_package = key_package_bundle.key_package().clone();
            
            // Serialize the KeyPackage using TLS serialization
            let key_package_bytes = serde_json::to_vec(&key_package)
                .map_err(|e| Error::Crypto(format!("Failed to serialize KeyPackage: {:?}", e)))?;

            // Create signature over the KeyPackage bytes
            let signature = self.sign_key_package(&key_package_bytes)?;

            // Store the KeyPackage locally
            self.available_packages.push(key_package);

            // Create bundle
            bundles.push(KeyPackageBundle {
                user_id: self.user_id,
                key_package_bytes,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                signature,
            });
        }

        Ok(bundles)
    }

    /// Sign a KeyPackage for verification
    fn sign_key_package(&self, key_package_bytes: &[u8]) -> Result<Vec<u8>> {
        use ed25519_dalek::Signer;
        
        // Create a signing keypair from the MLS signer
        // Note: This is a placeholder - we need to extract the Ed25519 key
        let signature = vec![0u8; 64]; // TODO: Implement proper signing
        
        Ok(signature)
    }

    /// Verify a KeyPackage bundle signature
    pub fn verify_key_package_bundle(bundle: &KeyPackageBundle) -> Result<()> {
        // TODO: Implement signature verification
        // This prevents malicious KeyPackages from being used
        Ok(())
    }

    /// Deserialize a KeyPackage from a bundle
    pub fn deserialize_key_package(
        bundle: &KeyPackageBundle,
        _provider: &DescordProvider,
    ) -> Result<KeyPackage> {
        // Verify signature first
        Self::verify_key_package_bundle(bundle)?;

        // Deserialize the KeyPackage using serde
        serde_json::from_slice(&bundle.key_package_bytes)
            .map_err(|e| Error::Crypto(format!("Failed to deserialize KeyPackage: {:?}", e)))
    }

    /// Get the number of available KeyPackages
    pub fn available_count(&self) -> usize {
        self.available_packages.len()
    }

    /// Consume a KeyPackage (removes it from available pool)
    pub fn consume_key_package(&mut self) -> Option<KeyPackage> {
        self.available_packages.pop()
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

    #[test]
    fn test_generate_key_packages() {
        let provider = create_provider();
        let user_id = UserId([1u8; 32]);
        let signer = create_test_keypair();
        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

        let mut store = KeyPackageStore::new(user_id, signer, ciphersuite);
        
        // Generate 5 KeyPackages
        let bundles = store.generate_key_packages(5, &provider).unwrap();
        
        assert_eq!(bundles.len(), 5);
        assert_eq!(store.available_count(), 5);
        
        // Each bundle should have the correct user_id
        for bundle in &bundles {
            assert_eq!(bundle.user_id, user_id);
            assert!(!bundle.key_package_bytes.is_empty());
        }
    }

    #[test]
    fn test_consume_key_package() {
        let provider = create_provider();
        let user_id = UserId([1u8; 32]);
        let signer = create_test_keypair();
        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

        let mut store = KeyPackageStore::new(user_id, signer, ciphersuite);
        store.generate_key_packages(3, &provider).unwrap();
        
        assert_eq!(store.available_count(), 3);
        
        // Consume one
        let kp = store.consume_key_package();
        assert!(kp.is_some());
        assert_eq!(store.available_count(), 2);
    }

    #[test]
    fn test_deserialize_key_package() {
        let provider = create_provider();
        let user_id = UserId([1u8; 32]);
        let signer = create_test_keypair();
        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

        let mut store = KeyPackageStore::new(user_id, signer, ciphersuite);
        let bundles = store.generate_key_packages(1, &provider).unwrap();
        
        // Deserialize the bundle
        let kp = KeyPackageStore::deserialize_key_package(&bundles[0], &provider);
        assert!(kp.is_ok());
    }
}
