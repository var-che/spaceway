//! Dashboard API types and snapshot helpers
//!
//! Provides serializable snapshots of Client state for visualization dashboards.
//! These types are safe to expose externally - they contain no private keys or
//! sensitive cryptographic material.

use crate::types::*;
use crate::crdt::{CrdtOp, OpType};
use crate::forum::{Space, Channel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete dashboard state snapshot (sent to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardState {
    /// All client snapshots
    pub clients: Vec<ClientSnapshot>,
    /// Network topology graph
    pub network_graph: NetworkGraph,
    /// CRDT operation timeline
    pub crdt_timeline: Vec<CrdtOperationSnapshot>,
}

/// Snapshot of a single client's state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClientSnapshot {
    /// Client display name
    pub name: String,
    /// User ID (hex-encoded)
    pub user_id: String,
    /// Spaces this client is a member of
    pub spaces: Vec<SpaceSnapshot>,
    /// DHT storage entries (metadata only)
    pub dht_storage: Vec<DhtEntry>,
    /// MLS group information
    pub mls_groups: Vec<MlsGroupInfo>,
    /// Connected peer IDs
    pub connected_peers: Vec<String>,
}

/// Space information snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SpaceSnapshot {
    /// Space ID (hex-encoded)
    pub id: String,
    /// Space display name
    pub name: String,
    /// Owner user ID (hex-encoded)
    pub owner: String,
    /// Space members
    pub members: Vec<MemberInfo>,
    /// Number of members (for frontend convenience)
    pub member_count: usize,
    /// Channels in this space
    pub channels: Vec<ChannelSnapshot>,
    /// Number of custom roles
    pub role_count: usize,
    /// Space creation timestamp
    pub created_at: u64,
    /// Current epoch
    pub epoch: u64,
}

/// Member information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MemberInfo {
    /// User ID (hex-encoded)
    pub user_id: String,
    /// Role name
    pub role: String,
    /// Permission strings
    pub permissions: Vec<String>,
}

/// Channel information snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChannelSnapshot {
    /// Channel ID (hex-encoded)
    pub id: String,
    /// Channel display name
    pub name: String,
    /// Message count (0 for now, implement later)
    pub message_count: usize,
    /// Channel description
    pub description: Option<String>,
    /// Creator user ID
    pub creator: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Whether archived
    pub archived: bool,
    /// Threads in this channel
    pub threads: Vec<ThreadSnapshot>,
}

/// Thread information snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThreadSnapshot {
    /// Thread ID (hex-encoded)
    pub id: String,
    /// Thread title (optional)
    pub title: Option<String>,
    /// Creator user ID
    pub creator: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Message count
    pub message_count: usize,
    /// Messages in this thread
    pub messages: Vec<MessageSnapshot>,
}

/// Message information snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageSnapshot {
    /// Message ID (hex-encoded)
    pub id: String,
    /// Author user ID
    pub author: String,
    /// Message content
    pub content: String,
    /// Creation timestamp
    pub created_at: u64,
}

/// DHT storage entry (metadata only, no actual data)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DhtEntry {
    /// DHT key (hex-encoded, truncated for display)
    pub key: String,
    /// Type of value stored
    pub value_type: String,
    /// Size in bytes
    pub size_bytes: usize,
}

/// MLS group information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MlsGroupInfo {
    /// Space ID this group belongs to (hex-encoded)
    pub space_id: String,
    /// Current epoch number
    pub epoch: u64,
    /// Number of members in the group
    pub member_count: usize,
}

/// Network topology graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkGraph {
    /// Network nodes (peers)
    pub nodes: Vec<NetworkNode>,
    /// Network edges (connections)
    pub edges: Vec<NetworkEdge>,
}

/// Network node (peer)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkNode {
    /// Node ID (peer_id or user_id)
    pub id: String,
    /// Display label
    pub label: String,
    /// Node type: "client" or "dht_node"
    pub peer_type: String,
}

/// Network edge (connection)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Edge type: "direct", "dht", "gossipsub"
    pub edge_type: String,
}

/// CRDT operation snapshot for timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CrdtOperationSnapshot {
    /// Unix timestamp
    pub timestamp: u64,
    /// Operation ID (hex-encoded)
    pub op_id: String,
    /// Operation type string
    pub op_type: String,
    /// Author user ID (hex-encoded)
    pub author: String,
    /// Space ID (hex-encoded)
    pub space_id: String,
    /// Channel ID (hex-encoded, optional)
    pub channel_id: Option<String>,
}

// ============================================================================
// Conversion helpers (From traits)
// ============================================================================

