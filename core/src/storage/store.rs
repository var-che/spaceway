//! RocksDB-based persistent storage

use crate::{Error, Result};
use crate::types::*;
use crate::crdt::CrdtOp;
use rocksdb::{DB, Options, IteratorMode};
use std::path::Path;

/// Main storage interface
pub struct Store {
    db: DB,
}

impl Store {
    /// Open or create a store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        let db = DB::open(&opts, path)
            .map_err(|e| Error::Storage(format!("Failed to open database: {}", e)))?;
        
        Ok(Self { db })
    }

    /// Store a CRDT operation
    pub fn put_op(&self, op: &CrdtOp) -> Result<()> {
        let value = minicbor::to_vec(op)
            .map_err(|e| Error::Serialization(format!("Failed to encode op: {}", e)))?;
        
        // Store by op_id for deduplication lookups
        let op_key = self.op_key(&op.op_id);
        self.db
            .put(&op_key, &value)
            .map_err(|e| Error::Storage(format!("Failed to store op by id: {}", e)))?;
        
        // ALSO store by space_id for space-wide queries
        let mut space_key = self.space_prefix(&op.space_id);
        space_key.extend_from_slice(op.op_id.0.as_bytes());
        self.db
            .put(&space_key, &value)
            .map_err(|e| Error::Storage(format!("Failed to store op by space: {}", e)))?;
        
        Ok(())
    }

    /// Get a CRDT operation by ID
    pub fn get_op(&self, op_id: &OpId) -> Result<Option<CrdtOp>> {
        let key = self.op_key(op_id);
        
        match self.db.get(&key) {
            Ok(Some(value)) => {
                let op = minicbor::decode(&value)
                    .map_err(|e| Error::Serialization(format!("Failed to decode op: {}", e)))?;
                Ok(Some(op))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Storage(format!("Failed to get op: {}", e))),
        }
    }

    /// Get all operations for a space
    pub fn get_space_ops(&self, space_id: &SpaceId) -> Result<Vec<CrdtOp>> {
        let prefix = self.space_prefix(space_id);
        let mut ops = Vec::new();
        
        let iter = self.db.iterator(IteratorMode::From(&prefix, rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item
                .map_err(|e| Error::Storage(format!("Iterator error: {}", e)))?;
            
            // Stop if we've moved past this space's prefix
            if !key.starts_with(&prefix) {
                break;
            }
            
            let op: CrdtOp = minicbor::decode(&value)
                .map_err(|e| Error::Serialization(format!("Failed to decode op: {}", e)))?;
            ops.push(op);
        }
        
        Ok(ops)
    }

    /// Store a content blob
    pub fn put_blob(&self, hash: &ContentHash, data: &[u8]) -> Result<()> {
        let key = self.blob_key(hash);
        
        self.db
            .put(&key, data)
            .map_err(|e| Error::Storage(format!("Failed to store blob: {}", e)))?;
        
        Ok(())
    }

    /// Get a content blob
    pub fn get_blob(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>> {
        let key = self.blob_key(hash);
        
        self.db
            .get(&key)
            .map_err(|e| Error::Storage(format!("Failed to get blob: {}", e)))
    }

    // Key construction helpers
    fn op_key(&self, op_id: &OpId) -> Vec<u8> {
        let mut key = b"op:".to_vec();
        key.extend_from_slice(op_id.0.as_bytes());
        key
    }

    fn space_prefix(&self, space_id: &SpaceId) -> Vec<u8> {
        let mut prefix = b"space:".to_vec();
        prefix.extend_from_slice(&space_id.0);
        prefix.push(b':');
        prefix
    }

    fn blob_key(&self, hash: &ContentHash) -> Vec<u8> {
        let mut key = b"blob:".to_vec();
        key.extend_from_slice(&hash.0);
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::{OpType, OpPayload, Hlc};
    use uuid::Uuid;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve_op() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::open(temp_dir.path()).unwrap();
        
        let op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id: SpaceId::new(),
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateSpace(OpPayload::CreateSpace {
                name: "Test".to_string(),
                description: None,
            }),
            prev_ops: vec![],
            author: UserId([0u8; 32]),
            epoch: EpochId(0),
            hlc: Hlc { wall_time: 1000, logical: 0 },
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        };
        
        store.put_op(&op).unwrap();
        let retrieved = store.get_op(&op.op_id).unwrap();
        
        assert_eq!(Some(op), retrieved);
    }

    #[test]
    fn test_store_and_retrieve_blob() {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::open(temp_dir.path()).unwrap();
        
        let data = b"Hello, blob storage!";
        let hash = crate::crypto::signing::hash_content(data);
        
        store.put_blob(&hash, data).unwrap();
        let retrieved = store.get_blob(&hash).unwrap();
        
        assert_eq!(Some(data.to_vec()), retrieved);
    }
}
