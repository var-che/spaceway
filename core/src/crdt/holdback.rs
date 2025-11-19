//! Holdback queue for buffered CRDT operations
//!
//! Manages operations that arrive before their dependencies (prev_ops)
//! or before the local node has processed the required MLS epoch.
//!
//! The holdback queue implements efficient dependency resolution:
//! - Operations are indexed by their dependencies
//! - When a dependency is satisfied, all blocked operations are checked
//! - Prevents infinite buffering with configurable limits

use crate::crdt::CrdtOp;
use crate::types::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Maximum number of operations to buffer before rejecting new ones
const MAX_BUFFERED_OPS: usize = 10000;

/// Maximum time (in seconds) an operation can stay buffered
const MAX_BUFFER_TIME_SECS: u64 = 300; // 5 minutes

/// Holdback queue for operations awaiting dependencies
pub struct HoldbackQueue {
    /// Operations indexed by op_id for quick lookup
    buffered_ops: HashMap<OpId, BufferedOp>,
    
    /// Index: dependency_id -> set of op_ids waiting for it
    waiting_for: HashMap<OpId, HashSet<OpId>>,
    
    /// Operations waiting for a specific epoch (space_id, epoch) -> op_ids
    waiting_for_epoch: HashMap<(SpaceId, EpochId), HashSet<OpId>>,
    
    /// FIFO queue for expiration checking
    insertion_order: VecDeque<OpId>,
}

/// A buffered operation with metadata
#[derive(Debug, Clone)]
struct BufferedOp {
    /// The CRDT operation
    op: CrdtOp,
    
    /// Timestamp when buffered (for expiration)
    buffered_at: u64,
    
    /// Missing dependencies (op_ids)
    missing_deps: HashSet<OpId>,
    
    /// Waiting for epoch (if any)
    waiting_epoch: Option<EpochId>,
}

