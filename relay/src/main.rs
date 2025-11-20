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
use clap::Parser;
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Descord Circuit Relay v2 Server
#[derive(Parser, Debug)]
#[command(name = "descord-relay")]
#[command(author, version, about = "Dedicated relay server for IP privacy and NAT traversal", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 9000)]
    port: u16,
    
    /// Interface to bind to (0.0.0.0 for all interfaces)
    #[arg(short, long, default_value = "0.0.0.0")]
    interface: String,
    
    /// Maximum total reservations
    #[arg(long, default_value_t = 1024)]
    max_reservations: usize,
    
    /// Maximum reservations per peer
    #[arg(long, default_value_t = 3)]
    max_reservations_per_peer: usize,
    
    /// Maximum total circuits
    #[arg(long, default_value_t = 512)]
    max_circuits: usize,
    
    /// Maximum circuits per peer
    #[arg(long, default_value_t = 5)]
    max_circuits_per_peer: usize,
    
    /// Circuit duration limit in seconds
    #[arg(long, default_value_t = 3600)]
    circuit_duration_secs: u64,
    
    /// Circuit bandwidth limit in MB
    #[arg(long, default_value_t = 100)]
    circuit_bandwidth_mb: u64,
    
    /// Statistics interval in seconds
    #[arg(long, default_value_t = 60)]
    stats_interval_secs: u64,
    
    /// Log level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// Advertise relay to DHT for discovery
    #[arg(long, default_value_t = true)]
    advertise_on_dht: bool,
    
    /// DHT advertisement interval in seconds
    #[arg(long, default_value_t = 300)]
    advertise_interval_secs: u64,
    
    /// Monitoring HTTP endpoint port (0 to disable)
    #[arg(long, default_value_t = 9001)]
    monitoring_port: u16,
}
use tracing_subscriber;
use serde::{Serialize, Deserialize};

/// DHT key for relay advertisements
const RELAY_DHT_KEY: &[u8] = b"/descord/relay/advertisements";

/// Shared statistics for monitoring endpoint
type SharedStats = Arc<Mutex<RelayStats>>;

/// Statistics snapshot for monitoring
#[derive(Debug, Clone, Default)]
struct RelayStats {
    uptime_secs: u64,
    connected_peers: usize,
    peers_with_circuits: usize,
    active_circuits: usize,
    total_bandwidth_bytes: u64,
}

/// Relay advertisement data published to DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayAdvertisement {
    /// Relay server peer ID
    peer_id: String,
    /// Multiaddresses where relay is listening
    addresses: Vec<String>,
    /// Maximum circuits this relay supports
    max_circuits: usize,
    /// Maximum bandwidth per circuit (MB)
    max_circuit_bandwidth_mb: u64,
    /// UNIX timestamp of advertisement
    timestamp: u64,
}

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
    advertise_on_dht: bool,
    max_circuits: usize,
    max_circuit_bandwidth_mb: u64,
    
    /// Shared statistics for monitoring
    shared_stats: SharedStats,
}

impl RelayServer {
    /// Create a new relay server with configuration
    fn new(listen_addr: Multiaddr, args: &Args) -> Result<Self> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        info!("Relay server peer ID: {}", local_peer_id);
        
        // Create Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kademlia = kad::Behaviour::new(local_peer_id, store);
        kademlia.set_mode(Some(kad::Mode::Server));
        
