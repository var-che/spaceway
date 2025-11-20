//! Networking module
//!
//! Provides libp2p-based networking primitives

pub mod node;
pub mod relay;
pub mod gossip_metrics;

pub use node::{NetworkNode, NetworkEvent, create_relay_server};
pub use gossip_metrics::GossipMetrics;
