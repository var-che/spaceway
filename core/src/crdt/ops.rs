//! CRDT operation types and structures
//!
//! This module defines the core operation envelope and payload types for all CRDT operations.

use crate::types::*;
use crate::crdt::Hlc;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// CRDT operation envelope
///
/// All operations in the system are wrapped in this envelope which provides:
/// - Causal ordering via prev_ops dependencies
/// - Cryptographic attribution via author and signature
/// - Epoch-based membership validation
/// - HLC-based logical timestamps
#[derive(Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Debug)]
pub struct CrdtOp {
    /// Unique operation identifier
    #[n(0)]
    pub op_id: OpId,

    /// Space this operation belongs to
    #[n(1)]
    pub space_id: SpaceId,

    /// Optional channel (if operation is channel-specific)
    #[n(2)]
    pub channel_id: Option<ChannelId>,

    /// Optional thread (if operation is thread-specific)
    #[n(3)]
    pub thread_id: Option<ThreadId>,

    /// Operation type and payload
    #[n(4)]
    pub op_type: OpType,

    /// Causal dependencies (previous operation IDs)
    #[n(5)]
    pub prev_ops: Vec<OpId>,

    /// Author of this operation
    #[n(6)]
    pub author: UserId,

    /// MLS epoch when this operation was created
    #[n(7)]
    pub epoch: EpochId,

    /// Hybrid logical clock timestamp
    #[n(8)]
    pub hlc: Hlc,

    /// Unix timestamp in milliseconds (for human display only)
    #[n(9)]
    pub timestamp: u64,

    /// Ed25519 signature over canonical CBOR encoding of operation content
    #[n(10)]
    pub signature: Signature,
}

impl CrdtOp {
    /// Get the canonical bytes for signing
    ///
    /// This serializes all fields except the signature itself
    pub fn signing_bytes(&self) -> Vec<u8> {
        // Create a temporary struct without signature for encoding
        #[derive(Encode)]
        struct SigningData<'a> {
            #[n(0)] op_id: &'a OpId,
            #[n(1)] space_id: &'a SpaceId,
            #[n(2)] channel_id: &'a Option<ChannelId>,
            #[n(3)] thread_id: &'a Option<ThreadId>,
            #[n(4)] op_type: &'a OpType,
            #[n(5)] prev_ops: &'a Vec<OpId>,
            #[n(6)] author: &'a UserId,
            #[n(7)] epoch: &'a EpochId,
            #[n(8)] hlc: &'a Hlc,
            #[n(9)] timestamp: u64,
        }
        
        let data = SigningData {
            op_id: &self.op_id,
            space_id: &self.space_id,
            channel_id: &self.channel_id,
            thread_id: &self.thread_id,
            op_type: &self.op_type,
            prev_ops: &self.prev_ops,
            author: &self.author,
            epoch: &self.epoch,
            hlc: &self.hlc,
            timestamp: self.timestamp,
        };
        
        minicbor::to_vec(&data).expect("CBOR encoding should not fail")
    }

    /// Check if this operation causally depends on another
    pub fn depends_on(&self, other: &OpId) -> bool {
        self.prev_ops.contains(other)
    }

    /// Compute transitive causal dependencies
    pub fn transitive_deps<'a>(&'a self, ops: &'a [CrdtOp]) -> HashSet<OpId> {
        let mut deps = HashSet::new();
        let mut to_visit: Vec<&OpId> = self.prev_ops.iter().collect();

        while let Some(dep_id) = to_visit.pop() {
            if deps.insert(*dep_id) {
                // Find the op with this ID and add its dependencies
                if let Some(dep_op) = ops.iter().find(|op| &op.op_id == dep_id) {
                    to_visit.extend(&dep_op.prev_ops);
                }
            }
        }

        deps
    }
}

