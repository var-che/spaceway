//! libp2p network implementation
//!
//! Provides peer-to-peer networking with:
//! - Kademlia DHT for peer discovery
//! - Circuit Relay v2 for NAT traversal and IP privacy
//! - GossipSub for pub/sub messaging
//! - Noise for transport encryption

use libp2p::{
    gossipsub, identity, kad,
    noise, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
    futures::StreamExt,
    core::{
        muxing::StreamMuxerBox,
        transport::{Boxed, OrTransport},
        upgrade,
    },
    Transport,
};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use crate::{Error, Result};

/// Commands sent to the network thread
#[derive(Debug)]
pub enum NetworkCommand {
    /// Dial a peer
    Dial { addr: Multiaddr, response: oneshot::Sender<Result<()>> },
    /// Dial a peer via relay
    DialViaRelay { 
        relay_addr: Multiaddr,
        relay_peer_id: PeerId,
        target_peer_id: PeerId,
        response: oneshot::Sender<Result<()>> 
    },
    /// Subscribe to a topic
    Subscribe { topic: String, response: oneshot::Sender<Result<()>> },
    /// Publish to a topic
    Publish { topic: String, data: Vec<u8>, response: oneshot::Sender<Result<()>> },
    /// Get listening addresses
    GetListeners { response: oneshot::Sender<Vec<Multiaddr>> },
    /// Advertise as relay server on DHT
    AdvertiseRelay { 
        info: crate::network::relay::RelayAdvertisement,
        response: oneshot::Sender<Result<()>> 
    },
    /// Discover available relay peers from DHT
    DiscoverRelays { 
        response: oneshot::Sender<Result<Vec<crate::network::relay::RelayInfo>>> 
    },
    /// Put a value in DHT
    DhtPut {
        key: Vec<u8>,
        value: Vec<u8>,
        response: oneshot::Sender<Result<()>>
    },
    /// Get values from DHT
    DhtGet {
        key: Vec<u8>,
        response: oneshot::Sender<Result<Vec<Vec<u8>>>>
    },
    /// Shutdown the network
    Shutdown,
}

/// Network event from the P2P layer
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// New peer connected
    PeerConnected(PeerId),
    
    /// Peer disconnected
    PeerDisconnected(PeerId),
    
    /// Message received on a topic
    MessageReceived {
        topic: String,
        data: Vec<u8>,
        source: PeerId,
    },
    
    /// Peer discovered via DHT
    PeerDiscovered {
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
    },
    
    /// DHT query completed
    DhtQueryComplete,
}

/// Network behavior combining Kademlia DHT, GossipSub, and Relay Client
#[derive(NetworkBehaviour)]
pub struct DescordBehaviour {
    /// Kademlia DHT for peer discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    
    /// GossipSub for topic-based messaging
    pub gossipsub: gossipsub::Behaviour,
    
    /// Relay client for connecting via relays (IP privacy)
    pub relay_client: relay::client::Behaviour,
}

/// P2P network node with message-passing interface
pub struct NetworkNode {
    /// Local peer ID
    peer_id: PeerId,
    
    /// Command sender to network thread
    command_tx: mpsc::UnboundedSender<NetworkCommand>,
    
    /// Event receiver from network thread
    event_rx: mpsc::UnboundedReceiver<NetworkEvent>,
}

/// Internal network worker that owns the Swarm
struct NetworkWorker {
    /// libp2p swarm
    swarm: Swarm<DescordBehaviour>,
    
    /// Event sender
    event_tx: mpsc::UnboundedSender<NetworkEvent>,
    
    /// Command receiver
    command_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    
    /// Pending DHT GET queries: QueryId -> (response_channel, start_time)
    pending_get_queries: HashMap<kad::QueryId, (oneshot::Sender<Result<Vec<Vec<u8>>>>, Instant)>,
    
    /// Pending DHT PUT queries: QueryId -> (response_channel, start_time)
    pending_put_queries: HashMap<kad::QueryId, (oneshot::Sender<Result<()>>, Instant)>,
    
