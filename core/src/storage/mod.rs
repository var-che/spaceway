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
use crate::types::{ThreadId, MessageId};
use serde::{Serialize, Deserialize};
use zeroize::Zeroizing;

pub use blob::EncryptedBlob;
pub use dht_blob::{DhtBlob, BlobIndex};
pub use indices::{BlobMetadata, MessageIndex};
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
    const CF_MESSAGES: &'static str = "messages";
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
            ColumnFamilyDescriptor::new(Self::CF_MESSAGES, Options::default()),
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

    /// Store an encrypted blob and return its hash
    pub fn store_blob(&self, data: &[u8], key: &[u8; 32]) -> Result<BlobHash> {
        // Create encrypted blob
        let encrypted = EncryptedBlob::encrypt(data, key)?;
        let hash = BlobHash::hash(data);
        
        // Write to file in blob directory
        let blob_path = self.blob_dir.join(hash.to_hex());
        encrypted.write_to_file(&blob_path)?;
        
        Ok(hash)
    }
    
    /// Load and decrypt a blob by hash
    pub fn load_blob(&self, hash: &BlobHash, key: &[u8; 32]) -> Result<Vec<u8>> {
        let blob_path = self.blob_dir.join(hash.to_hex());
        let encrypted = EncryptedBlob::read_from_file(&blob_path)?;
        encrypted.decrypt(key)
    }
    
    /// Store metadata for a blob
    pub fn store_blob_metadata(&self, hash: &BlobHash, metadata: &BlobMetadata) -> Result<()> {
        let cf = self.db.cf_handle(Self::CF_BLOB_METADATA)
            .ok_or_else(|| anyhow::anyhow!("CF_BLOB_METADATA not found"))?;
        
        let key = hash.to_hex();
        let value = bincode::serialize(metadata)?;
        
        self.db.put_cf(&cf, key.as_bytes(), &value)?;
        Ok(())
    }
    
    /// Get metadata for a blob
    pub fn get_blob_metadata(&self, hash: &BlobHash) -> Result<Option<BlobMetadata>> {
        let cf = self.db.cf_handle(Self::CF_BLOB_METADATA)
            .ok_or_else(|| anyhow::anyhow!("CF_BLOB_METADATA not found"))?;
        
        let key = hash.to_hex();
        match self.db.get_cf(&cf, key.as_bytes())? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
    
    /// Get a message blob by message ID
    pub fn get_message_blob(&self, message_id: &MessageId) -> Result<Option<BlobHash>> {
        let cf = self.db.cf_handle(Self::CF_MESSAGES)
            .ok_or_else(|| anyhow::anyhow!("CF_MESSAGES not found"))?;
        
        let key = message_id.as_bytes();
        match self.db.get_cf(&cf, key)? {
            Some(bytes) => {
                // Stored as hex string
                let hex = String::from_utf8(bytes)?;
                Ok(Some(BlobHash::from_hex(&hex)?))
            }
            None => Ok(None),
        }
    }
    
    /// Index a message in thread and user message indices
    pub fn index_message(&self, index: &MessageIndex) -> Result<()> {
        // Store in thread messages index
        let thread_cf = self.db.cf_handle(Self::CF_THREAD_MESSAGES)
            .ok_or_else(|| anyhow::anyhow!("CF_THREAD_MESSAGES not found"))?;
        
        // Key: thread_id || timestamp || message_id
        let mut thread_key = Vec::new();
        thread_key.extend_from_slice(index.thread_id.as_bytes());
        thread_key.extend_from_slice(&index.timestamp.to_be_bytes());
        thread_key.extend_from_slice(index.message_id.as_bytes());
        
        let value = bincode::serialize(index)?;
        self.db.put_cf(&thread_cf, &thread_key, &value)?;
        
        // Store in user messages index
        let user_cf = self.db.cf_handle(Self::CF_USER_MESSAGES)
            .ok_or_else(|| anyhow::anyhow!("CF_USER_MESSAGES not found"))?;
        
        // Key: user_id || timestamp || message_id
        let mut user_key = Vec::new();
        user_key.extend_from_slice(index.author.as_bytes());
        user_key.extend_from_slice(&index.timestamp.to_be_bytes());
        user_key.extend_from_slice(index.message_id.as_bytes());
        
        self.db.put_cf(&user_cf, &user_key, &value)?;
        
        // Store message_id -> blob_hash mapping
        let msg_cf = self.db.cf_handle(Self::CF_MESSAGES)
            .ok_or_else(|| anyhow::anyhow!("CF_MESSAGES not found"))?;
        
        let blob_hex = index.blob_hash.to_hex();
        self.db.put_cf(&msg_cf, index.message_id.as_bytes(), blob_hex.as_bytes())?;
        
        Ok(())
    }
    
    /// Get messages in a thread, ordered by timestamp
    pub fn get_thread_messages(&self, thread_id: &ThreadId, limit: usize) -> Result<Vec<MessageIndex>> {
        let cf = self.db.cf_handle(Self::CF_THREAD_MESSAGES)
            .ok_or_else(|| anyhow::anyhow!("CF_THREAD_MESSAGES not found"))?;
        
        let prefix = thread_id.as_bytes();
        let iter = self.db.prefix_iterator_cf(&cf, prefix);
        
        let mut messages = Vec::new();
        for item in iter.take(limit) {
            let (_key, value) = item?;
            let index: MessageIndex = bincode::deserialize(&value)?;
            messages.push(index);
        }
        
        Ok(messages)
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