/// Operation type discriminant and payload
#[derive(Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Debug)]
pub enum OpType {
    /// Create a new space
    #[n(0)]
    CreateSpace(#[n(0)] OpPayload),

    /// Update space visibility
    #[n(1)]
    UpdateSpaceVisibility(#[n(0)] OpPayload),

    /// Create a new channel
    #[n(2)]
    CreateChannel(#[n(0)] OpPayload),

    /// Update a channel
    #[n(3)]
    UpdateChannel(#[n(0)] OpPayload),

    /// Archive a channel
    #[n(4)]
    ArchiveChannel,

    /// Create a new thread
    #[n(5)]
    CreateThread(#[n(0)] OpPayload),

    /// Post a message
    #[n(6)]
    PostMessage(#[n(0)] OpPayload),

    /// Edit a message
    #[n(7)]
    EditMessage(#[n(0)] OpPayload),

    /// Delete a message
    #[n(8)]
    DeleteMessage(#[n(0)] OpPayload),

    /// Add a member to the space
    #[n(9)]
    AddMember(#[n(0)] OpPayload),

    /// Remove a member from the space
    #[n(10)]
    RemoveMember(#[n(0)] OpPayload),

    /// Assign a role to a user
    #[n(11)]
    AssignRole(#[n(0)] OpPayload),

    /// Remove a role from a user
    #[n(12)]
    RemoveRole(#[n(0)] OpPayload),

    /// Mute a user
    #[n(13)]
    MuteUser(#[n(0)] OpPayload),

    /// Ban a user
    #[n(14)]
    BanUser(#[n(0)] OpPayload),

    /// Create an invite
    #[n(15)]
    CreateInvite(#[n(0)] OpPayload),

    /// Revoke an invite
    #[n(16)]
    RevokeInvite(#[n(0)] OpPayload),

    /// Use an invite (join via invite)
    #[n(17)]
    UseInvite(#[n(0)] OpPayload),
}

/// Operation payload (type-specific data)
#[derive(Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize, Debug)]
pub enum OpPayload {
    /// Create space payload
    #[n(0)]
    CreateSpace {
        #[n(0)]
        name: String,
        #[n(1)]
        description: Option<String>,
    },

    /// Update space visibility payload
    #[n(1)]
    UpdateSpaceVisibility {
        #[n(0)]
        visibility: SpaceVisibility,
    },

    /// Create channel payload
    #[n(2)]
    CreateChannel {
        #[n(0)]
        name: String,
        #[n(1)]
        description: Option<String>,
    },

    /// Update channel payload
    #[n(3)]
    UpdateChannel {
        #[n(0)]
        name: Option<String>,
        #[n(1)]
        description: Option<String>,
    },

    /// Create thread payload
    #[n(4)]
    CreateThread {
        #[n(0)]
        title: Option<String>,
        #[n(1)]
        first_message: String,
        #[n(2)]
        first_message_id: MessageId,
    },

    /// Post message payload
    #[n(5)]
    PostMessage {
        #[n(0)]
        message_id: MessageId,
        #[n(1)]
        content: String,
    },

    /// Edit message payload
    #[n(6)]
    EditMessage {
        #[n(0)]
        message_id: MessageId,
        #[n(1)]
        new_content: String,
    },

    /// Delete message payload
    #[n(7)]
    DeleteMessage {
        #[n(0)]
        message_id: MessageId,
        #[n(1)]
        reason: Option<String>,
    },

    /// Add member payload
    #[n(8)]
    AddMember {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        role: Role,
    },

    /// Remove member payload
    #[n(9)]
    RemoveMember {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        reason: Option<String>,
    },

    /// Assign role payload
    #[n(10)]
    AssignRole {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        role: Role,
        #[n(2)]
        channel_id: Option<ChannelId>,
    },

    /// Remove role payload
    #[n(11)]
    RemoveRole {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        role: Role,
        #[n(2)]
        channel_id: Option<ChannelId>,
    },

    /// Mute user payload
    #[n(12)]
    MuteUser {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        duration_secs: Option<u64>,
    },

    /// Ban user payload
    #[n(13)]
    BanUser {
        #[n(0)]
        user_id: UserId,
        #[n(1)]
        reason: Option<String>,
    },

    /// Create invite payload
    #[n(14)]
    CreateInvite {
        #[n(0)]
        invite: Invite,
    },

    /// Revoke invite payload
    #[n(15)]
    RevokeInvite {
        #[n(0)]
        invite_id: InviteId,
    },

    /// Use invite payload
    #[n(16)]
    UseInvite {
        #[n(0)]
        invite_id: InviteId,
        #[n(1)]
        code: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_crdt_op_signing_bytes() {
        let op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id: SpaceId(Uuid::new_v4()),
            channel_id: None,
            thread_id: None,
            op_type: OpType::CreateSpace(OpPayload::CreateSpace {
                name: "Test Space".to_string(),
                description: None,
            }),
            prev_ops: vec![],
            author: UserId([0u8; 32]),
            epoch: EpochId(0),
            hlc: Hlc { wall_time: 1000, logical: 0 },
            timestamp: 1000,
            signature: Signature([0u8; 64]),
        };

        let bytes = op.signing_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_crdt_op_serialization() {
        let op = CrdtOp {
            op_id: OpId(Uuid::new_v4()),
            space_id: SpaceId(Uuid::new_v4()),
            channel_id: None,
            thread_id: None,
            op_type: OpType::PostMessage(OpPayload::PostMessage {
                message_id: MessageId(Uuid::new_v4()),
                content: "Hello world".to_string(),
            }),
            prev_ops: vec![],
            author: UserId([1u8; 32]),
            epoch: EpochId(5),
            hlc: Hlc { wall_time: 2000, logical: 3 },
            timestamp: 2000,
            signature: Signature([0u8; 64]),
        };

        let mut buf = Vec::new();
        minicbor::encode(&op, &mut buf).expect("encode failed");
        
        let decoded: CrdtOp = minicbor::decode(&buf).expect("decode failed");
        assert_eq!(op, decoded);
    }
}