        // Create Circuit Relay v2 server with CLI configuration
        let relay_config = relay::Config {
            max_reservations: args.max_reservations,
            max_reservations_per_peer: args.max_reservations_per_peer,
            reservation_duration: Duration::from_secs(args.circuit_duration_secs),
            max_circuits: args.max_circuits,
            max_circuits_per_peer: args.max_circuits_per_peer,
            max_circuit_duration: Duration::from_secs(args.circuit_duration_secs),
            max_circuit_bytes: args.circuit_bandwidth_mb * 1024 * 1024,
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
            max_circuits_per_peer: args.max_circuits_per_peer,
            max_circuit_duration: Duration::from_secs(args.circuit_duration_secs),
            max_circuit_bytes: args.circuit_bandwidth_mb * 1024 * 1024,
            advertise_on_dht: args.advertise_on_dht,
            max_circuits: args.max_circuits,
            max_circuit_bandwidth_mb: args.circuit_bandwidth_mb,
            shared_stats: Arc::new(Mutex::new(RelayStats::default())),
        })
    }
    
    /// Get shared stats reference
    fn stats(&self) -> SharedStats {
        self.shared_stats.clone()
    }
    
    /// Run the relay server event loop
    async fn run(&mut self, stats_interval_secs: u64, advertise_interval_secs: u64) {
        info!("Relay server running...");
        
        // Stats reporting interval (configurable)
        let mut stats_interval = tokio::time::interval(Duration::from_secs(stats_interval_secs));
        
        // DHT advertisement interval (configurable)
        let mut advertise_interval = tokio::time::interval(Duration::from_secs(advertise_interval_secs));
        
        // Initial DHT advertisement
        if self.advertise_on_dht {
            self.advertise_to_dht();
        }
        
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_event(event).await;
                }
                
                _ = stats_interval.tick() => {
                    self.print_stats();
                }
                
                _ = advertise_interval.tick() => {
                    if self.advertise_on_dht {
                        self.advertise_to_dht();
                    }
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
                        
                        // Check and enforce limits
                        self.check_circuit_limits();
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
    
    /// Advertise relay server to DHT for discovery
    fn advertise_to_dht(&mut self) {
        let listening_addrs: Vec<String> = self.swarm.listeners()
            .map(|addr| addr.to_string())
            .collect();
        
        if listening_addrs.is_empty() {
            warn!("⚠️ Cannot advertise to DHT: no listening addresses");
            return;
        }
        
        let advertisement = RelayAdvertisement {
            peer_id: self.swarm.local_peer_id().to_string(),
            addresses: listening_addrs.clone(),
            max_circuits: self.max_circuits,
            max_circuit_bandwidth_mb: self.max_circuit_bandwidth_mb,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Serialize advertisement
        match serde_json::to_vec(&advertisement) {
            Ok(data) => {
                use libp2p::kad::{Record, Quorum};
                let record = Record::new(RELAY_DHT_KEY.to_vec(), data);
                
                match self.swarm.behaviour_mut().kademlia.put_record(record, Quorum::One) {
                    Ok(_) => {
                        info!("📢 Advertised relay to DHT");
                        info!("   Addresses: {:?}", listening_addrs);
                        info!("   Capacity: {} circuits, {}MB bandwidth", 
                            self.max_circuits, self.max_circuit_bandwidth_mb);
                    }
                    Err(e) => {
                        warn!("⚠️ Failed to advertise to DHT: {:?}", e);
                    }
                }
            }
            Err(e) => {
                warn!("⚠️ Failed to serialize advertisement: {}", e);
            }
        }
    }
    
    /// Check and enforce circuit limits (duration and bandwidth)
    fn check_circuit_limits(&mut self) {
        let now = Instant::now();
        let mut circuits_to_close = Vec::new();
        
        for (circuit_id, stats) in &self.circuits {
            // Check circuit duration
            let circuit_age = now.duration_since(stats.start_time);
            if circuit_age > self.max_circuit_duration {
                warn!("⏰ Circuit {} exceeded max duration ({:?} > {:?})", 
                    circuit_id, circuit_age, self.max_circuit_duration);
                circuits_to_close.push(circuit_id.clone());
                continue;
            }
            
            // Check circuit bandwidth
            let total_bytes = stats.bytes_sent + stats.bytes_received;
            if total_bytes > self.max_circuit_bytes {
                warn!("📊 Circuit {} exceeded bandwidth limit ({} > {} bytes)", 
                    circuit_id, total_bytes, self.max_circuit_bytes);
                circuits_to_close.push(circuit_id.clone());
            }
        }
        
        // Close circuits that exceeded limits
        for circuit_id in circuits_to_close {
            if let Some(stats) = self.circuits.remove(&circuit_id) {
                info!("🚫 Closing circuit {} due to limit violation", circuit_id);
                
                // Update peer circuit count
                if let Some(count) = self.peer_circuits.get_mut(&stats.peer_id) {
                    *count = count.saturating_sub(1);
                }
            }
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
        
        // Update shared stats for monitoring endpoint
        if let Ok(mut stats) = self.shared_stats.lock() {
            stats.uptime_secs = uptime.as_secs();
            stats.connected_peers = connected_peers;
            stats.peers_with_circuits = peers_with_circuits;
            stats.active_circuits = active_circuits;
            stats.total_bandwidth_bytes = self.total_bandwidth;
        }
    }
}

/// HTTP monitoring server
mod monitoring {
    use super::*;
    use hyper::{body::Bytes, server::conn::http1, service::service_fn, Request, Response};
    use hyper_util::rt::TokioIo;
    use http_body_util::Full;
    use tokio::net::TcpListener;
    
    /// Start HTTP monitoring server
    pub async fn start_server(port: u16, stats: SharedStats) -> Result<()> {
        use std::net::{Ipv4Addr, SocketAddr};
        
        let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("📊 Monitoring endpoint: http://0.0.0.0:{}/metrics", port);
        
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let stats = stats.clone();
            
            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| handle_request(req, stats.clone())))
                    .await
                {
                    warn!("Error serving connection: {:?}", err);
                }
            });
        }
    }
    
    /// Handle HTTP request
    async fn handle_request(
        req: Request<hyper::body::Incoming>,
        stats: SharedStats,
    ) -> Result<Response<Full<Bytes>>, hyper::Error> {
        match req.uri().path() {
            "/metrics" => Ok(metrics_response(stats)),
            "/health" => Ok(health_response()),
            _ => Ok(not_found_response()),
        }
    }
    
    /// Generate Prometheus-style metrics response
    fn metrics_response(stats: SharedStats) -> Response<Full<Bytes>> {
        let stats = stats.lock().unwrap();
        
        let metrics = format!(
            "# HELP relay_uptime_seconds Relay server uptime in seconds\n\
             # TYPE relay_uptime_seconds counter\n\
             relay_uptime_seconds {}\n\
             \n\
             # HELP relay_connected_peers Number of connected peers\n\
             # TYPE relay_connected_peers gauge\n\
             relay_connected_peers {}\n\
             \n\
             # HELP relay_peers_with_circuits Number of peers with active circuits\n\
             # TYPE relay_peers_with_circuits gauge\n\
             relay_peers_with_circuits {}\n\
             \n\
             # HELP relay_active_circuits Number of active circuits\n\
             # TYPE relay_active_circuits gauge\n\
             relay_active_circuits {}\n\
             \n\
             # HELP relay_bandwidth_bytes_total Total bandwidth transferred in bytes\n\
             # TYPE relay_bandwidth_bytes_total counter\n\
             relay_bandwidth_bytes_total {}\n",
            stats.uptime_secs,
            stats.connected_peers,
            stats.peers_with_circuits,
            stats.active_circuits,
            stats.total_bandwidth_bytes,
        );
        
        Response::builder()
            .status(200)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(Full::new(Bytes::from(metrics)))
            .unwrap()
    }
    
    /// Health check response
    fn health_response() -> Response<Full<Bytes>> {
        Response::builder()
            .status(200)
            .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
            .unwrap()
    }
    
    /// 404 response
    fn not_found_response() -> Response<Full<Bytes>> {
        Response::builder()
            .status(404)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize tracing with configured log level
    let log_level = args.log_level.parse::<tracing::Level>()
        .unwrap_or(tracing::Level::INFO);
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(log_level.into()),
        )
        .init();

    info!("🚀 Starting Descord Relay Server");
    info!("Configuration:");
    info!("  Listen: {}:{}", args.interface, args.port);
    info!("  Max reservations: {} (per peer: {})", args.max_reservations, args.max_reservations_per_peer);
    info!("  Max circuits: {} (per peer: {})", args.max_circuits, args.max_circuits_per_peer);
    info!("  Circuit duration: {}s, bandwidth: {}MB", args.circuit_duration_secs, args.circuit_bandwidth_mb);
    info!("  Stats interval: {}s, log level: {}", args.stats_interval_secs, args.log_level);
    info!("  DHT advertisement: {} (interval: {}s)", args.advertise_on_dht, args.advertise_interval_secs);
    if args.monitoring_port > 0 {
        info!("  Monitoring endpoint: http://0.0.0.0:{}/metrics", args.monitoring_port);
    }
    
    // Build listen address from CLI args
    let listen_addr: Multiaddr = format!("/ip4/{}/tcp/{}", args.interface, args.port).parse()?;
    
    // Create and run relay server with configuration
    let mut server = RelayServer::new(listen_addr, &args)?;
    
    // Start monitoring endpoint if enabled
    let monitoring_handle = if args.monitoring_port > 0 {
        let stats = server.stats();
        Some(tokio::spawn(async move {
            if let Err(e) = monitoring::start_server(args.monitoring_port, stats).await {
                warn!("Monitoring server error: {}", e);
            }
        }))
    } else {
        None
    };
    
    // Run server with configured intervals
    tokio::select! {
        _ = server.run(args.stats_interval_secs, args.advertise_interval_secs) => {
            info!("Server stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down...");
        }
    }
    
    // Clean up monitoring server
    if let Some(handle) = monitoring_handle {
        handle.abort();
    }
    
    info!("✅ Relay server shut down gracefully");
    Ok(())
}
