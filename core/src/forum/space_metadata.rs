//! Space metadata for DHT storage
//!
//! This module defines serializable Space metadata that can be stored in the DHT
//! to enable offline Space discovery and joining.

use crate::types::*;
use crate::{Error, Result};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use sha2::{Sha256, Digest};

/// Serializable Space metadata for DHT storage
#[derive(Clone, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct SpaceMetadata {
    /// Space ID (content-addressed)
    #[n(0)]
    pub id: SpaceId,
    
    /// Display name
    #[n(1)]
    pub name: String,
    
    /// Optional description
    #[n(2)]
    pub description: Option<String>,
    
    /// Creator/owner
    #[n(3)]
    pub owner: UserId,
    
    /// Visibility settings
    #[n(4)]
    pub visibility: SpaceVisibility,
    
    /// Initial member list (at creation time)
    #[n(5)]
    pub initial_members: HashMap<UserId, Role>,
    
    /// Invite permissions
    #[n(6)]
    pub invite_permissions: InvitePermissions,
    
    /// Current MLS epoch
    #[n(7)]
    pub epoch: EpochId,
    
    /// Creation timestamp
    #[n(8)]
    pub created_at: u64,
    
    /// Ed25519 signature (owner signs the metadata)
    #[n(9)]
    pub signature: Signature,
}

impl SpaceMetadata {
    /// Create metadata from a Space
    pub fn from_space(space: &crate::forum::space::Space, keypair: &ed25519_dalek::SigningKey) -> Self {
        let mut metadata = Self {
            id: space.id,
            name: space.name.clone(),
            description: space.description.clone(),
            owner: space.owner,
            visibility: space.visibility,
            initial_members: space.members.clone(),
            invite_permissions: space.invite_permissions.clone(),
            epoch: space.epoch,
            created_at: space.created_at,
            signature: Signature([0u8; 64]), // Temporary
        };
        
        // Sign the metadata
        let signing_bytes = metadata.signing_bytes();
        use ed25519_dalek::Signer;
        let sig = keypair.sign(&signing_bytes);
        metadata.signature = Signature(sig.to_bytes());
        
        metadata
    }
    
    /// Get bytes to sign (all fields except signature)
    fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.id.as_bytes());
        buf.extend_from_slice(self.name.as_bytes());
        if let Some(desc) = &self.description {
            buf.extend_from_slice(desc.as_bytes());
        }
        buf.extend_from_slice(self.owner.as_bytes());
        buf.extend_from_slice(&[self.visibility as u8]);
        buf.extend_from_slice(&self.epoch.0.to_le_bytes());
        buf.extend_from_slice(&self.created_at.to_le_bytes());
        buf
    }
    
    /// Verify signature
    pub fn verify_signature(&self) -> bool {
        use ed25519_dalek::Verifier;
        
        let signing_bytes = self.signing_bytes();
        let public_key = match ed25519_dalek::VerifyingKey::from_bytes(&self.owner.0) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        let signature = ed25519_dalek::Signature::from_bytes(&self.signature.0);
        
        public_key.verify(&signing_bytes, &signature).is_ok()
    }
    
    /// Serialize to CBOR bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        minicbor::to_vec(self)
            .map_err(|e| Error::Serialization(format!("Failed to encode SpaceMetadata: {}", e)))
    }
    
    /// Deserialize from CBOR bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to decode SpaceMetadata: {}", e)))
    }
}

/// Encrypted Space metadata for DHT storage
#[derive(Clone, Debug, Encode, Decode)]
pub struct EncryptedSpaceMetadata {
    /// Space ID (plaintext for lookups)
    #[n(0)]
    pub space_id: SpaceId,
    
    /// Nonce for AES-GCM encryption
    #[n(1)]
    pub nonce: [u8; 12],
    
    /// Encrypted metadata (CBOR-serialized SpaceMetadata)
    #[n(2)]
    pub ciphertext: Vec<u8>,
    
    /// Visibility (plaintext to determine if decryption key should be shared)
    #[n(3)]
    pub visibility: SpaceVisibility,
}

impl EncryptedSpaceMetadata {
    /// Encrypt Space metadata for DHT storage
    /// 
    /// Uses a key derived from the Space ID for Public/Private spaces.
    /// For Hidden spaces, uses a key that must be shared out-of-band.
    pub fn encrypt(metadata: &SpaceMetadata) -> Result<Self> {
        // Derive encryption key from Space ID
        // For Public/Private: Anyone with the Space ID can decrypt
        // For Hidden: The Space ID itself should be kept secret
        let key_bytes = Self::derive_key(&metadata.id);
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        use rand::Rng;
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Serialize and encrypt
        let plaintext = metadata.to_bytes()?;
        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to encrypt metadata: {}", e)))?;
        
        Ok(Self {
            space_id: metadata.id,
            nonce: nonce_bytes,
            ciphertext,
            visibility: metadata.visibility,
        })
    }
    
