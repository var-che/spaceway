//! Property-based tests for CRDT convergence
//!
//! Tests that verify CRDT properties hold under various scenarios:
//! - Eventual consistency: all replicas converge to the same state
//! - Commutativity: order of applying operations doesn't matter
//! - Idempotence: applying the same operation multiple times has same effect as once

use proptest::prelude::*;
use crate::crdt::{CrdtOp, OpType, OpPayload, Hlc, OpValidator, ValidationResult};
use crate::crypto::signing::Keypair;
use crate::types::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Generate a valid CRDT operation
fn arb_crdt_op() -> impl Strategy<Value = CrdtOp> {
    let space_id = SpaceId(Uuid::new_v4());
    let keypair = Keypair::generate();
    let user_id = keypair.user_id();
    
    prop::collection::vec(any::<u8>(), 1..100)
        .prop_map(move |content_bytes| {
            let content = String::from_utf8_lossy(&content_bytes).to_string();
            let op_id = OpId(Uuid::new_v4());
            
            let mut op = CrdtOp {
                op_id,
                space_id,
                channel_id: None,
                thread_id: None,
                op_type: OpType::PostMessage(OpPayload::PostMessage {
                    message_id: MessageId(Uuid::new_v4()),
                    content,
                }),
                prev_ops: vec![],
                author: user_id,
                epoch: EpochId(0),
                hlc: Hlc::now(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                signature: Signature([0u8; 64]),
            };
            
            // Sign the operation
            let signing_bytes = op.signing_bytes();
            op.signature = Signature(keypair.sign(&signing_bytes).0);
            
            op
        })
}

/// Replica of a CRDT state
#[derive(Clone)]
struct Replica {
    validator: OpValidator,
    operations: HashMap<OpId, CrdtOp>,
    message_count: usize,
}

impl Replica {
    fn new() -> Self {
        Self {
            validator: OpValidator::new(),
            operations: HashMap::new(),
            message_count: 0,
        }
    }
    
    fn apply(&mut self, op: CrdtOp) -> bool {
        match self.validator.validate(&op, &self.operations) {
            ValidationResult::Accept => {
                if matches!(op.op_type, OpType::PostMessage(_)) {
                    self.message_count += 1;
                }
                self.operations.insert(op.op_id, op.clone());
                self.validator.apply_op(&op);
                true
            }
            _ => false,
        }
    }
    
    fn state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.message_count.hash(&mut hasher);
        
        // Hash sorted operation IDs for determinism
        let mut op_ids: Vec<_> = self.operations.keys().collect();
        op_ids.sort_by_key(|id| id.0);
        for id in op_ids {
            id.0.hash(&mut hasher);
        }
        
        hasher.finish()
    }
}

proptest! {
    /// Test: Applying the same operations in different orders converges
    #[test]
    fn test_commutativity(ops in prop::collection::vec(arb_crdt_op(), 1..10)) {
        let mut replica1 = Replica::new();
        let mut replica2 = Replica::new();
        
        // Apply operations in original order to replica1
        for op in &ops {
            replica1.apply(op.clone());
        }
        
        // Apply operations in reverse order to replica2
        for op in ops.iter().rev() {
            replica2.apply(op.clone());
        }
        
        // Both replicas should converge to the same state
        prop_assert_eq!(replica1.message_count, replica2.message_count);
        prop_assert_eq!(replica1.operations.len(), replica2.operations.len());
    }
    
    /// Test: Applying the same operation multiple times is idempotent
    #[test]
    fn test_idempotence(op in arb_crdt_op()) {
        let mut replica = Replica::new();
        
        // Apply operation once
        let accepted1 = replica.apply(op.clone());
        let count_after_first = replica.message_count;
        
        // Apply same operation again
        let accepted2 = replica.apply(op.clone());
        let count_after_second = replica.message_count;
        
        // First should be accepted, second should be rejected (duplicate)
        prop_assert!(accepted1);
        prop_assert!(!accepted2);
        prop_assert_eq!(count_after_first, count_after_second);
    }
    
    /// Test: Multiple replicas converge when receiving same operations
    #[test]
    fn test_eventual_consistency(ops in prop::collection::vec(arb_crdt_op(), 1..20)) {
        let mut replica1 = Replica::new();
        let mut replica2 = Replica::new();
        let mut replica3 = Replica::new();
        
        // Apply all operations to all replicas
        for op in &ops {
            replica1.apply(op.clone());
            replica2.apply(op.clone());
            replica3.apply(op.clone());
        }
        
        // All replicas should have the same state
        let hash1 = replica1.state_hash();
        let hash2 = replica2.state_hash();
        let hash3 = replica3.state_hash();
        
        prop_assert_eq!(hash1, hash2);
        prop_assert_eq!(hash2, hash3);
    }
    
    /// Test: Interleaved operations from multiple replicas converge
    #[test]
    fn test_concurrent_operations(
        ops1 in prop::collection::vec(arb_crdt_op(), 1..10),
        ops2 in prop::collection::vec(arb_crdt_op(), 1..10),
    ) {
        let mut replica_a = Replica::new();
        let mut replica_b = Replica::new();
        
        // Replica A generates ops1 and sends to B
        // Replica B generates ops2 and sends to A
        
        // Apply local operations
        for op in &ops1 {
            replica_a.apply(op.clone());
        }
        for op in &ops2 {
            replica_b.apply(op.clone());
        }
        
        // Exchange operations
        for op in &ops2 {
            replica_a.apply(op.clone());
        }
        for op in &ops1 {
            replica_b.apply(op.clone());
        }
        
        // Both replicas should converge
        prop_assert_eq!(replica_a.state_hash(), replica_b.state_hash());
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[test]
    fn test_replica_basic() {
        let mut replica = Replica::new();
        let keypair = Keypair::generate();
        
        let mut op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id: SpaceId(Uuid::new_v4()),
            channel_id: None,
            thread_id: None,
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id: MessageId(Uuid::new_v4()),
                content: "Test".to_string(),
            }),
            prev_ops: vec![],
            author: keypair.user_id(),
            epoch: EpochId(0),
            hlc: Hlc::now(),
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(keypair.sign(&signing_bytes).0);
        
        assert!(replica.apply(op));
        assert_eq!(replica.message_count, 1);
    }
    
    #[test]
    fn test_duplicate_detection() {
        let mut replica = Replica::new();
        let keypair = Keypair::generate();
        
        let mut op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id: SpaceId(Uuid::new_v4()),
            channel_id: None,
            thread_id: None,
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id: MessageId(Uuid::new_v4()),
                content: "Test".to_string(),
            }),
            prev_ops: vec![],
            author: keypair.user_id(),
            epoch: EpochId(0),
            hlc: Hlc::now(),
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(keypair.sign(&signing_bytes).0);
        
        assert!(replica.apply(op.clone()));
        assert!(!replica.apply(op)); // Duplicate should be rejected
    }
}
