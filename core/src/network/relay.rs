//! Relay node networking
//!
//! Implements libp2p Circuit Relay v2 for IP privacy protection.
//! 
//! Privacy Model:
//! - Public spaces: Direct P2P (user consents to IP exposure)
//! - Private/Hidden spaces: Relay-only (IP hidden from peers)
//!
//! Architecture:
//! User A <--encrypted--> Relay Node <--encrypted--> User B
//!          (A's IP hidden from B)     (B's IP hidden from A)

use libp2p::{
    relay,
    identity,
    Multiaddr,
    PeerId,
};
use std::time::Duration;

/// Relay configuration
#[derive(Clone, Debug)]
pub struct RelayConfig {
    /// Maximum reservations per peer
    pub max_reservations_per_peer: usize,
    /// Maximum circuits per peer
    pub max_circuits_per_peer: usize,
    /// Circuit duration limit
    pub max_circuit_duration: Duration,
    /// Maximum circuit bytes
    pub max_circuit_bytes: u64,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            max_reservations_per_peer: 3,
            max_circuits_per_peer: 5,
            max_circuit_duration: Duration::from_secs(3600), // 1 hour
            max_circuit_bytes: 100 * 1024 * 1024, // 100 MB
        }
    }
}

/// Bootstrap relay addresses (hardcoded for now, can be configurable later)
/// In production, these would be community-run or paid relay nodes
pub fn default_relay_addresses() -> Vec<Multiaddr> {
    vec![
        // Localhost for testing
        "/ip4/127.0.0.1/tcp/4001".parse().unwrap(),
        // TODO: Add production relay addresses
        // "/ip4/relay1.descord.network/tcp/4001".parse().unwrap(),
        // "/ip4/relay2.descord.network/tcp/4001".parse().unwrap(),
    ]
}

/// Check if relay transport should be used for a given visibility level
pub fn should_use_relay(visibility: crate::types::SpaceVisibility) -> bool {
    use crate::types::SpaceVisibility;
    matches!(visibility, SpaceVisibility::Private | SpaceVisibility::Hidden)
}

/// Get relay multiaddr from peer ID and relay address
pub fn relay_multiaddr(relay_addr: &Multiaddr, relay_peer_id: &PeerId, target_peer_id: &PeerId) -> Multiaddr {
    // Format: /ip4/127.0.0.1/tcp/4001/p2p/{relay_peer_id}/p2p-circuit/p2p/{target_peer_id}
    let mut addr = relay_addr.clone();
    addr.push(libp2p::multiaddr::Protocol::P2p((*relay_peer_id).into()));
    addr.push(libp2p::multiaddr::Protocol::P2pCircuit);
    addr.push(libp2p::multiaddr::Protocol::P2p((*target_peer_id).into()));
    addr
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_use_relay() {
        use crate::types::SpaceVisibility;
        
        assert!(!should_use_relay(SpaceVisibility::Public));
        assert!(should_use_relay(SpaceVisibility::Private));
        assert!(should_use_relay(SpaceVisibility::Hidden));
    }

    #[test]
    fn test_relay_config_defaults() {
        let config = RelayConfig::default();
        assert_eq!(config.max_reservations_per_peer, 3);
        assert_eq!(config.max_circuits_per_peer, 5);
    }

    #[test]
    fn test_default_relay_addresses() {
        let addrs = default_relay_addresses();
        assert!(!addrs.is_empty());
    }
}