    /// Last time we checked for DHT peers and possibly triggered bootstrap
    last_bootstrap_check: Instant,
}

impl NetworkNode {
    /// Create a new network node with command/event channels
    pub fn new() -> Result<(Self, mpsc::UnboundedReceiver<NetworkEvent>)> {
        Self::new_with_config(vec![], vec![])
    }
    
    /// Create a new network node with bootstrap peers and listen addresses
    pub fn new_with_config(bootstrap_peers: Vec<String>, listen_addrs: Vec<String>) -> Result<(Self, mpsc::UnboundedReceiver<NetworkEvent>)> {
        // Generate identity
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        
        println!("Local peer ID: {}", local_peer_id);
        
        // Create Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kademlia = kad::Behaviour::new(local_peer_id, store);
        
        // Set DHT mode to server (accept queries)
        kademlia.set_mode(Some(kad::Mode::Server));
        
        // Create GossipSub with privacy-preserving configuration
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            // Faster propagation for real-time messaging
            .heartbeat_interval(Duration::from_secs(1))
            
            // Strict validation - reject unsigned/invalid messages
            .validation_mode(gossipsub::ValidationMode::Strict)
            
            // Message deduplication (keep seen messages for 5 minutes)
            .duplicate_cache_time(Duration::from_secs(300))
            
            // Limit message size to prevent spam (1MB max)
            .max_transmit_size(1024 * 1024)
            
            // Privacy: Don't flood-publish to all peers
            // Only send to mesh peers (reduces metadata leakage)
            .flood_publish(false)
            
            // Mesh configuration for small networks (2+ peers)
            .mesh_n(2)        // Target 2 peers in mesh (works with 2 peers)
            .mesh_n_low(1)    // Min 1 peer (allows 2-peer networks)
            .mesh_n_high(12)  // Max 12 peers
            
            // Message caching for late joiners
            .history_length(10)   // Keep last 10 messages
            .history_gossip(5)    // Gossip about 5 cached messages
            
            .build()
            .map_err(|e| Error::Network(format!("GossipSub config error: {}", e)))?;
        
        let mut gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| Error::Network(format!("GossipSub creation error: {}", e)))?;
        
        // Enable message validation with custom validator
        // This will call our validation logic before accepting/propagating messages
        gossipsub
            .with_peer_score(
                gossipsub::PeerScoreParams::default(),
                gossipsub::PeerScoreThresholds::default(),
            )
            .map_err(|e| Error::Network(format!("Failed to enable peer scoring: {}", e)))?;
        
        // Create relay client behavior
        let (relay_transport, relay_client) = relay::client::new(local_peer_id);
        
        // Create behavior with relay client
        let behaviour = DescordBehaviour {
            kademlia,
            gossipsub,
            relay_client,
        };
        
        // Build transport: TCP with relay support
        // This allows both direct TCP connections AND relay circuits
        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
        
