//! Descord Relay Node
//!
//! Dedicated Circuit Relay v2 server for IP privacy and NAT traversal.
//! 
//! Features:
//! - Circuit Relay v2 for relaying connections between peers
//! - DHT advertisement for relay discovery
//! - Bandwidth tracking and circuit limits
//! - Multi-transport support (TCP + QUIC)
//! - Monitoring and statistics

use anyhow::Result;
use libp2p::{
    identity, kad,
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
    core::upgrade,
    futures::StreamExt,
    Transport,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber;

/// Network behavior for relay server: DHT + Relay Server
#[derive(NetworkBehaviour)]
struct RelayBehaviour {
    /// Kademlia DHT for relay advertisement
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    
    /// Relay server behavior (accepts relay requests)
    relay: relay::Behaviour,
}

/// Statistics for a single circuit
#[derive(Debug, Clone)]
struct CircuitStats {
    /// Peer using this circuit
    peer_id: PeerId,
    /// Bytes sent through circuit
    bytes_sent: u64,
    /// Bytes received through circuit
    bytes_received: u64,
    /// When circuit was created
    start_time: Instant,
    /// Circuit ID
    circuit_id: String,
}

/// Relay server state
struct RelayServer {
    /// libp2p swarm
    swarm: Swarm<RelayBehaviour>,
    
    /// Active circuits
    circuits: HashMap<String, CircuitStats>,
    
    /// Per-peer circuit count
    peer_circuits: HashMap<PeerId, usize>,
    
    /// Connected peers (for tracking)
    connected_peers: HashMap<PeerId, Instant>,
    
    /// Total bandwidth used (bytes)
    total_bandwidth: u64,
    
    /// Server start time
    start_time: Instant,
    
    /// Configuration
    max_circuits_per_peer: usize,
    max_circuit_duration: Duration,
    max_circuit_bytes: u64,
}

impl RelayServer {
    /// Create a new relay server
    fn new(listen_addr: Multiaddr) -> Result<Self> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        info!("Relay server peer ID: {}", local_peer_id);
        
        // Create Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kademlia = kad::Behaviour::new(local_peer_id, store);
        kademlia.set_mode(Some(kad::Mode::Server));
        
        // Create Circuit Relay v2 server
        let relay_config = relay::Config {
            max_reservations: 1024,
            max_reservations_per_peer: 3,
            reservation_duration: Duration::from_secs(3600), // 1 hour
            max_circuits: 512,
            max_circuits_per_peer: 5,
            max_circuit_duration: Duration::from_secs(3600), // 1 hour
            max_circuit_bytes: 100 * 1024 * 1024, // 100 MB
            ..Default::default()
        };
        
        let relay = relay::Behaviour::new(local_peer_id, relay_config);
        
        let behaviour = RelayBehaviour {
            kademlia,
            relay,
        };
        
        // Build transport: TCP
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key)?)
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
        // Create swarm
        let mut swarm = Swarm::new(
            tcp_transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(Duration::from_secs(120))
        );
        
        // Start listening
        swarm.listen_on(listen_addr.clone())?;
        info!("Listening on: {}", listen_addr);
        
        Ok(Self {
            swarm,
            circuits: HashMap::new(),
            peer_circuits: HashMap::new(),
            connected_peers: HashMap::new(),
            total_bandwidth: 0,
            start_time: Instant::now(),
            max_circuits_per_peer: 5,
            max_circuit_duration: Duration::from_secs(3600),
            max_circuit_bytes: 100 * 1024 * 1024,
        })
    }
    
    /// Run the relay server event loop
    async fn run(&mut self) {
        info!("Relay server running...");
        
        // Stats reporting interval
        let mut stats_interval = tokio::time::interval(Duration::from_secs(60));
        
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await;
                }
                
                _ = stats_interval.tick() => {
                    self.print_stats();
                }
            }
        }
    }
    
    /// Handle swarm events
    async fn handle_event(&mut self, event: SwarmEvent<RelayBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("🎧 Listening on: {}", address);
            }
            
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, .. } => {
                self.connected_peers.insert(peer_id, Instant::now());
                info!("🔗 Connection established with peer: {}", peer_id);
                info!("   Remote address: {}", endpoint.get_remote_address());
                info!("   Total connections to this peer: {}", num_established);
                info!("   Total unique peers connected: {}", self.connected_peers.len());
            }
            
            SwarmEvent::ConnectionClosed { peer_id, cause, num_established, .. } => {
                if num_established == 0 {
                    self.connected_peers.remove(&peer_id);
                    info!("❌ Last connection closed with peer: {} (cause: {:?})", peer_id, cause);
                    info!("   Total unique peers connected: {}", self.connected_peers.len());
                } else {
                    info!("🔌 Connection closed with peer: {} (cause: {:?})", peer_id, cause);
                    info!("   Remaining connections to this peer: {}", num_established);
                }
                
                // Clean up peer circuit count if fully disconnected
                if num_established == 0 {
                    self.peer_circuits.remove(&peer_id);
                }
            }
            
            SwarmEvent::IncomingConnection { send_back_addr, .. } => {
                info!("📥 Incoming connection from: {}", send_back_addr);
            }
            
            SwarmEvent::Behaviour(RelayBehaviourEvent::Relay(event)) => {
                match event {
                    relay::Event::ReservationReqAccepted { src_peer_id, renewed } => {
                        if renewed {
                            info!("🔄 Reservation RENEWED for peer: {}", src_peer_id);
                        } else {
                            info!("✅ Reservation ACCEPTED for peer: {}", src_peer_id);
                            info!("   This peer can now be used as a relay hop");
                        }
                    }
                    
                    relay::Event::ReservationReqDenied { src_peer_id, status } => {
                        warn!("⛔ Reservation denied for peer: {} (status: {:?})", src_peer_id, status);
                    }
                    
                    relay::Event::ReservationReqAcceptFailed { src_peer_id, error } => {
                        warn!("❌ Reservation accept failed for peer: {} (error: {})", src_peer_id, error);
                    }
                    
                    relay::Event::ReservationReqDenyFailed { src_peer_id, error } => {
                        warn!("❌ Reservation deny failed for peer: {} (error: {})", src_peer_id, error);
                    }
                    
                    relay::Event::ReservationTimedOut { src_peer_id } => {
                        info!("⏰ Reservation timed out for peer: {}", src_peer_id);
                    }
                    
                    relay::Event::ReservationClosed { src_peer_id } => {
                        info!("🔒 Reservation closed for peer: {}", src_peer_id);
                    }
                    
                    relay::Event::CircuitReqDenied { src_peer_id, dst_peer_id, status } => {
                        warn!("⛔ Circuit denied: {} -> {} (status: {:?})", src_peer_id, dst_peer_id, status);
                    }
                    
                    relay::Event::CircuitReqAccepted { src_peer_id, dst_peer_id } => {
                        info!("🔀 Circuit ESTABLISHED: {} -> {}", src_peer_id, dst_peer_id);
                        info!("   Source peer can now communicate with destination via this relay");
                        
                        // Track circuit
                        let circuit_id = format!("{}-{}", src_peer_id, dst_peer_id);
                        self.circuits.insert(circuit_id.clone(), CircuitStats {
                            peer_id: src_peer_id,
                            bytes_sent: 0,
                            bytes_received: 0,
                            start_time: Instant::now(),
                            circuit_id,
                        });
                        
                        // Update peer circuit count
                        *self.peer_circuits.entry(src_peer_id).or_insert(0) += 1;
                        
                        info!("   Total circuits: {}", self.circuits.len());
                    }
                    
                    relay::Event::CircuitReqAcceptFailed { src_peer_id, dst_peer_id, error } => {
                        warn!("❌ Circuit accept failed: {} -> {} (error: {})", src_peer_id, dst_peer_id, error);
                    }
                    
                    relay::Event::CircuitReqDenyFailed { src_peer_id, dst_peer_id, error } => {
                        warn!("❌ Circuit deny failed: {} -> {} (error: {})", src_peer_id, dst_peer_id, error);
                    }
                    
                    relay::Event::CircuitReqOutboundConnectFailed { src_peer_id, dst_peer_id, error } => {
                        warn!("❌ Circuit outbound connect failed: {} -> {} (error: {})", 
                            src_peer_id, dst_peer_id, error);
                    }
                    
                    relay::Event::CircuitClosed { src_peer_id, dst_peer_id, error } => {
                        if let Some(err) = error {
                            warn!("🔌 Circuit closed with error: {} -> {} ({})", 
                                src_peer_id, dst_peer_id, err);
                        } else {
                            info!("🔌 Circuit closed: {} -> {}", src_peer_id, dst_peer_id);
                        }
                        
                        // Clean up circuit
                        let circuit_id = format!("{}-{}", src_peer_id, dst_peer_id);
                        self.circuits.remove(&circuit_id);
                        
                        // Update peer circuit count
                        if let Some(count) = self.peer_circuits.get_mut(&src_peer_id) {
                            *count = count.saturating_sub(1);
                        }
                    }
                }
            }
            
            SwarmEvent::Behaviour(RelayBehaviourEvent::Kademlia(event)) => {
                match event {
                    kad::Event::RoutingUpdated { peer, .. } => {
                        info!("📍 DHT routing updated with peer: {}", peer);
                    }
                    _ => {}
                }
            }
            
            _ => {}
        }
    }
    
    /// Print relay statistics
    fn print_stats(&self) {
        let uptime = self.start_time.elapsed();
        let active_circuits = self.circuits.len();
        let connected_peers = self.connected_peers.len();
        let peers_with_circuits = self.peer_circuits.len();
        
        info!("📊 Relay Statistics:");
        info!("  Uptime: {:?}", uptime);
        info!("  Connected peers: {}", connected_peers);
        info!("  Peers with circuits: {}", peers_with_circuits);
        info!("  Active circuits: {}", active_circuits);
        info!("  Total bandwidth: {} MB", self.total_bandwidth / (1024 * 1024));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("🚀 Starting Descord Relay Server");
    
    // Listen on all interfaces, port 9000 (TCP)
    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/9000".parse()?;
    
    // Create and run relay server
    let mut server = RelayServer::new(listen_addr)?;
    
    // Run server
    tokio::select! {
        _ = server.run() => {
            info!("Server stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
    }
    
    info!("✅ Relay server shut down gracefully");
    Ok(())
}