impl HoldbackQueue {
    pub fn new() -> Self {
        Self {
            buffered_ops: HashMap::new(),
            waiting_for: HashMap::new(),
            waiting_for_epoch: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }

    /// Buffer an operation that has missing dependencies
    ///
    /// Returns Ok(()) if buffered successfully, Err if queue is full
    pub fn buffer(
        &mut self,
        op: CrdtOp,
        missing_deps: Vec<OpId>,
        current_time: u64,
    ) -> Result<(), String> {
        if self.buffered_ops.len() >= MAX_BUFFERED_OPS {
            return Err("Holdback queue full".to_string());
        }

        let op_id = op.op_id;
        let missing_set: HashSet<OpId> = missing_deps.into_iter().collect();

        // Index by dependencies
        for dep_id in &missing_set {
            self.waiting_for
                .entry(*dep_id)
                .or_insert_with(HashSet::new)
                .insert(op_id);
        }

        // Store the buffered operation
        self.buffered_ops.insert(
            op_id,
            BufferedOp {
                op,
                buffered_at: current_time,
                missing_deps: missing_set,
                waiting_epoch: None,
            },
        );

        self.insertion_order.push_back(op_id);

        Ok(())
    }

    /// Buffer an operation waiting for a future epoch
    pub fn buffer_for_epoch(
        &mut self,
        op: CrdtOp,
        epoch: EpochId,
        current_time: u64,
    ) -> Result<(), String> {
        if self.buffered_ops.len() >= MAX_BUFFERED_OPS {
            return Err("Holdback queue full".to_string());
        }

        let op_id = op.op_id;
        let space_id = op.space_id;

        // Index by epoch
        self.waiting_for_epoch
            .entry((space_id, epoch))
            .or_insert_with(HashSet::new)
            .insert(op_id);

        // Store the buffered operation
        self.buffered_ops.insert(
            op_id,
            BufferedOp {
                op,
                buffered_at: current_time,
                missing_deps: HashSet::new(),
                waiting_epoch: Some(epoch),
            },
        );

        self.insertion_order.push_back(op_id);

        Ok(())
    }

    /// Notify that an operation has been accepted
    ///
    /// Returns operations that are now ready (all dependencies satisfied)
    pub fn on_op_accepted(&mut self, op_id: OpId) -> Vec<CrdtOp> {
        let mut ready = Vec::new();

        // Find operations waiting for this dependency
        if let Some(waiting_ops) = self.waiting_for.remove(&op_id) {
            for waiting_op_id in waiting_ops {
                if let Some(buffered) = self.buffered_ops.get_mut(&waiting_op_id) {
                    // Remove this dependency
                    buffered.missing_deps.remove(&op_id);

                    // If all dependencies satisfied, mark as ready
                    if buffered.missing_deps.is_empty() && buffered.waiting_epoch.is_none() {
                        ready.push(buffered.op.clone());
                    }
                }
            }
        }

        // Remove ready operations from buffer
        for op in &ready {
            self.remove_op(op.op_id);
        }

        ready
    }

    /// Notify that a space has advanced to a new epoch
    ///
    /// Returns operations that were waiting for this epoch
    pub fn on_epoch_updated(&mut self, space_id: SpaceId, epoch: EpochId) -> Vec<CrdtOp> {
        let mut ready = Vec::new();

        // Collect all epochs <= the new epoch for this space
        let epochs_to_check: Vec<EpochId> = self.waiting_for_epoch
            .keys()
            .filter(|(sid, e)| *sid == space_id && e.0 <= epoch.0)
            .map(|(_, e)| *e)
            .collect();

        for old_epoch in epochs_to_check {
            if let Some(waiting_ops) = self.waiting_for_epoch.remove(&(space_id, old_epoch)) {
                for waiting_op_id in waiting_ops {
                    if let Some(buffered) = self.buffered_ops.get_mut(&waiting_op_id) {
                        // Clear epoch wait
                        buffered.waiting_epoch = None;

                        // If all dependencies satisfied, mark as ready
                        if buffered.missing_deps.is_empty() {
                            ready.push(buffered.op.clone());
                        }
                    }
                }
            }
        }

        // Remove ready operations from buffer
        for op in &ready {
            self.remove_op(op.op_id);
        }

        ready
    }

    /// Remove expired operations
    ///
    /// Returns operations that have been buffered longer than MAX_BUFFER_TIME_SECS
    pub fn expire_old_ops(&mut self, current_time: u64) -> Vec<CrdtOp> {
        let mut expired = Vec::new();

        // Check from front of queue (oldest first)
        while let Some(&op_id) = self.insertion_order.front() {
            if let Some(buffered) = self.buffered_ops.get(&op_id) {
                if current_time - buffered.buffered_at > MAX_BUFFER_TIME_SECS {
                    expired.push(buffered.op.clone());
                    self.insertion_order.pop_front();
                    self.remove_op(op_id);
                } else {
                    // Queue is ordered, so stop at first non-expired
                    break;
                }
            } else {
                // Already removed, skip
                self.insertion_order.pop_front();
            }
        }

        expired
    }

    /// Remove an operation from all indexes
    fn remove_op(&mut self, op_id: OpId) {
        if let Some(buffered) = self.buffered_ops.remove(&op_id) {
            // Remove from dependency indexes
            for dep_id in &buffered.missing_deps {
                if let Some(waiting_set) = self.waiting_for.get_mut(dep_id) {
                    waiting_set.remove(&op_id);
                    if waiting_set.is_empty() {
                        self.waiting_for.remove(dep_id);
                    }
                }
            }

            // Remove from epoch index
            if let Some(epoch) = buffered.waiting_epoch {
                let key = (buffered.op.space_id, epoch);
                if let Some(waiting_set) = self.waiting_for_epoch.get_mut(&key) {
                    waiting_set.remove(&op_id);
                    if waiting_set.is_empty() {
                        self.waiting_for_epoch.remove(&key);
                    }
                }
            }
        }
    }

    /// Get current queue size
    pub fn len(&self) -> usize {
        self.buffered_ops.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.buffered_ops.is_empty()
    }

    /// Get all buffered operations (for debugging)
    pub fn buffered_ops(&self) -> Vec<&CrdtOp> {
        self.buffered_ops.values().map(|b| &b.op).collect()
    }
}

impl Default for HoldbackQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::{OpType, OpPayload, Hlc};
    use uuid::Uuid;