impl SpaceSnapshot {
    /// Create a snapshot from a Space
    pub fn from_space(space: &Space) -> Self {
        #[allow(deprecated)]
        let members: Vec<MemberInfo> = space.members.iter().map(|(user_id, role)| {
            // Get permissions for this role
            let permissions = match role {
                Role::Admin => vec![
                    "CREATE_CHANNELS".to_string(),
                    "INVITE_MEMBERS".to_string(),
                    "KICK_MEMBERS".to_string(),
                    "MANAGE_ROLES".to_string(),
                    "SEND_MESSAGES".to_string(),
                ],
                Role::Moderator => vec![
                    "CREATE_CHANNELS".to_string(),
                    "KICK_MEMBERS".to_string(),
                    "SEND_MESSAGES".to_string(),
                ],
                Role::Member => vec![
                    "INVITE_MEMBERS".to_string(),
                    "SEND_MESSAGES".to_string(),
                ],
            };
            
            MemberInfo {
                user_id: hex::encode(&user_id.0),
                role: format!("{:?}", role),
                permissions,
            }
        }).collect();
        
        Self {
            id: hex::encode(&space.id.0),
            name: space.name.clone(),
            owner: hex::encode(&space.owner.0),
            member_count: members.len(),
            members,
            channels: Vec::new(), // Will be populated separately
            role_count: space.roles.len(),
            created_at: space.created_at,
            epoch: space.epoch.0,
        }
    }
}

impl ChannelSnapshot {
    /// Create a snapshot from a Channel
    pub fn from_channel(channel: &Channel) -> Self {
        Self {
            id: hex::encode(&channel.id.0),
            name: channel.name.clone(),
            message_count: 0, // TODO: implement message counting
            description: channel.description.clone(),
            creator: hex::encode(&channel.creator.0),
            created_at: channel.created_at,
            archived: channel.archived,
            threads: Vec::new(), // Will be filled in by the caller
        }
    }
}

impl ThreadSnapshot {
    /// Create a snapshot from a Thread with messages
    pub fn from_thread(thread: &crate::forum::Thread, messages: Vec<MessageSnapshot>) -> Self {
        Self {
            id: hex::encode(&thread.id.0),
            title: thread.title.clone(),
            creator: hex::encode(&thread.creator.0),
            created_at: thread.created_at,
            message_count: thread.message_count as usize,
            messages,
        }
    }
}

impl MessageSnapshot {
    /// Create a snapshot from a Message
    pub fn from_message(message: &crate::forum::Message) -> Self {
        Self {
            id: hex::encode(&message.id.0),
            author: hex::encode(&message.author.0),
            content: message.content.clone(),
            created_at: message.created_at,
        }
    }
}

impl CrdtOperationSnapshot {
    /// Create a snapshot from a CRDT operation
    pub fn from_crdt_op(op: &CrdtOp) -> Self {
        let op_type_str = match &op.op_type {
            OpType::CreateSpace(_) => "CreateSpace",
            OpType::CreateChannel(_) => "CreateChannel",
            OpType::CreateThread(_) => "CreateThread",
            OpType::PostMessage(_) => "PostMessage",
            OpType::UpdateChannel(_) => "UpdateChannel",
            OpType::ArchiveChannel => "ArchiveChannel",
            OpType::AddMember(_) => "AddMember",
            OpType::RemoveMember(_) => "RemoveMember",
            OpType::AssignRole(_) => "AssignRole",
            OpType::EditMessage(_) => "EditMessage",
            OpType::DeleteMessage(_) => "DeleteMessage",
            OpType::UpdateSpaceVisibility(_) => "UpdateSpaceVisibility",
            OpType::RemoveRole(_) => "RemoveRole",
            _ => "Other", // For other operation types
        };

        Self {
            timestamp: op.timestamp,
            op_id: hex::encode(op.op_id.0.as_bytes()),
            op_type: op_type_str.to_string(),
            author: hex::encode(&op.author.0),
            space_id: hex::encode(&op.space_id.0),
            channel_id: op.channel_id.map(|id| hex::encode(&id.0)),
        }
    }
}

impl NetworkGraph {
    /// Create an empty network graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add a client node
    pub fn add_client_node(&mut self, user_id: &str, name: &str) {
        self.nodes.push(NetworkNode {
            id: user_id.to_string(),
            label: name.to_string(),
            peer_type: "client".to_string(),
        });
    }

    /// Add a GossipSub edge between two clients (they share a space)
    pub fn add_gossipsub_edge(&mut self, from: &str, to: &str) {
        self.edges.push(NetworkEdge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: "gossipsub".to_string(),
        });
    }

    /// Add a DHT edge (direct peer connection)
    pub fn add_dht_edge(&mut self, from: &str, to: &str) {
        self.edges.push(NetworkEdge {
            from: from.to_string(),
            to: to.to_string(),
            edge_type: "dht".to_string(),
        });
    }
}

impl Default for NetworkGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = ClientSnapshot {
            name: "Alice".to_string(),
            user_id: "deadbeef".to_string(),
            spaces: vec![],
            dht_storage: vec![],
            mls_groups: vec![],
            connected_peers: vec![],
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let deserialized: ClientSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot.name, deserialized.name);
    }

    #[test]
    fn test_network_graph_builder() {
        let mut graph = NetworkGraph::new();
        graph.add_client_node("alice", "Alice");
        graph.add_client_node("bob", "Bob");
        graph.add_gossipsub_edge("alice", "bob");

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].edge_type, "gossipsub");
    }
}
