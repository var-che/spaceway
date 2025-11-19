//! CRDT operation validation and acceptance logic
//!
//! Implements the formal specification from project_desc.md section 5:
//! - Signature verification
//! - Causality checking (prev_ops dependencies)
//! - Membership/epoch validation
//! - Deterministic conflict resolution

use crate::crdt::{CrdtOp, OpType, OpPayload};
use crate::types::*;
use std::collections::{HashMap, HashSet};

/// Validation result for a CRDT operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Operation is valid and can be applied
    Accept,
    /// Operation has missing dependencies (buffered in holdback queue)
    Buffered(Vec<OpId>),
    /// Operation is invalid and should be rejected
    Reject(RejectionReason),
}

/// Reason why an operation was rejected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    /// Signature verification failed
    InvalidSignature,
    /// Author was not a member at the claimed epoch
    InvalidMembership,
    /// Operation from future epoch (node hasn't received MLS Welcome yet)
    FutureEpoch,
    /// Author was removed before this operation's epoch
    AuthorRemoved,
    /// Operation already exists (duplicate)
    Duplicate,
    /// Invalid operation content
    InvalidContent(String),
}

/// CRDT operation validator
///
/// Implements the `accept_op` algorithm from the specification
pub struct OpValidator {
    /// Current MLS epoch for each space
    space_epochs: HashMap<SpaceId, EpochId>,
    
    /// Membership state: Space -> User -> (Epoch joined, Epoch removed)
    /// Epoch removed is None if still a member
    memberships: HashMap<SpaceId, HashMap<UserId, MembershipRecord>>,
    
    /// Operations we've already seen (for deduplication)
    seen_ops: HashSet<OpId>,
}

/// Membership record for epoch-based validation
#[derive(Debug, Clone)]
struct MembershipRecord {
    /// Epoch when user joined
    joined_at: EpochId,
    /// Epoch when user was removed (None if still member)
    removed_at: Option<EpochId>,
    /// Current role
    role: Role,
}

impl OpValidator {
    pub fn new() -> Self {
        Self {
            space_epochs: HashMap::new(),
            memberships: HashMap::new(),
            seen_ops: HashSet::new(),
        }
    }

    /// Validate a CRDT operation according to the formal specification
    ///
    /// This implements the `accept_op(op)` pseudocode from project_desc.md:
    /// 1. Verify signature
    /// 2. Verify causality (check prev_ops)
    /// 3. Verify membership/epoch constraints
    /// 4. Check for duplicates
    pub fn validate(
        &self,
        op: &CrdtOp,
        known_ops: &HashMap<OpId, CrdtOp>,
    ) -> ValidationResult {
        // Step 1: Verify signature
        if !self.verify_signature(op) {
            return ValidationResult::Reject(RejectionReason::InvalidSignature);
        }

        // Step 2: Verify causality - check all prev_ops are known
        let missing_deps: Vec<OpId> = op.prev_ops
            .iter()
            .filter(|&dep_id| !known_ops.contains_key(dep_id))
            .copied()
            .collect();

        if !missing_deps.is_empty() {
            return ValidationResult::Buffered(missing_deps);
        }

        // Step 3: Verify membership/epoch constraints
        let local_epoch = self.space_epochs.get(&op.space_id).copied().unwrap_or(EpochId(0));

        if op.epoch.0 > local_epoch.0 {
            // Operation from future epoch - need to request MLS Welcome
            return ValidationResult::Buffered(vec![]);
        }

        // Check author membership at op.epoch
        if let Some(rejection) = self.check_membership(&op.author, &op.space_id, &op.epoch, local_epoch) {
            return ValidationResult::Reject(rejection);
        }

        // Step 4: Check for duplicates
        if self.seen_ops.contains(&op.op_id) {
            return ValidationResult::Reject(RejectionReason::Duplicate);
        }

        ValidationResult::Accept
    }

    /// Verify the cryptographic signature on an operation
    fn verify_signature(&self, op: &CrdtOp) -> bool {
        let signing_bytes = op.signing_bytes();
        
        // Extract public key from author (UserId is Ed25519 public key)
        let public_key = ed25519_dalek::VerifyingKey::from_bytes(&op.author.0)
            .ok();
        
        if public_key.is_none() {
            return false;
        }

        // Parse signature
        let signature = ed25519_dalek::Signature::from_bytes(&op.signature.0);

        // Verify
        use ed25519_dalek::Verifier;
        public_key.unwrap().verify(&signing_bytes, &signature).is_ok()
    }

    /// Check if author was a member at the operation's epoch
    fn check_membership(
        &self,
        author: &UserId,
        space_id: &SpaceId,
        op_epoch: &EpochId,
        _local_epoch: EpochId,
    ) -> Option<RejectionReason> {
        let space_members = self.memberships.get(space_id)?;
        let member_record = space_members.get(author)?;

        // Check if author had joined by op_epoch
        if member_record.joined_at.0 > op_epoch.0 {
            return Some(RejectionReason::InvalidMembership);
        }

        // Check if author was removed before op_epoch
        if let Some(removed_at) = member_record.removed_at {
            if removed_at.0 <= op_epoch.0 {
                return Some(RejectionReason::AuthorRemoved);
            }
        }

        None
    }

