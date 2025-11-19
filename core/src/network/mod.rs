//! Networking module
//!
//! Provides libp2p-based networking primitives

pub mod node;
pub mod relay;

pub use node::{NetworkNode, NetworkEvent, create_relay_server};
