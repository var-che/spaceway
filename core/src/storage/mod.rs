///! Storage module - Phase 1, 2, 3 Implementation
///!
///! Provides persistent storage using RocksDB for:
///! - Encrypted message blobs (AES-256-GCM)
///! - Message indices (thread, user, message ID)
///! - CRDT state (vector clocks, tombstones)
///! - Relay cache and reputation
///! - Lazy loading and pagination
///! - LZ4 compression

pub mod blob;
pub mod store;
pub mod indices;
pub mod crdt;
pub mod sync;
pub mod lazy;
pub mod relay_cache;
pub mod compression;
pub mod dht_blob;

use anyhow::{Context, Result, anyhow};
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use sha2::{Sha256, Digest};
use hkdf::Hkdf;
use std::path::{Path, PathBuf};
use std::fs;
use crate::types::ThreadId;
use serde::{Serialize, Deserialize};
use zeroize::Zeroizing;

pub use blob::EncryptedBlob;
pub use dht_blob::{DhtBlob, BlobIndex};
pub use indices::BlobMetadata;
pub use crdt::{VectorClock, TombstoneSet};
pub use store::Store;
pub use sync::{SyncRequest, SyncResponse, SyncMessage};
pub use lazy::{ThreadPreview, MessageCursor, MessagePage};
pub use relay_cache::RelayStats;

/// Content-addressed blob hash (SHA256)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, minicbor::Encode, minicbor::Decode)]
pub struct BlobHash(#[n(0)] pub [u8; 32]);

impl BlobHash {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Compute hash of data
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }

    /// Convert to hex string for filesystem
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex::decode(hex)
            .context("Invalid hex string")?;
        if bytes.len() != 32 {
            return Err(anyhow!("Hash must be 32 bytes, got {}", bytes.len()));
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }
}

/// Storage manager
pub struct Storage {
    /// RocksDB instance
    db: DB,
    /// Blob storage directory
    blob_dir: PathBuf,
}

impl Storage {
    /// Column family names
    const CF_THREAD_MESSAGES: &'static str = "thread_messages";
    const CF_USER_MESSAGES: &'static str = "user_messages";
    const CF_BLOB_METADATA: &'static str = "blob_metadata";
    const CF_MESSAGE_REFS: &'static str = "message_refs";
    const CF_VECTOR_CLOCKS: &'static str = "vector_clocks";
    const CF_TOMBSTONES: &'static str = "tombstones";
    const CF_RELAYS: &'static str = "relays";

    /// Open storage at the given path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        
        // Create directory structure
        let db_path = path.join("db");
        let blob_dir = path.join("blobs");
        
        fs::create_dir_all(&db_path)
            .context("Failed to create database directory")?;
        fs::create_dir_all(&blob_dir)
            .context("Failed to create blob directory")?;

        // Configure RocksDB options
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Define column families
        let cfs = vec![
            ColumnFamilyDescriptor::new(Self::CF_THREAD_MESSAGES, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_USER_MESSAGES, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_BLOB_METADATA, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_MESSAGE_REFS, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_VECTOR_CLOCKS, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_TOMBSTONES, Options::default()),
            ColumnFamilyDescriptor::new(Self::CF_RELAYS, Options::default()),
        ];

        // Open database
        let db = DB::open_cf_descriptors(&opts, &db_path, cfs)
            .context("Failed to open RocksDB")?;

        Ok(Self {
            db,
            blob_dir,
        })
    }

    /// Get the blob directory path
    pub fn blob_dir(&self) -> &Path {
        &self.blob_dir
    }

    /// Close the database (explicit cleanup)
    pub fn close(self) -> Result<()> {
        drop(self.db);
        Ok(())
    }
}

/// Derive a thread-specific encryption key from MLS group secret
pub fn derive_thread_key(mls_secret: &[u8], thread_id: &ThreadId) -> Zeroizing<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(Some(thread_id.as_bytes()), mls_secret);
    let mut key = Zeroizing::new([0u8; 32]);
    hkdf.expand(b"descord-thread-v1", key.as_mut())
        .expect("HKDF expand failed");
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_storage_initialization() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        // Verify directory structure
        assert!(temp_dir.path().join("db").exists());
        assert!(temp_dir.path().join("blobs").exists());
        
        storage.close()?;
        Ok(())
    }

    #[test]
    fn test_blob_hash_roundtrip() -> Result<()> {
        let data = b"Hello, world!";
        let hash = BlobHash::hash(data);
        
        // Convert to hex and back
        let hex = hash.to_hex();
        let parsed = BlobHash::from_hex(&hex)?;
        
        assert_eq!(hash, parsed);
        Ok(())
    }

    #[test]
    fn test_derive_thread_key() -> Result<()> {
        let mls_secret = b"test_mls_secret_32_bytes_long!!";
        let thread_id = ThreadId::new();
        
        // Derive key
        let key1 = derive_thread_key(mls_secret, &thread_id);
        let key2 = derive_thread_key(mls_secret, &thread_id);
        
        // Same inputs = same key
        assert_eq!(&*key1, &*key2);
        
        // Different thread = different key
        let thread_id2 = ThreadId::new();
        let key3 = derive_thread_key(mls_secret, &thread_id2);
        assert_ne!(&*key1, &*key3);
        
        Ok(())
    }

    #[test]
    fn test_blob_hash_deterministic() {
        let data = b"Test message content";
        let hash1 = BlobHash::hash(data);
        let hash2 = BlobHash::hash(data);
        
        // Same content = same hash
        assert_eq!(hash1, hash2);
        
        // Different content = different hash
        let hash3 = BlobHash::hash(b"Different content");
        assert_ne!(hash1, hash3);
    }
}