    fn create_test_op(op_id: OpId, space_id: SpaceId, prev_ops: Vec<OpId>) -> CrdtOp {
        CrdtOp {
            op_id,
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id: MessageId(Uuid::new_v4()),
                content: "Test message".to_string(),
            }),
            prev_ops,
            author: UserId([1u8; 32]),
            epoch: EpochId(0),
            hlc: Hlc { wall_time: 1000, logical: 0 },
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        }
    }

    #[test]
    fn test_buffer_and_accept() {
        let mut queue = HoldbackQueue::new();
        
        let dep_id = OpId(Uuid::new_v4());
        let op_id = OpId(Uuid::new_v4());
        let space_id = SpaceId(Uuid::new_v4());
        
        let op = create_test_op(op_id, space_id, vec![dep_id]);
        
        // Buffer operation waiting for dependency
        queue.buffer(op, vec![dep_id], 1000).unwrap();
        assert_eq!(queue.len(), 1);
        
        // Notify dependency accepted
        let ready = queue.on_op_accepted(dep_id);
        
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].op_id, op_id);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_buffer_for_epoch() {
        let mut queue = HoldbackQueue::new();
        
        let op_id = OpId(Uuid::new_v4());
        let space_id = SpaceId(Uuid::new_v4());
        
        let op = create_test_op(op_id, space_id, vec![]);
        
        // Buffer for epoch 5
        queue.buffer_for_epoch(op, EpochId(5), 1000).unwrap();
        assert_eq!(queue.len(), 1);
        
        // Update to epoch 5
        let ready = queue.on_epoch_updated(space_id, EpochId(5));
        
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].op_id, op_id);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_expire_old_ops() {
        let mut queue = HoldbackQueue::new();
        
        let op_id = OpId(Uuid::new_v4());
        let dep_id = OpId(Uuid::new_v4());
        let space_id = SpaceId(Uuid::new_v4());
        
        let op = create_test_op(op_id, space_id, vec![dep_id]);
        
        // Buffer at time 1000
        queue.buffer(op, vec![dep_id], 1000).unwrap();
        
        // Check expiration at 1000 + MAX_BUFFER_TIME_SECS + 1
        let expired = queue.expire_old_ops(1000 + MAX_BUFFER_TIME_SECS + 1);
        
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].op_id, op_id);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_multiple_dependencies() {
        let mut queue = HoldbackQueue::new();
        
        let dep1 = OpId(Uuid::new_v4());
        let dep2 = OpId(Uuid::new_v4());
        let op_id = OpId(Uuid::new_v4());
        let space_id = SpaceId(Uuid::new_v4());
        
        let op = create_test_op(op_id, space_id, vec![dep1, dep2]);
        
        // Buffer with 2 dependencies
        queue.buffer(op, vec![dep1, dep2], 1000).unwrap();
        
        // Accept first dependency - not ready yet
        let ready = queue.on_op_accepted(dep1);
        assert_eq!(ready.len(), 0);
        assert_eq!(queue.len(), 1);
        
        // Accept second dependency - now ready
        let ready = queue.on_op_accepted(dep2);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].op_id, op_id);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_queue_full() {
        let mut queue = HoldbackQueue::new();
        let space_id = SpaceId(Uuid::new_v4());
        let dep_id = OpId(Uuid::new_v4());
        
        // Fill queue to capacity
        for _ in 0..MAX_BUFFERED_OPS {
            let op = create_test_op(OpId(Uuid::new_v4()), space_id, vec![dep_id]);
            queue.buffer(op, vec![dep_id], 1000).unwrap();
        }
        
        // Try to add one more - should fail
        let op = create_test_op(OpId(Uuid::new_v4()), space_id, vec![dep_id]);
        let result = queue.buffer(op, vec![dep_id], 1000);
        assert!(result.is_err());
    }
}