    /// Decrypt Space metadata
    pub fn decrypt(&self) -> Result<SpaceMetadata> {
        // Derive decryption key from Space ID
        let key_bytes = Self::derive_key(&self.space_id);
        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        
        let nonce = Nonce::from_slice(&self.nonce);
        
        // Decrypt and deserialize
        let plaintext = cipher.decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|e| Error::Crypto(format!("Failed to decrypt metadata: {}", e)))?;
        
        SpaceMetadata::from_bytes(&plaintext)
    }
    
    /// Derive encryption key from Space ID
    fn derive_key(space_id: &SpaceId) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_SPACE_METADATA_KEY_V1:");
        hasher.update(space_id.as_bytes());
        let hash = hasher.finalize();
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        key
    }
    
    /// Serialize to CBOR bytes for DHT storage
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        minicbor::to_vec(self)
            .map_err(|e| Error::Serialization(format!("Failed to encode EncryptedSpaceMetadata: {}", e)))
    }
    
    /// Deserialize from CBOR bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        minicbor::decode(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to decode EncryptedSpaceMetadata: {}", e)))
    }
    
    /// Get DHT key for this Space (hash of Space ID)
    pub fn dht_key(space_id: &SpaceId) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"DESCORD_SPACE_DHT_KEY:");
        hasher.update(space_id.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_space_metadata_serialization() {
        let user_id = UserId([1u8; 32]);
        let space_id = SpaceId([2u8; 32]);
        let keypair = ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]);
        
        let mut members = HashMap::new();
        members.insert(user_id, Role::Admin);
        
        let mut metadata = SpaceMetadata {
            id: space_id,
            name: "Test Space".to_string(),
            description: Some("Test description".to_string()),
            owner: user_id,
            visibility: SpaceVisibility::Public,
            initial_members: members,
            invite_permissions: InvitePermissions::default(),
            epoch: EpochId(0),
            created_at: 1234567890,
            signature: Signature([0u8; 64]),
        };
        
        // Sign
        use ed25519_dalek::Signer;
        let signing_bytes = metadata.signing_bytes();
        let sig = keypair.sign(&signing_bytes);
        metadata.signature = Signature(sig.to_bytes());
        
        // Serialize and deserialize
        let bytes = metadata.to_bytes().unwrap();
        let decoded = SpaceMetadata::from_bytes(&bytes).unwrap();
        
        assert_eq!(metadata.id, decoded.id);
        assert_eq!(metadata.name, decoded.name);
        assert_eq!(metadata.description, decoded.description);
        assert!(decoded.verify_signature());
    }
    
    #[test]
    fn test_encrypted_space_metadata() {
        let user_id = UserId([1u8; 32]);
        let space_id = SpaceId([2u8; 32]);
        let keypair = ed25519_dalek::SigningKey::from_bytes(&[3u8; 32]);
        
        let mut members = HashMap::new();
        members.insert(user_id, Role::Admin);
        
        let mut metadata = SpaceMetadata {
            id: space_id,
            name: "Secret Space".to_string(),
            description: Some("Very secret".to_string()),
            owner: user_id,
            visibility: SpaceVisibility::Hidden,
            initial_members: members,
            invite_permissions: InvitePermissions::default(),
            epoch: EpochId(0),
            created_at: 1234567890,
            signature: Signature([0u8; 64]),
        };
        
        // Sign
        use ed25519_dalek::Signer;
        let signing_bytes = metadata.signing_bytes();
        let sig = keypair.sign(&signing_bytes);
        metadata.signature = Signature(sig.to_bytes());
        
        // Encrypt
        let encrypted = EncryptedSpaceMetadata::encrypt(&metadata).unwrap();
        
        // Decrypt
        let decrypted = encrypted.decrypt().unwrap();
        
        assert_eq!(metadata.name, decrypted.name);
        assert_eq!(metadata.description, decrypted.description);
        assert!(decrypted.verify_signature());
    }
    
    #[test]
    fn test_dht_key_derivation() {
        let space_id = SpaceId([42u8; 32]);
        let key = EncryptedSpaceMetadata::dht_key(&space_id);
        
        assert_eq!(key.len(), 32);
        
        // Same ID should give same key
        let key2 = EncryptedSpaceMetadata::dht_key(&space_id);
        assert_eq!(key, key2);
        
        // Different ID should give different key
        let other_id = SpaceId([43u8; 32]);
        let other_key = EncryptedSpaceMetadata::dht_key(&other_id);
        assert_ne!(key, other_key);
    }
}