    /// Update validator state after accepting an operation
    pub fn apply_op(&mut self, op: &CrdtOp) {
        self.seen_ops.insert(op.op_id);

        // Update membership state based on operation type
        match &op.op_type {
            OpType::CreateSpace(_) => {
                // Creator becomes first admin
                self.space_epochs.insert(op.space_id, EpochId(0));
                let mut members = HashMap::new();
                members.insert(op.author, MembershipRecord {
                    joined_at: EpochId(0),
                    removed_at: None,
                    role: Role::Admin,
                });
                self.memberships.insert(op.space_id, members);
            }
            
            OpType::RemoveMember(payload) => {
                if let OpPayload::RemoveMember { user_id, .. } = payload {
                    if let Some(space_members) = self.memberships.get_mut(&op.space_id) {
                        if let Some(record) = space_members.get_mut(user_id) {
                            record.removed_at = Some(op.epoch);
                        }
                    }
                }
            }

            OpType::AssignRole(payload) => {
                if let OpPayload::AssignRole { user_id, role, .. } = payload {
                    if let Some(space_members) = self.memberships.get_mut(&op.space_id) {
                        space_members.entry(*user_id).or_insert(MembershipRecord {
                            joined_at: op.epoch,
                            removed_at: None,
                            role: Role::Member,
                        }).role = *role;
                    }
                }
            }
            
            _ => {}
        }
    }

    /// Update the local epoch for a space (when receiving MLS Welcome)
    pub fn update_epoch(&mut self, space_id: SpaceId, epoch: EpochId) {
        self.space_epochs.insert(space_id, epoch);
    }

    /// Add a member to a space at a specific epoch
    pub fn add_member(&mut self, space_id: SpaceId, user_id: UserId, epoch: EpochId, role: Role) {
        let space_members = self.memberships.entry(space_id).or_insert_with(HashMap::new);
        space_members.insert(user_id, MembershipRecord {
            joined_at: epoch,
            removed_at: None,
            role,
        });
    }
}

impl Default for OpValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::Hlc;
    use uuid::Uuid;

    fn create_test_op(
        _author: UserId,
        space_id: SpaceId,
        epoch: EpochId,
        prev_ops: Vec<OpId>,
    ) -> CrdtOp {
        use crate::crypto::signing::Keypair;
        
        // Generate a keypair for valid signatures
        let keypair = Keypair::generate();
        let author_with_pubkey = keypair.user_id();
        
        let mut op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id,
            channel_id: None,
            thread_id: None,
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id: MessageId(Uuid::new_v4()),
                content: "Test message".to_string(),
            }),
            prev_ops,
            author: author_with_pubkey,
            epoch,
            hlc: Hlc { wall_time: 1000, logical: 0 },
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        };
        
        // Sign the operation properly
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(keypair.sign(&signing_bytes).0);
        
        op
    }

    #[test]
    fn test_validate_missing_dependencies() {
        let validator = OpValidator::new();
        let known_ops = HashMap::new();
        
        let missing_dep = OpId(Uuid::new_v4());
        let op = create_test_op(
            UserId([1u8; 32]),
            SpaceId(Uuid::new_v4()),
            EpochId(0),
            vec![missing_dep],
        );

        match validator.validate(&op, &known_ops) {
            ValidationResult::Buffered(deps) => {
                assert_eq!(deps.len(), 1);
                assert_eq!(deps[0], missing_dep);
            }
            _ => panic!("Expected buffered result"),
        }
    }

    #[test]
    fn test_validate_future_epoch() {
        let mut validator = OpValidator::new();
        let space_id = SpaceId(Uuid::new_v4());
        
        // Set local epoch to 5
        validator.update_epoch(space_id, EpochId(5));
        
        // Create op with epoch 10
        let op = create_test_op(
            UserId([1u8; 32]),
            space_id,
            EpochId(10),
            vec![],
        );

        let known_ops = HashMap::new();
        match validator.validate(&op, &known_ops) {
            ValidationResult::Buffered(_) => {} // Expected
            _ => panic!("Expected buffered for future epoch"),
        }
    }

    #[test]
    fn test_validate_duplicate_op() {
        use crate::crypto::signing::Keypair;
        
        let mut validator = OpValidator::new();
        let op_id = OpId(Uuid::new_v4());
        validator.seen_ops.insert(op_id);

        // Create a keypair first
        let keypair = Keypair::generate();
        
        let mut op = create_test_op(
            UserId([1u8; 32]),
            SpaceId(Uuid::new_v4()),
            EpochId(0),
            vec![],
        );
        
        // Change op_id and author to match our test setup
        op.op_id = op_id;
        op.author = keypair.user_id();
        
        // Re-sign with the new op_id
        let signing_bytes = op.signing_bytes();
        op.signature = Signature(keypair.sign(&signing_bytes).0);

        let known_ops = HashMap::new();
        match validator.validate(&op, &known_ops) {
            ValidationResult::Reject(RejectionReason::Duplicate) => {}
            _ => panic!("Expected duplicate rejection"),
        }
    }
}