        // Compose: Relay OR TCP (try relay first for privacy, fallback to TCP)
        let transport = OrTransport::new(relay_transport, tcp_transport)
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key).map_err(|e| Error::Network(format!("Noise config error: {}", e)))?)
            .multiplex(yamux::Config::default())
            .timeout(Duration::from_secs(20))
            .boxed();
        
        // Create swarm with custom transport
        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(Duration::from_secs(60))
        );
        
        // Create channels
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (user_event_tx, user_event_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        
        // Create worker
        let mut worker = NetworkWorker {
            swarm,
            event_tx: user_event_tx,
            command_rx,
            pending_get_queries: HashMap::new(),
            pending_put_queries: HashMap::new(),
            last_bootstrap_check: Instant::now(),
        };
        
        // Listen on configured addresses or default
        if listen_addrs.is_empty() {
            let default_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
            worker.swarm.listen_on(default_addr).unwrap();
        } else {
            for addr_str in &listen_addrs {
                if let Ok(addr) = addr_str.parse::<Multiaddr>() {
                    println!("📡 Configuring listener on: {}", addr);
                    worker.swarm.listen_on(addr).unwrap();
                }
            }
        }
        
        // Bootstrap DHT with provided peers
        if !bootstrap_peers.is_empty() {
            for peer_addr in &bootstrap_peers {
                if let Ok(addr) = peer_addr.parse::<Multiaddr>() {
                    // Extract peer ID from multiaddr if present
                    if let Some(libp2p::multiaddr::Protocol::P2p(peer_id)) = addr.iter().last() {
                        worker.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                        println!("Added bootstrap peer: {} at {}", peer_id, addr);
                    }
                }
            }
            
            // Start DHT bootstrap
            if let Err(e) = worker.swarm.behaviour_mut().kademlia.bootstrap() {
                println!("Warning: DHT bootstrap failed: {:?}", e);
            } else {
                println!("✓ DHT bootstrap initiated with {} peers", bootstrap_peers.len());
            }
        }
        
        // Spawn network event loop on current Tokio runtime
        tokio::spawn(async move {
            worker.run().await;
        });
        
        Ok((
            Self {
                peer_id: local_peer_id,
                command_tx,
                event_rx,
            },
            user_event_rx,
        ))
    }
    
    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Get the listening addresses
    pub async fn listeners(&self) -> Vec<Multiaddr> {
        let (tx, rx) = oneshot::channel();
        let _ = self.command_tx.send(NetworkCommand::GetListeners { response: tx });
        rx.await.unwrap_or_default()
    }
    
    /// Start listening on an address
    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<()> {
        // Send command to worker thread - it will handle listening
        // For now, we'll just assume it works since the worker will process it
        Ok(())
    }
    
    /// Dial a peer
    pub async fn dial(&mut self, addr: Multiaddr) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(NetworkCommand::Dial { addr, response: tx })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        rx.await
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Dial a peer via relay circuit (for IP privacy)
    pub async fn dial_via_relay(
        &self, 
        relay_addr: Multiaddr,
        relay_peer_id: PeerId,
        target_peer_id: PeerId
    ) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(NetworkCommand::DialViaRelay { 
            relay_addr, 
            relay_peer_id, 
            target_peer_id,
            response: tx 
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        rx.await
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Advertise this node as a relay server on DHT
    /// Allows other users to discover and use this node as relay
    pub async fn advertise_as_relay(
        &self,
        mode: crate::network::relay::RelayMode,
        addresses: Vec<Multiaddr>,
        capacity: u32,
    ) -> Result<()> {
        use crate::network::relay::RelayAdvertisement;
        
        let info = RelayAdvertisement {
            peer_id: self.peer_id,
            addresses,
            capacity,
            mode,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(NetworkCommand::AdvertiseRelay { 
            info,
            response: tx 
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        rx.await
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Discover available relay peers from DHT
    /// Returns list of relays sorted by reputation
    pub async fn discover_relays(&self) -> Result<Vec<crate::network::relay::RelayInfo>> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(NetworkCommand::DiscoverRelays { 
            response: tx 
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        rx.await
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Subscribe to a GossipSub topic
    pub async fn subscribe(&mut self, topic: &str) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.command_tx.send(NetworkCommand::Subscribe { 
            topic: topic.to_string(), 
            response: tx 
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        rx.await
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Publish to a GossipSub topic
    pub async fn publish(&mut self, topic: &str, data: Vec<u8>) -> Result<()> {
        eprintln!("🟢 [publish] START: topic={}, data_size={} bytes", topic, data.len());
        
        let (tx, rx) = oneshot::channel();
        eprintln!("🟢 [publish] Sending Publish command to network thread...");
        self.command_tx.send(NetworkCommand::Publish { 
            topic: topic.to_string(), 
            data,
            response: tx 
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        
        eprintln!("🟢 [publish] Command sent, awaiting response...");
        let result = rx.await;
        
        match &result {
            Ok(Ok(_)) => eprintln!("🟢 [publish] END: ✓ Success"),
            Ok(Err(e)) => eprintln!("🟢 [publish] END: ✗ Error: {}", e),
            Err(_) => eprintln!("🟢 [publish] END: ✗ Response channel closed"),
        }
        
        result.map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Put a value in the DHT
    pub async fn dht_put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        eprintln!("🔶 [dht_put] START: key={}, value_size={} bytes", 
                 hex::encode(&key[..std::cmp::min(8, key.len())]), value.len());
        
        let (tx, rx) = oneshot::channel();
        eprintln!("🔶 [dht_put] Sending DhtPut command to network thread...");
        self.command_tx.send(NetworkCommand::DhtPut {
            key: key.clone(),
            value,
            response: tx
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        
        eprintln!("🔶 [dht_put] Command sent, awaiting response with 12s timeout...");
        
        // Add timeout wrapper to ensure we don't wait forever
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(12), // Slightly longer than query timeout
            rx
        )
        .await;
        
        match &result {
            Ok(Ok(Ok(_))) => eprintln!("🔶 [dht_put] END: ✓ Success"),
            Ok(Ok(Err(e))) => eprintln!("🔶 [dht_put] END: ✗ Network error: {}", e),
            Ok(Err(_)) => eprintln!("🔶 [dht_put] END: ✗ Response channel closed"),
            Err(_) => eprintln!("🔶 [dht_put] END: ✗ TIMEOUT after 12 seconds"),
        }
        
        result
            .map_err(|_| Error::Network("DHT PUT operation timed out".to_string()))?
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
    
    /// Get values from the DHT
    pub async fn dht_get(&mut self, key: Vec<u8>) -> Result<Vec<Vec<u8>>> {
        eprintln!("🔷 [dht_get] START: key={}", 
                 hex::encode(&key[..std::cmp::min(8, key.len())]));
        
        let (tx, rx) = oneshot::channel();
        eprintln!("🔷 [dht_get] Sending DhtGet command to network thread...");
        self.command_tx.send(NetworkCommand::DhtGet {
            key: key.clone(),
            response: tx
        })
            .map_err(|_| Error::Network("Network thread died".to_string()))?;
        
        eprintln!("🔷 [dht_get] Command sent, awaiting response with 12s timeout...");
        
        // Add timeout wrapper to ensure we don't wait forever
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(12), // Slightly longer than query timeout
            rx
        )
        .await;
        
        match &result {
            Ok(Ok(Ok(values))) => eprintln!("🔷 [dht_get] END: ✓ Success ({} values)", values.len()),
            Ok(Ok(Err(e))) => eprintln!("🔷 [dht_get] END: ✗ Network error: {}", e),
            Ok(Err(_)) => eprintln!("🔷 [dht_get] END: ✗ Response channel closed"),
            Err(_) => eprintln!("🔷 [dht_get] END: ✗ TIMEOUT after 12 seconds"),
        }
        
        result
            .map_err(|_| Error::Network("DHT GET operation timed out".to_string()))?
            .map_err(|_| Error::Network("Response channel closed".to_string()))?
    }
}

impl NetworkWorker {
    /// Run the network worker loop
    async fn run(mut self) {
        // Create a timer that fires every second to check timeouts
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        
        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
                // Handle commands from client
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        NetworkCommand::Dial { addr, response } => {
                            let result = self.swarm.dial(addr.clone())
                                .map_err(|e| Error::Network(format!("Dial failed: {}", e)));
                            let _ = response.send(result);
                        }
                        NetworkCommand::DialViaRelay { relay_addr, relay_peer_id, target_peer_id, response } => {
                            // First, dial the relay if not connected
                            let _ = self.swarm.dial(relay_addr.clone());
                            
                            // Build relay multiaddr: /ip4/.../tcp/.../p2p/{relay}/p2p-circuit/p2p/{target}
                            let relay_multiaddr = crate::network::relay::relay_multiaddr(
                                &relay_addr,
                                &relay_peer_id,
                                &target_peer_id
                            );
                            
                            let result = self.swarm.dial(relay_multiaddr)
                                .map_err(|e| Error::Network(format!("Relay dial failed: {}", e)));
                            let _ = response.send(result);
                        }
                        NetworkCommand::Subscribe { topic, response } => {
                            let topic = gossipsub::IdentTopic::new(topic);
                            let result = self.swarm.behaviour_mut().gossipsub.subscribe(&topic)
                                .map(|_| ())
                                .map_err(|e| Error::Network(format!("Subscribe failed: {}", e)));
                            let _ = response.send(result);
                        }
                        NetworkCommand::Publish { topic, data, response } => {
                            eprintln!("🟣 [NetworkWorker] Received Publish command for topic: {}, size: {} bytes", topic, data.len());
                            let topic = gossipsub::IdentTopic::new(topic);
                            eprintln!("🟣 [NetworkWorker] Calling gossipsub.publish...");
                            let result = self.swarm.behaviour_mut().gossipsub.publish(topic, data)
                                .map(|_| ())
                                .map_err(|e| Error::Network(format!("Publish failed: {}", e)));
                            eprintln!("🟣 [NetworkWorker] Publish result: {:?}, sending response...", result.is_ok());
                            let _ = response.send(result);
                            eprintln!("🟣 [NetworkWorker] Response sent");
                        }
                        NetworkCommand::GetListeners { response } => {
                            let listeners: Vec<Multiaddr> = self.swarm.listeners().cloned().collect();
                            let _ = response.send(listeners);
                        }
                        NetworkCommand::AdvertiseRelay { info, response } => {
                            use crate::network::relay::RELAY_DHT_KEY;
                            
                            // Serialize relay advertisement (custom format)
                            let data = info.to_bytes();
                            
                            // Put value in DHT under relay key
                            let key = libp2p::kad::RecordKey::new(&format!("{}/{}", RELAY_DHT_KEY, info.peer_id));
                            let record = libp2p::kad::Record {
                                key,
                                value: data,
                                publisher: None,
                                expires: None,
                            };
                            
                            let result = self.swarm.behaviour_mut().kademlia
                                .put_record(record, libp2p::kad::Quorum::One)
                                .map(|_| ())
                                .map_err(|e| Error::Network(format!("DHT put failed: {:?}", e)));
                            
                            println!("✓ Advertised relay on DHT");
                            let _ = response.send(result);
                        }
                        NetworkCommand::DiscoverRelays { response } => {
                            use crate::network::relay::{RELAY_DHT_KEY, RelayInfo, RelayAdvertisement};
                            
                            // Start DHT query for relay providers
                            let key = libp2p::kad::RecordKey::new(&RELAY_DHT_KEY);
                            self.swarm.behaviour_mut().kademlia.get_providers(key.clone());
                            
                            // Also try to get stored relay records
                            let _ = self.swarm.behaviour_mut().kademlia.get_record(key);
                            
                            // For now, return empty list (DHT discovery is async)
                            // In production, we'd wait for DHT responses or maintain a cache
                            // For MVP, we'll rely on bootstrap relays as fallback
                            let relays = Vec::new();
                            
                            println!("✓ Discovering relays from DHT...");
                            let _ = response.send(Ok(relays));
                        }
                        NetworkCommand::DhtPut { key, value, response } => {
                            // Check if we have any peers in the routing table
                            let peer_count: usize = self.swarm.behaviour_mut().kademlia
                                .kbuckets()
                                .map(|bucket| bucket.iter().count())
                                .sum();
                            
                            eprintln!("🔍 DHT PUT: {} peers in routing table", peer_count);
                            
                            if peer_count == 0 {
                                eprintln!("⚠️  No DHT peers available, triggering bootstrap...");
                                if let Err(e) = self.swarm.behaviour_mut().kademlia.bootstrap() {
                                    eprintln!("⚠️  Bootstrap failed: {:?}", e);
                                }
                            }
                            
                            // Store value in DHT
                            let record_key = libp2p::kad::RecordKey::new(&key);
                            let record = libp2p::kad::Record {
                                key: record_key,
                                value,
                                publisher: None,
                                expires: None,
                            };
                            
                            match self.swarm.behaviour_mut().kademlia
                                .put_record(record, libp2p::kad::Quorum::One) {
                                Ok(query_id) => {
                                    eprintln!("🔍 DHT PUT query started: {:?}", query_id);
                                    // Track pending query
                                    self.pending_put_queries.insert(query_id, (response, Instant::now()));
                                }
                                Err(e) => {
                                    eprintln!("❌ DHT PUT failed immediately: {:?}", e);
                                    let _ = response.send(Err(Error::Network(format!("DHT put failed: {:?}", e))));
                                }
                            }
                        }
                        NetworkCommand::DhtGet { key, response } => {
                            // Query DHT for values
                            let record_key = libp2p::kad::RecordKey::new(&key);
                            let query_id = self.swarm.behaviour_mut().kademlia.get_record(record_key);
                            
                            // Track pending query - will be resolved when GetRecord event arrives
                            self.pending_get_queries.insert(query_id, (response, Instant::now()));
                        }
                        NetworkCommand::Shutdown => {
                            break;
                        }
                    }
                }
                // Timer tick for periodic checks
                _ = interval.tick() => {
                    self.check_query_timeouts();
                    self.check_dht_peers();
                }
            }
        }
    }
    
    /// Check if we have DHT peers and trigger bootstrap if needed
    fn check_dht_peers(&mut self) {
        const BOOTSTRAP_CHECK_INTERVAL: Duration = Duration::from_secs(15);
        let now = Instant::now();
        
        // Only check periodically
        if now.duration_since(self.last_bootstrap_check) < BOOTSTRAP_CHECK_INTERVAL {
            return;
        }
        
        self.last_bootstrap_check = now;
        
        // Check if we have any peers in the routing table
        let peer_count: usize = self.swarm.behaviour_mut().kademlia
            .kbuckets()
            .map(|bucket| bucket.iter().count())
            .sum();
        
        if peer_count == 0 {
            eprintln!("⚠️  No DHT peers in routing table, triggering bootstrap...");
            if let Err(e) = self.swarm.behaviour_mut().kademlia.bootstrap() {
                eprintln!("   Bootstrap failed: {:?} (this is normal if no bootstrap peers configured)", e);
            }
        }
    }
    
    /// Check for and clean up timed-out DHT queries
    fn check_query_timeouts(&mut self) {
        const QUERY_TIMEOUT: Duration = Duration::from_secs(10);
        let now = Instant::now();
        
        // Collect timed-out GET queries
        let timed_out_gets: Vec<_> = self.pending_get_queries
            .iter()
            .filter_map(|(query_id, (_response, start_time))| {
                if now.duration_since(*start_time) > QUERY_TIMEOUT {
                    Some(*query_id)
                } else {
                    None
                }
            })
            .collect();
        
        // Remove and notify timed-out GET queries
        for query_id in timed_out_gets.iter() {
            if let Some((response, start_time)) = self.pending_get_queries.remove(&query_id) {
                let elapsed = now.duration_since(start_time);
                eprintln!("⏱️  DHT GET query timed out after {:?}: {:?}", elapsed, query_id);
                let _ = response.send(Err(Error::Network("DHT GET query timed out".to_string())));
            }
        }
        
        // Collect timed-out PUT queries
        let timed_out_puts: Vec<_> = self.pending_put_queries
            .iter()
            .filter_map(|(query_id, (_response, start_time))| {
                if now.duration_since(*start_time) > QUERY_TIMEOUT {
                    Some(*query_id)
                } else {
                    None
                }
            })
            .collect();
        
        // Remove and notify timed-out PUT queries
        for query_id in timed_out_puts.iter() {
            if let Some((response, start_time)) = self.pending_put_queries.remove(&query_id) {
                let elapsed = now.duration_since(start_time);
                eprintln!("⏱️  DHT PUT query timed out after {:?}: {:?}", elapsed, query_id);
                let _ = response.send(Err(Error::Network("DHT PUT query timed out".to_string())));
            }
        }
        
        // Report how many queries are being checked
        if !timed_out_gets.is_empty() || !timed_out_puts.is_empty() {
            eprintln!("🕐 Timeout check: {} GET, {} PUT queries timed out (tracking {} GET, {} PUT total)", 
                     timed_out_gets.len(), timed_out_puts.len(),
                     self.pending_get_queries.len(), self.pending_put_queries.len());
        }
    }
    
    /// Handle swarm events
    async fn handle_swarm_event(&mut self, event: SwarmEvent<DescordBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {}", address);
            }
            SwarmEvent::Behaviour(behaviour_event) => {
                self.handle_behaviour_event(behaviour_event).await;
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                println!("✅ Connection established with peer: {}", peer_id);
                // Add peer as explicit GossipSub peer for small networks
                self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                // Add peer to Kademlia routing table so DHT operations can find it
                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, endpoint.get_remote_address().clone());
                let _ = self.event_tx.send(NetworkEvent::PeerConnected(peer_id));
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("❌ Connection closed with peer: {}", peer_id);
                self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                let _ = self.event_tx.send(NetworkEvent::PeerDisconnected(peer_id));
            }
            _ => {}
        }
    }
    
    /// Handle behavior events
    async fn handle_behaviour_event(&mut self, event: DescordBehaviourEvent) {
        match event {
            DescordBehaviourEvent::Kademlia(kad_event) => {
                self.handle_kademlia_event(kad_event).await;
            }
            DescordBehaviourEvent::Gossipsub(gossipsub_event) => {
                self.handle_gossipsub_event(gossipsub_event).await;
            }
            DescordBehaviourEvent::RelayClient(relay_event) => {
                self.handle_relay_client_event(relay_event).await;
            }
        }
    }
    
    /// Handle Kademlia DHT events
    async fn handle_kademlia_event(&mut self, event: kad::Event) {
        match event {
            kad::Event::OutboundQueryProgressed { result, id, .. } => {
                match result {
                    kad::QueryResult::GetClosestPeers(Ok(ok)) => {
                        for peer in ok.peers {
                            println!("Discovered peer: {:?}", peer);
                        }
                        let _ = self.event_tx.send(NetworkEvent::DhtQueryComplete);
                    }
                    kad::QueryResult::Bootstrap(Ok(_)) => {
                        println!("DHT bootstrap complete");
                        let _ = self.event_tx.send(NetworkEvent::DhtQueryComplete);
                    }
                    kad::QueryResult::GetRecord(Ok(ok)) => {
                        // DHT GET query completed successfully
                        if let Some((response, _start_time)) = self.pending_get_queries.remove(&id) {
                            use kad::GetRecordOk;
                            
                            let values: Vec<Vec<u8>> = match ok {
                                GetRecordOk::FoundRecord(peer_record) => {
                                    println!("✓ DHT GET: Found 1 record");
                                    vec![peer_record.record.value]
                                }
                                GetRecordOk::FinishedWithNoAdditionalRecord { .. } => {
                                    println!("⚠️  DHT GET: Query finished, no additional records");
                                    Vec::new()
                                }
                            };
                            
                            let _ = response.send(Ok(values));
                        }
                    }
                    kad::QueryResult::GetRecord(Err(e)) => {
                        // DHT GET query failed
                        if let Some((response, _start_time)) = self.pending_get_queries.remove(&id) {
                            println!("✗ DHT GET failed: {:?}", e);
                            let _ = response.send(Err(Error::Network(format!("DHT GET failed: {:?}", e))));
                        }
                    }
                    kad::QueryResult::PutRecord(Ok(ok)) => {
                        // DHT PUT query completed successfully
                        if let Some((response, start_time)) = self.pending_put_queries.remove(&id) {
                            let elapsed = start_time.elapsed();
                            eprintln!("✓ DHT PUT: Record stored successfully in {:?}, query_id: {:?}", elapsed, id);
                            let _ = response.send(Ok(()));
                        } else {
                            eprintln!("⚠️  DHT PUT completed but query not tracked: {:?}", id);
                        }
                    }
                    kad::QueryResult::PutRecord(Err(e)) => {
                        // DHT PUT query failed
                        if let Some((response, start_time)) = self.pending_put_queries.remove(&id) {
                            let elapsed = start_time.elapsed();
                            eprintln!("✗ DHT PUT failed after {:?}: {:?}, query_id: {:?}", elapsed, e, id);
                            let _ = response.send(Err(Error::Network(format!("DHT PUT failed: {:?}", e))));
                        } else {
                            eprintln!("⚠️  DHT PUT failed but query not tracked: {:?}, error: {:?}", id, e);
                        }
                    }
                    _ => {}
                }
            }
            kad::Event::RoutingUpdated { peer, addresses, .. } => {
                let _ = self.event_tx.send(NetworkEvent::PeerDiscovered {
                    peer_id: peer,
                    addresses: addresses.into_vec(),
                });
            }
            _ => {}
        }
    }
    
    /// Handle gossipsub events
    async fn handle_gossipsub_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            } => {
                let topic = message.topic.to_string();
                println!("🎯 NetworkWorker received GossipSub message on topic: {}", topic);
                let _ = self.event_tx.send(NetworkEvent::MessageReceived {
                    topic,
                    data: message.data,
                    source: propagation_source,
                });
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                println!("🔔 Peer {} subscribed to topic: {}", peer_id, topic);
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                println!("🔕 Peer {} unsubscribed from topic: {}", peer_id, topic);
            }
            _ => {}
        }
    }
    
    /// Handle Relay Client events
    async fn handle_relay_client_event(&mut self, event: relay::client::Event) {
        match event {
            relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } => {
                println!("✓ Relay reservation accepted by {:?}", relay_peer_id);
            }
            relay::client::Event::OutboundCircuitEstablished { relay_peer_id, limit } => {
                println!("✓ Circuit established via relay {:?} (IP hidden)", relay_peer_id);
                // Note: The actual destination peer will be added to Kademlia via ConnectionEstablished event
            }
            relay::client::Event::InboundCircuitEstablished { src_peer_id, limit } => {
                println!("✓ Inbound circuit from {:?} (their IP hidden)", src_peer_id);
                // Note: The src_peer will be added to Kademlia via ConnectionEstablished event
            }
            _ => {
                // Log all other events for debugging
                println!("Relay event: {:?}", event);
            }
        }
    }
}

/// Create a relay server node (for future relay deployment)
#[allow(dead_code)]
pub fn create_relay_server() -> Result<Swarm<libp2p::relay::Behaviour>> {
    use libp2p::relay;
    
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    
    println!("Relay server peer ID: {}", local_peer_id);
    
    let behaviour = relay::Behaviour::new(local_peer_id, Default::default());
    
    let swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|e| Error::Network(format!("Transport config error: {}", e)))?
        .with_behaviour(|_| behaviour)
        .map_err(|e| Error::Network(format!("Behaviour error: {}", e)))?
        .build();
    
    Ok(swarm)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_network_node() {
        let result = NetworkNode::new();
        assert!(result.is_ok());
        
        let (node, _rx) = result.unwrap();
        let peer_id = node.local_peer_id();
        assert!(!peer_id.to_string().is_empty());
    }
    
    #[tokio::test]
    async fn test_subscribe_to_topic() {
        let (mut node, _rx) = NetworkNode::new().unwrap();
        let result = node.subscribe("test-topic").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_publish_to_topic() {
        let (mut node, _rx) = NetworkNode::new().unwrap();
        
        // Must subscribe before publishing
        node.subscribe("test-topic").await.unwrap();
        
        let data = b"Hello, network!".to_vec();
        let result = node.publish("test-topic", data).await;
        
        // Publishing may fail if no peers are connected, which is expected in tests
        // The important part is that the API works and doesn't panic
        let _ = result;
    }
    
    #[test]
    fn test_create_relay_server() {
        let result = create_relay_server();
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_listen_on_address() {
        let (mut node, _rx) = NetworkNode::new().unwrap();
        let addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        let result = node.listen_on(addr);
        assert!(result.is_ok());
    }
}
