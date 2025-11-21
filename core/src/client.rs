//! Client API for Descord
//!
//! High-level API for interacting with Spaces, Channels, Threads, and Messages.
//! Integrates CRDT operations, MLS encryption, and P2P networking.

use crate::crdt::CrdtOp;
use crate::crypto::signing::Keypair;
use crate::forum::{Space, SpaceManager, Channel, ChannelManager, Thread, ThreadManager, Message};
use crate::mls::provider::{create_provider, DescordProvider};
use crate::network::{NetworkNode, NetworkEvent};
use anyhow::Context;
use crate::storage::Store;
use crate::types::*;
use crate::{Error, Result};

use std::path::PathBuf;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

/// Information about a peer discovered in a space
#[derive(Debug, Clone)]
pub struct SpacePeerInfo {
    /// Peer's libp2p peer ID
    pub peer_id: String,
    
    /// Peer's relay address (circuit relay format, no IP exposed)
    pub relay_address: String,
}

/// Client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Storage directory
    pub storage_path: PathBuf,
    
    /// Network listen addresses
    pub listen_addrs: Vec<String>,
    
    /// Bootstrap peers for DHT
    pub bootstrap_peers: Vec<String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from("./descord-data"),
            listen_addrs: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: vec![],
        }
    }
}

/// Main client for interacting with Descord
pub struct Client {
    /// User's keypair
    keypair: Keypair,
    
    /// User ID (derived from keypair)
    user_id: UserId,
    
    /// Space manager
    space_manager: Arc<RwLock<SpaceManager>>,
    
    /// Channel manager
    channel_manager: Arc<RwLock<ChannelManager>>,
    
    /// Thread manager
    thread_manager: Arc<RwLock<ThreadManager>>,
    
    /// Storage backend for encrypted blobs
    storage: Arc<crate::storage::Storage>,
    
    /// Network node
    network: Arc<RwLock<NetworkNode>>,
    
    /// Network event receiver
    network_rx: Arc<RwLock<mpsc::UnboundedReceiver<NetworkEvent>>>,
    
    /// Storage backend
    store: Arc<Store>,
    
    /// MLS provider (wrapped in Arc<RwLock> for shared mutable access)
    mls_provider: Arc<RwLock<DescordProvider>>,
    
    /// KeyPackage store for MLS member addition
    keypackage_store: Arc<RwLock<crate::mls::KeyPackageStore>>,
    
    /// Current relay information
    current_relay: Arc<RwLock<Option<crate::network::relay::RelayInfo>>>,
    
    /// Relay rotation task handle
    rotation_task: Arc<RwLock<Option<JoinHandle<()>>>>,
    
    /// GossipSub metrics
    gossip_metrics: Arc<crate::network::GossipMetrics>,
}

impl Client {
    /// Create a new client with the given keypair and configuration
    pub fn new(keypair: Keypair, config: ClientConfig) -> Result<Self> {
        let user_id = keypair.user_id();
        
        // Create storage backends
        let store = Arc::new(Store::open(&config.storage_path)?);
        
        // Create managers
        let space_manager = Arc::new(RwLock::new(SpaceManager::new()));
        let channel_manager = Arc::new(RwLock::new(ChannelManager::new()));
        let thread_manager = Arc::new(RwLock::new(ThreadManager::new()));
        
        // Initialize blob storage
        let storage = Arc::new(crate::storage::Storage::open(&config.storage_path)?);
        
        // Create network with bootstrap peers and listen addresses
        let (network_node, network_rx) = NetworkNode::new_with_config(
            config.bootstrap_peers.clone(),
            config.listen_addrs.clone()
        )?;
        let network = Arc::new(RwLock::new(network_node));
        let network_rx = Arc::new(RwLock::new(network_rx));
        
        // Create MLS provider (wrapped in Arc<RwLock> for shared mutable access)
        let mls_provider = Arc::new(RwLock::new(create_provider()));
        
        // Create MLS signer and KeyPackage store
        use openmls::prelude::*;
        use openmls_basic_credential::SignatureKeyPair;
        let ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
        let mls_signer = SignatureKeyPair::new(ciphersuite.signature_algorithm())
            .map_err(|e| crate::Error::Crypto(format!("Failed to create MLS signer: {:?}", e)))?;
        
        let mut kp_store = crate::mls::KeyPackageStore::new(user_id, mls_signer, ciphersuite);
        
        // Generate initial batch of KeyPackages (10 packages)
        // Using try_read() since this is not an async context
        {
            let provider_lock = mls_provider.try_read()
                .map_err(|e| crate::Error::Crypto(format!("Failed to acquire provider lock: {}", e)))?;
            let _key_packages = kp_store.generate_key_packages(10, &provider_lock)?;
            println!("✓ Generated {} KeyPackages for user {}", 10, user_id);
        }
        
        let keypackage_store = Arc::new(RwLock::new(kp_store));
        
        // Create GossipSub metrics
        let gossip_metrics = Arc::new(crate::network::GossipMetrics::new());
        
        Ok(Self {
            keypair,
            user_id,
            space_manager,
            channel_manager,
            thread_manager,
            storage,
            network,
            network_rx,
            store,
            mls_provider,
            keypackage_store,
            current_relay: Arc::new(RwLock::new(None)),
            rotation_task: Arc::new(RwLock::new(None)),
            gossip_metrics,
        })
    }
    
    /// Start the client (network and event processing)
    pub async fn start(&self) -> Result<()> {
        // Subscribe to space discovery topic
        {
            let mut network = self.network.write().await;
            let _ = network.subscribe("descord/space-discovery").await;
            
            // Subscribe to user's personal Welcome message topic for MLS group invitations
            let welcome_topic = format!("user/{}/welcome", hex::encode(&self.user_id.0[..8]));
            let _ = network.subscribe(&welcome_topic).await;
            println!("✓ Subscribed to Welcome message topic: {}", welcome_topic);
        }
        
        // Spawn event processing task
        let space_manager = Arc::clone(&self.space_manager);
        let channel_manager = Arc::clone(&self.channel_manager);
        let thread_manager = Arc::clone(&self.thread_manager);
        let store = Arc::clone(&self.store);
        let network_rx = Arc::clone(&self.network_rx);
        let network = Arc::clone(&self.network);
        let gossip_metrics = Arc::clone(&self.gossip_metrics);
        let mls_provider = Arc::clone(&self.mls_provider); // Clone Arc<RwLock> to share provider
        let user_id = self.user_id; // Clone user_id for the async task
        
        tokio::spawn(async move {
            loop {
                let event_opt = {
                    let mut rx = network_rx.write().await;
                    rx.recv().await
                };
                
                if let Some(event) = event_opt {
                    match event {
                        NetworkEvent::MessageReceived { topic, data, source } => {
                            println!("📬 Client received network message on topic: {}", topic);
                            
                            // Check if this is a sync request (starts with "SYNC_REQUEST:")
                            if let Ok(text) = String::from_utf8(data.clone()) {
                                if text.starts_with("SYNC_REQUEST:") {
                                    println!("  🔄 Received sync request from peer");
                                    if let Some(space_id_hex) = text.strip_prefix("SYNC_REQUEST:") {
                                        println!("    Space ID hex: {}", space_id_hex);
                                        if let Ok(space_id_bytes) = hex::decode(space_id_hex) {
                                            println!("    Decoded {} bytes", space_id_bytes.len());
                                            if space_id_bytes.len() == 32 {
                                                let mut space_id_arr = [0u8; 32];
                                                space_id_arr.copy_from_slice(&space_id_bytes);
                                                let space_id = SpaceId(space_id_arr);
                                                
                                                // Handle sync request inline (we're already in async context)
                                                match store.get_space_ops(&space_id) {
                                                    Ok(ops) => {
                                                        println!("    Found {} operations in storage", ops.len());
                                                        if !ops.is_empty() {
                                                            println!("  📤 Re-broadcasting {} operations for Space", ops.len());
                                                            let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
                                                            for op in ops {
                                                                // Broadcast each operation
                                                                if let Ok(data) = minicbor::to_vec(&op) {
                                                                    let mut net = network.write().await;
                                                                    let _ = net.publish(&space_topic, data).await;
                                                                    drop(net);
                                                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                                                }
                                                            }
                                                            println!("  ✓ Sync complete");
                                                        } else {
                                                            println!("    ⚠️ No operations to send");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!("    ⚠️ Error getting operations: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    continue; // Don't try to decode as CrdtOp
                                }
                            }
                            
                            // Check if this is a Welcome message (on user/{id}/welcome topic)
                            if topic.starts_with("user/") && topic.ends_with("/welcome") {
                                println!("  🎉 Received MLS Welcome message");
                                
                                // Process Welcome message to join MLS group
                                use openmls_basic_credential::SignatureKeyPair;
                                let ciphersuite = openmls::prelude::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;
                                let signer = SignatureKeyPair::new(ciphersuite.signature_algorithm())
                                    .map_err(|e| crate::Error::Crypto(format!("Failed to create signer: {:?}", e)));
                                
                                if let Ok(signer) = signer {
                                    // Use the shared provider that has the KeyPackage private keys
                                    let provider = mls_provider.read().await;
                                    match crate::mls::MlsGroup::from_welcome(
                                        data.clone(),
                                        user_id,
                                        signer,
                                        &provider
                                    ) {
                                    Ok(mls_group) => {
                                        println!("  ✓ Successfully joined MLS group (epoch {})", mls_group.current_epoch().0);
                                        
                                        // Store the MLS group
                                        // TODO: Extract space_id from Welcome context to properly store
                                        // For now we log success but can't store without space_id
                                        println!("  ⚠️ MLS group joined but needs space_id to store (will be synced via CRDT)");
                                    }
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to process Welcome message: {}", e);
                                    }
                                }
                                }
                                
                                continue; // Don't try to decode as CrdtOp
                            }
                            
                            // Check for MLS encryption marker and decode the operation
                            let op = if data.first() == Some(&0x01) {
                                // MLS-encrypted - decrypt it
                                println!("  🔒 MLS-encrypted message detected");
                                
                                // Message format: [0x01][space_id (32 bytes)][encrypted_data]
                                if data.len() < 33 {
                                    eprintln!("  ⚠️ MLS message too short (need at least 33 bytes)");
                                    continue;
                                }
                                
                                // Extract space_id from message
                                let space_id_bytes: [u8; 32] = match data[1..33].try_into() {
                                    Ok(bytes) => bytes,
                                    Err(_) => {
                                        eprintln!("  ⚠️ Invalid space_id in MLS message");
                                        continue;
                                    }
                                };
                                let space_id = SpaceId(space_id_bytes);
                                
                                // Get the encrypted data (after marker + space_id)
                                let encrypted_data = &data[33..];
                                
                                // Decrypt using the space's MLS group
                                let decrypted_bytes = {
                                    let mut space_mgr = space_manager.write().await;
                                    let provider = mls_provider.read().await;
                                    
                                    match space_mgr.get_mls_group_mut(&space_id) {
                                        Some(mls_group) => {
                                            match mls_group.decrypt_application_message(encrypted_data, &provider) {
                                                Ok(plaintext) => {
                                                    println!("  ✓ Decrypted MLS message ({} bytes)", plaintext.len());
                                                    plaintext
                                                }
                                                Err(e) => {
                                                    eprintln!("  ⚠️ Failed to decrypt MLS message: {}", e);
                                                    eprintln!("     (You may have been removed from this Space)");
                                                    continue;
                                                }
                                            }
                                        }
                                        None => {
                                            eprintln!("  ⚠️ No MLS group found for space_id {}", hex::encode(&space_id.0[..8]));
                                            eprintln!("     (You may not be a member of this Space)");
                                            continue;
                                        }
                                    }
                                };
                                
                                // Decode the decrypted operation
                                match minicbor::decode::<CrdtOp>(&decrypted_bytes) {
                                    Ok(op) => op,
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to decode decrypted operation: {}", e);
                                        continue;
                                    }
                                }
                            } else if data.first() == Some(&0x00) {
                                // Plaintext - strip marker and decode
                                match minicbor::decode::<CrdtOp>(&data[1..]) {
                                    Ok(op) => op,
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to decode operation: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                // Legacy format (no marker) - assume plaintext
                                match minicbor::decode::<CrdtOp>(&data[..]) {
                                    Ok(op) => op,
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to decode operation: {}", e);
                                        continue;
                                    }
                                }
                            };
                            
                            // Process the decoded operation
                            println!("  ✓ Decoded operation: {:?}", op.op_type);
                            // Verify signature before processing
                            if !op.verify_signature() {
                                eprintln!("⚠️ Rejected message with invalid signature from {:?}", source);
                                continue;
                            }
                            println!("  ✓ Signature verified");
                            
                            // Check if we've already processed this operation (deduplication)
                            let is_duplicate = if let Ok(Some(_)) = store.get_op(&op.op_id) {
                                // Already seen this op, skip processing
                                gossip_metrics.record_receive(&topic, true).await;
                                println!("  ⚠️ Duplicate operation, skipping");
                                true
                            } else {
                                gossip_metrics.record_receive(&topic, false).await;
                                false
                            };
                            
                            if is_duplicate {
                                continue;
                            }
                            println!("  ✓ Not a duplicate, processing...");
                            
                            tracing::debug!(
                                op_id = ?op.op_id,
                                op_type = ?op.op_type,
                                topic = %topic,
                                source = ?source,
                                "Received and validated CRDT operation"
                            );
                            
                            // If this is a CreateSpace on discovery topic, auto-subscribe to the space
                            if topic == "descord/space-discovery" {
                                if let crate::crdt::OpType::CreateSpace(payload) = &op.op_type {
                                    if let crate::crdt::OpPayload::CreateSpace { name, .. } = payload {
                                                println!("📢 Discovered space: {} (space_{})", name, ::hex::encode(&op.space_id.0[..4]));
                                                
                                                // Auto-subscribe to the space topic
                                                let space_topic = format!("space/{}", ::hex::encode(&op.space_id.0[..8]));
                                                let mut net = network.write().await;
                                                if let Ok(_) = net.subscribe(&space_topic).await {
                                                    println!("  → Auto-subscribed to {}", space_topic);
                                                }
                                                drop(net);
                                            }
                                        }
                                    }
                                    
                                    // Store the operation (persistence + deduplication)
                                    if let Err(e) = store.put_op(&op) {
                                        eprintln!("⚠️ Failed to store operation: {}", e);
                                        continue;
                                    }
                                    
                                    // Process based on operation type
                                    match &op.op_type {
                                        crate::crdt::OpType::CreateSpace(payload) => {
                                            if let crate::crdt::OpPayload::CreateSpace { name, .. } = payload {
                                                let mut manager = space_manager.write().await;
                                                let _ = manager.process_create_space(&op);
                                                
                                                println!("✓ Processed CreateSpace: {} ({})", name, op.space_id);
                                            }
                                        }
                                        crate::crdt::OpType::UpdateSpaceVisibility(_) => {
                                            let mut manager = space_manager.write().await;
                                            let _ = manager.process_update_space_visibility(&op);
                                        }
                                        crate::crdt::OpType::CreateChannel(_) => {
                                            let mut manager = channel_manager.write().await;
                                            let _ = manager.process_create_channel(&op);
                                        }
                                        crate::crdt::OpType::CreateThread(_) => {
                                            let mut manager = thread_manager.write().await;
                                            let _ = manager.process_create_thread(&op);
                                        }
                                        crate::crdt::OpType::PostMessage(_) => {
                                            let mut manager = thread_manager.write().await;
                                            let _ = manager.process_post_message(&op);
                                        }
                                        crate::crdt::OpType::EditMessage(_) => {
                                            let mut manager = thread_manager.write().await;
                                            let _ = manager.process_edit_message(&op);
                                        }
                                        _ => {}
                                    }
                        }
                        NetworkEvent::PeerConnected(peer_id) => {
                            println!("Peer connected: {}", peer_id);
                            // Note: Space discovery subscription happens in start() before event loop
                        }
                        NetworkEvent::PeerDisconnected(peer_id) => {
                            println!("Peer disconnected: {}", peer_id);
                        }
                        _ => {}
                    }
                } else {
                    // Channel closed, exit loop
                    break;
                }
            }
        });
        
        // Give the network a moment to start listening
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        Ok(())
    }
    
    /// Get the user's ID
    pub fn user_id(&self) -> UserId {
        self.user_id
    }
    
    /// Create a new Space
    pub async fn create_space(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<(Space, CrdtOp, PrivacyInfo)> {
        self.create_space_with_visibility(name, description, SpaceVisibility::default()).await
    }

    /// Create a new Space with specific visibility
    /// 
    /// Privacy Warning: This function returns privacy information that MUST be shown to the user
    /// before the space is created, especially for Public spaces which expose IP addresses.
    pub async fn create_space_with_visibility(
        &self,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
    ) -> Result<(Space, CrdtOp, PrivacyInfo)> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let space_id = SpaceId::from_content(&self.user_id, &name, timestamp);
        
        // Generate privacy information for user consent
        let privacy_info = PrivacyInfo::from_visibility(visibility);
        
        // Clone name before passing to manager (which consumes it)
        let name_for_announcement = name.clone();
        
        let mut manager = self.space_manager.write().await;
        let provider = self.mls_provider.read().await;
        let op = manager.create_space_with_visibility(
            space_id,
            name,
            description,
            visibility,
            self.user_id,
            &self.keypair,
            &provider,
        )?;
                drop(provider);        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation on space topic
        self.broadcast_op(&op).await?;
        
        // Auto-subscribe to the space topic
        self.subscribe_to_space(&space_id).await?;
        
        // ALSO broadcast CreateSpace on discovery topic so peers can discover and join
        // This allows peers who aren't subscribed to the space yet to receive the initial CreateSpace op
        let _ = self.broadcast_op_on_topic(&op, "descord/space-discovery").await;
        
        let space = manager.get_space(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?
            .clone();
        
        // Store Space metadata in DHT for offline discovery
        drop(manager); // Release lock before calling dht_put_space
        if let Err(e) = self.dht_put_space(&space_id).await {
            eprintln!("⚠️  Failed to store Space in DHT: {}", e);
            // Non-fatal - space still created locally
        }
        
        Ok((space, op, privacy_info))
    }

    /// Get privacy information for joining a space (to show before join)
    pub fn get_join_privacy_info(&self, visibility: SpaceVisibility) -> PrivacyInfo {
        PrivacyInfo::from_visibility(visibility)
    }

    /// Update a Space's visibility (admins only)
    pub async fn update_space_visibility(
        &self,
        space_id: SpaceId,
        visibility: SpaceVisibility,
    ) -> Result<CrdtOp> {
        let mut manager = self.space_manager.write().await;
        let op = manager.update_space_visibility(
            space_id,
            visibility,
            self.user_id,
            &self.keypair,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// Create an invite for a space
    pub async fn create_invite(
        &self,
        space_id: SpaceId,
        max_uses: Option<u32>,
        max_age_hours: Option<u32>,
    ) -> Result<CrdtOp> {
        let mut manager = self.space_manager.write().await;
        let op = manager.create_invite(
            space_id,
            self.user_id,
            &self.keypair,
            max_uses,
            max_age_hours,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// Revoke an invite
    pub async fn revoke_invite(
        &self,
        space_id: SpaceId,
        invite_id: InviteId,
    ) -> Result<CrdtOp> {
        let mut manager = self.space_manager.write().await;
        let op = manager.revoke_invite(
            space_id,
            invite_id,
            self.user_id,
            &self.keypair,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// Join a space using an invite code
    /// 
    /// Automatically fetches Space metadata from DHT if creator is offline.
    pub async fn join_with_invite(
        &self,
        space_id: SpaceId,
        code: String,
    ) -> Result<CrdtOp> {
        // Subscribe to space topic FIRST so we can receive operations via GossipSub
        println!("ℹ Subscribing to Space topic...");
        self.subscribe_to_space(&space_id).await?;
        
        // First check if we have the Space locally
        let has_space = {
            let manager = self.space_manager.read().await;
            manager.get_space(&space_id).is_some()
        };
        
        // If Space doesn't exist locally, try fetching from DHT or create placeholder
        if !has_space {
            println!("⚠️  Space not found locally, will sync via GossipSub from connected peers...");
            
            // Try DHT as a fallback
            match self.dht_get_space(&space_id).await {
                Ok(space) => {
                    println!("✓ Retrieved Space '{}' from DHT", space.name);
                    
                    // Store space metadata locally
                    let mut manager = self.space_manager.write().await;
                    manager.add_space_from_dht(space);
                    drop(manager); // Release lock for async operation
                    
                    // Fetch CRDT operations from DHT to rebuild state
                    match self.dht_get_operations(&space_id).await {
                        Ok(ops) => {
                            if !ops.is_empty() {
                                println!("✓ Fetched {} operations from DHT", ops.len());
                                for op in ops {
                                    if let Err(e) = self.handle_incoming_op(op).await {
                                        eprintln!("⚠ Failed to apply operation: {}", e);
                                    }
                                }
                                println!("✓ Applied operations to rebuild Space state");
                            }
                        }
                        Err(e) => {
                            eprintln!("⚠ Failed to fetch operations from DHT: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("⚠ DHT fetch failed: {}", e);
                    println!("  Requesting sync from connected peers via GossipSub...");
                    
                    // Broadcast a sync request on the Space topic
                    let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
                    let sync_request = format!("SYNC_REQUEST:{}", hex::encode(&space_id.0));
                    if let Err(e) = self.broadcast_raw(&space_topic, sync_request.as_bytes().to_vec()).await {
                        eprintln!("⚠ Failed to send sync request: {}", e);
                    }
                    
                    // Wait for peers to respond with operations
                    println!("  Waiting 3 seconds for peers to resend Space operations...");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    
                    // Check if we received the Space
                    let manager = self.space_manager.read().await;
                    if manager.get_space(&space_id).is_none() {
                        drop(manager);
                        println!("  Tip: Make sure you're connected to the Space creator");
                        println!("  Use 'network' to check connections, 'connect <multiaddr>' to connect");
                        return Err(Error::NotFound(format!(
                            "Space not found. Connect to the Space creator first, then try again."
                        )));
                    }
                    drop(manager);
                    println!("✓ Received Space data from peer");
                }
            }
        }
        
        let mut manager = self.space_manager.write().await;
        let op = manager.use_invite(
            space_id,
            code,
            self.user_id,
            &self.keypair,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        // Subscribe to space topic for future updates
        drop(manager);
        self.subscribe_to_space(&space_id).await?;
        
        Ok(op)
    }
    
    /// List all invites for a space
    pub async fn list_invites(&self, space_id: &SpaceId) -> Vec<Invite> {
        let manager = self.space_manager.read().await;
        manager.list_invites(space_id).into_iter().cloned().collect()
    }
    
    /// Sync a Space from DHT (useful after being added as a member via GossipSub)
    /// 
    /// When you're added to a Space via GossipSub, you receive the AddMember operation
    /// but not the historical operations (CreateSpace, CreateChannel, messages, etc.).
    /// This method fetches all historical operations from DHT and applies them.
    pub async fn sync_space_from_dht(&self, space_id: SpaceId) -> Result<()> {
        println!("🔄 Syncing Space {} from DHT...", space_id);
        
        // Fetch CRDT operations from DHT
        let ops = self.dht_get_operations(&space_id).await?;
        
        println!("  → Fetched {} operations from DHT", ops.len());
        
        // Apply operations to rebuild state
        if !ops.is_empty() {
            for op in &ops {
                // Apply each operation (this rebuilds channels, threads, messages, etc.)
                if let Err(e) = self.handle_incoming_op(op.clone()).await {
                    eprintln!("⚠ Failed to apply operation: {}", e);
                }
            }
            println!("✓ Synced Space state from {} operations", ops.len());
        }
        
        // Subscribe to space topic for future updates
        self.subscribe_to_space(&space_id).await?;
        
        Ok(())
    }
    
    /// Join a space by fetching metadata from DHT (works when creator is offline)
    /// 
    /// This is the primary way to join a space when you have the Space ID but
    /// the creator is not online. The Space metadata is retrieved from the DHT.
    pub async fn join_space_from_dht(&self, space_id: SpaceId) -> Result<crate::forum::Space> {
        // First, try to get the space from DHT
        let space = self.dht_get_space(&space_id).await?;
        
        // Add space to local manager
        let mut manager = self.space_manager.write().await;
        
        // Check if we already have this space
        if manager.get_space(&space_id).is_some() {
            println!("ℹ️  Space already exists locally: {}", space.name);
            return Ok(space);
        }
        
        // Fetch CRDT operations from DHT
        drop(manager); // Release lock for async operation
        let ops = self.dht_get_operations(&space_id).await?;
        
        println!("✓ Joined Space from DHT: {}", space.name);
        println!("  Space ID: {}", space_id);
        println!("  Owner: {}", space.owner);
        println!("  Members: {}", space.members.len());
        println!("  Operations fetched: {}", ops.len());
        
        // Apply operations to rebuild state
        if !ops.is_empty() {
            for op in ops {
                // Apply each operation (this rebuilds channels, threads, messages, etc.)
                if let Err(e) = self.handle_incoming_op(op).await {
                    eprintln!("⚠ Failed to apply operation: {}", e);
                }
            }
            println!("✓ Applied operations to rebuild Space state");
        }
        
        // Subscribe to space topic for future updates
        self.subscribe_to_space(&space_id).await?;
        
        Ok(space)
    }
    
    // ========================================================================
    // DHT Space Metadata Storage (Phase 2: Persistent Distributed Storage)
    // ========================================================================
    
    /// Store Space metadata in the DHT for offline discovery
    /// 
    /// This allows other users to join the Space even when the creator is offline.
    /// The metadata is encrypted and can only be decrypted by those who know the Space ID.
    pub async fn dht_put_space(&self, space_id: &SpaceId) -> Result<()> {
        use crate::forum::{SpaceMetadata, EncryptedSpaceMetadata};
        
        // Get the space
        let manager = self.space_manager.read().await;
        let space = manager.get_space(space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
        
        // Create metadata (convert Keypair to ed25519_dalek::SigningKey)
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&self.keypair.to_bytes());
        let metadata = SpaceMetadata::from_space(space, &signing_key);
        
        // Encrypt metadata
        let encrypted = EncryptedSpaceMetadata::encrypt(&metadata)?;
        
        // Serialize for DHT
        let value = encrypted.to_bytes()?;
        
        // Compute DHT key
        let key = EncryptedSpaceMetadata::dht_key(space_id);
        
        // Store in DHT
        let mut network = self.network.write().await;
        network.dht_put(key, value).await?;
        
        println!("✓ Stored Space metadata in DHT: {}", space.name);
        
        Ok(())
    }
    
    /// Retrieve Space metadata from the DHT
    /// 
    /// This allows joining a Space even when the creator is offline.
    pub async fn dht_get_space(&self, space_id: &SpaceId) -> Result<crate::forum::Space> {
        use crate::forum::{SpaceMetadata, EncryptedSpaceMetadata};
        
        // Compute DHT key
        let key = EncryptedSpaceMetadata::dht_key(space_id);
        
        // Query DHT
        let mut network = self.network.write().await;
        let values = network.dht_get(key).await?;
        
        if values.is_empty() {
            return Err(Error::NotFound(format!("Space {:?} not found in DHT", space_id)));
        }
        
        // Deserialize first value
        let encrypted = EncryptedSpaceMetadata::from_bytes(&values[0])?;
        
        // Decrypt metadata
        let metadata = encrypted.decrypt()?;
        
        // Verify signature
        if !metadata.verify_signature() {
            return Err(Error::InvalidSignature);
        }
        
        // Verify Space ID matches
        if metadata.id != *space_id {
            return Err(Error::InvalidOperation("Space ID mismatch".to_string()));
        }
        
        // Convert metadata to Space
        let space = crate::forum::Space {
            id: metadata.id,
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            owner: metadata.owner,
            members: metadata.initial_members.clone(),
            visibility: metadata.visibility,
            invites: std::collections::HashMap::new(), // Start with no invites
            invite_permissions: metadata.invite_permissions.clone(),
            epoch: metadata.epoch,
            created_at: metadata.created_at,
        };
        
        println!("✓ Retrieved Space from DHT: {}", space.name);
        
        Ok(space)
    }
    
    /// Store CRDT operations in the DHT
    /// 
    /// Batches operations and stores them encrypted for later retrieval.
    /// This enables offline message history sync.
    pub async fn dht_put_operations(
        &self,
        space_id: &SpaceId,
        ops: Vec<CrdtOp>,
    ) -> Result<()> {
        use crate::crdt::{OperationBatch, EncryptedOperationBatch, OperationBatchIndex};
        
        if ops.is_empty() {
            return Ok(());
        }
        
        // First, fetch or create the index
        let mut network = self.network.write().await;
        let index_key = OperationBatchIndex::compute_dht_key(space_id);
        
        let mut index = match network.dht_get(index_key.clone()).await {
            Ok(values) if !values.is_empty() => {
                OperationBatchIndex::from_bytes(&values[0])?
            }
            _ => {
                // Create new index
                OperationBatchIndex::new(*space_id)
            }
        };
        
        // Get next sequence number
        let sequence = index.batch_sequences.last().copied().unwrap_or(0) + 1;
        
        // Create operation batch
        let batch = OperationBatch::new(*space_id, ops.clone(), sequence);
        
        // Encrypt batch
        let encrypted = EncryptedOperationBatch::encrypt(&batch)?;
        
        // Store batch in DHT
        let batch_key = encrypted.dht_key();
        let batch_bytes = encrypted.to_bytes()?;
        network.dht_put(batch_key, batch_bytes).await?;
        
        // Update index
        index.add_batch(sequence, ops.len() as u32);
        
        // Store updated index
        let index_bytes = index.to_bytes()?;
        network.dht_put(index_key, index_bytes).await?;
        
        println!("✓ Stored {} operations in DHT (batch {})", ops.len(), sequence);
        
        Ok(())
    }
    
    /// Retrieve CRDT operations from the DHT
    /// 
    /// Fetches all operation batches for a Space and returns them in order.
    pub async fn dht_get_operations(&self, space_id: &SpaceId) -> Result<Vec<CrdtOp>> {
        use crate::crdt::{EncryptedOperationBatch, OperationBatchIndex};
        
        // Fetch index
        let mut network = self.network.write().await;
        let index_key = OperationBatchIndex::compute_dht_key(space_id);
        
        let index = match network.dht_get(index_key).await {
            Ok(values) if !values.is_empty() => {
                OperationBatchIndex::from_bytes(&values[0])?
            }
            _ => {
                // No operations stored yet
                return Ok(Vec::new());
            }
        };
        
        // Fetch all batches
        let mut all_ops = Vec::new();
        
        for sequence in &index.batch_sequences {
            // Compute batch key
            let batch_key = EncryptedOperationBatch::compute_dht_key(space_id, *sequence);
            
            // Fetch batch
            match network.dht_get(batch_key).await {
                Ok(values) if !values.is_empty() => {
                    let encrypted = EncryptedOperationBatch::from_bytes(&values[0])?;
                    let batch = encrypted.decrypt()?;
                    
                    // Verify Space ID matches
                    if batch.space_id != *space_id {
                        return Err(Error::InvalidOperation("Space ID mismatch in batch".to_string()));
                    }
                    
                    all_ops.extend(batch.operations);
                }
                _ => {
                    // Batch not found, skip (might be still propagating)
                    println!("⚠ Batch {} not found in DHT", sequence);
                }
            }
        }
        
        println!("✓ Retrieved {} operations from DHT", all_ops.len());
        
        Ok(all_ops)
    }
    
    // ========================================================================
    // DHT Blob Storage (Phase 4: Encrypted Blob Replication)
    // ========================================================================
    
    /// Store an encrypted blob in the DHT for offline availability
    /// 
    /// Takes a locally-encrypted blob and encrypts it again with the Space-derived
    /// key before storing in the DHT. This allows Space members to discover and
    /// fetch blobs even when the original author is offline.
    pub async fn dht_put_blob(
        &self,
        space_id: &SpaceId,
        blob_hash: &crate::storage::BlobHash,
        local_blob: &crate::storage::EncryptedBlob,
    ) -> Result<()> {
        use crate::storage::{DhtBlob, BlobIndex};
        
        // Encrypt blob for DHT storage
        let dht_blob = DhtBlob::encrypt(space_id, blob_hash, local_blob)?;
        
        // Serialize for DHT
        let blob_bytes = dht_blob.to_bytes()?;
        
        // Compute DHT key
        let blob_key = dht_blob.dht_key();
        
        // First, fetch or create the index
        let mut network = self.network.write().await;
        let index_key = BlobIndex::compute_dht_key(space_id);
        
        let mut index = match network.dht_get(index_key.clone()).await {
            Ok(values) if !values.is_empty() => {
                BlobIndex::from_bytes(&values[0])?
            }
            _ => {
                // Create new index
                BlobIndex::new(*space_id)
            }
        };
        
        // Store blob in DHT
        network.dht_put(blob_key, blob_bytes).await?;
        
        // Update index (approximate size - we don't track exact size here)
        index.add_blob(*blob_hash, dht_blob.ciphertext.len() as u64);
        
        // Store updated index
        let index_bytes = index.to_bytes()?;
        network.dht_put(index_key, index_bytes).await?;
        
        println!("✓ Stored blob in DHT: {} bytes", dht_blob.ciphertext.len());
        
        Ok(())
    }
    
    /// Retrieve an encrypted blob from the DHT
    /// 
    /// Fetches the blob, decrypts the DHT layer, and returns the locally-encrypted
    /// blob. The caller must then decrypt with the local key.
    pub async fn dht_get_blob(
        &self,
        space_id: &SpaceId,
        blob_hash: &crate::storage::BlobHash,
    ) -> Result<crate::storage::EncryptedBlob> {
        use crate::storage::DhtBlob;
        
        // Compute DHT key
        let blob_key = DhtBlob::compute_dht_key(space_id, blob_hash);
        
        // Fetch from DHT
        let mut network = self.network.write().await;
        let values = network.dht_get(blob_key).await?;
        
        if values.is_empty() {
            return Err(Error::NotFound(format!("Blob {:?} not found in DHT", blob_hash.to_hex())));
        }
        
        // Deserialize and decrypt
        let dht_blob = DhtBlob::from_bytes(&values[0])?;
        
        // Verify Space ID and hash match
        if dht_blob.space_id != *space_id {
            return Err(Error::InvalidOperation("Space ID mismatch in blob".to_string()));
        }
        if dht_blob.content_hash != *blob_hash {
            return Err(Error::InvalidOperation("Blob hash mismatch".to_string()));
        }
        
        // Decrypt DHT layer to get locally-encrypted blob
        let local_blob = dht_blob.decrypt()?;
        
        println!("✓ Retrieved blob from DHT: {} bytes", dht_blob.ciphertext.len());
        
        Ok(local_blob)
    }
    
    /// Retrieve all blob hashes available in the DHT for a Space
    /// 
    /// Useful for discovering what blobs can be fetched.
    pub async fn dht_list_blobs(&self, space_id: &SpaceId) -> Result<Vec<crate::storage::BlobHash>> {
        use crate::storage::BlobIndex;
        
        // Fetch index
        let mut network = self.network.write().await;
        let index_key = BlobIndex::compute_dht_key(space_id);
        
        let index = match network.dht_get(index_key).await {
            Ok(values) if !values.is_empty() => {
                BlobIndex::from_bytes(&values[0])?
            }
            _ => {
                // No blobs stored yet
                return Ok(Vec::new());
            }
        };
        
        println!("✓ Found {} blobs in DHT for Space", index.blob_hashes.len());
        
        Ok(index.blob_hashes)
    }
    
    // ============ MLS KeyPackage Management ============
    
    /// Get a KeyPackage bundle for this user (for direct P2P exchange)
    /// 
    /// This allows direct KeyPackage exchange between connected peers without using DHT.
    /// Useful for 2-peer scenarios where DHT quorum cannot be achieved.
    pub async fn get_key_package_bundle(&self) -> Result<crate::mls::KeyPackageBundle> {
        let mut kp_store = self.keypackage_store.write().await;
        kp_store.get_key_package_bundle()
    }
    
    /// Publish this user's KeyPackages to the DHT
    /// 
    /// Other users can fetch these KeyPackages to add this user to their MLS groups.
    pub async fn publish_key_packages_to_dht(&self) -> Result<()> {
        use sha2::{Sha256, Digest};
        
        // Get KeyPackages from store
        let mut kp_store = self.keypackage_store.write().await;
        let provider = self.mls_provider.read().await;
        let bundles = kp_store.generate_key_packages(5, &provider)?;
        drop(provider);
        drop(kp_store);
        
        if bundles.is_empty() {
            return Ok(());
        }
        
        // Compute DHT key: SHA256("keypackage:" + user_id_hex)
        let user_id_hex = hex::encode(&self.user_id.0);
        let mut hasher = Sha256::new();
        hasher.update(b"keypackage:");
        hasher.update(user_id_hex.as_bytes());
        let dht_key_hash = hasher.finalize();
        let mut dht_key = Vec::new();
        dht_key.extend_from_slice(&dht_key_hash[..32]);
        
        // Serialize all bundles
        let bundles_bytes = serde_json::to_vec(&bundles)
            .map_err(|e| Error::Serialization(format!("Failed to serialize KeyPackages: {}", e)))?;
        
        // Store in DHT
        let mut network = self.network.write().await;
        network.dht_put(dht_key, bundles_bytes).await?;
        
        println!("✓ Published {} KeyPackages to DHT for user {}", bundles.len(), self.user_id);
        
        Ok(())
    }
    
    /// Fetch a user's KeyPackages from the DHT
    /// 
    /// Returns one KeyPackageBundle that can be used to add the user to an MLS group.
    pub async fn fetch_key_package_from_dht(&self, user_id: &UserId) -> Result<crate::mls::KeyPackageBundle> {
        use sha2::{Sha256, Digest};
        
        // Compute DHT key
        let user_id_hex = hex::encode(&user_id.0);
        let mut hasher = Sha256::new();
        hasher.update(b"keypackage:");
        hasher.update(user_id_hex.as_bytes());
        let dht_key_hash = hasher.finalize();
        let mut dht_key = Vec::new();
        dht_key.extend_from_slice(&dht_key_hash[..32]);
        
        // Fetch from DHT
        let mut network = self.network.write().await;
        let values = network.dht_get(dht_key).await?;
        
        if values.is_empty() {
            return Err(Error::NotFound(format!("No KeyPackages found for user {}", user_id)));
        }
        
        // Deserialize bundles
        let bundles: Vec<crate::mls::KeyPackageBundle> = serde_json::from_slice(&values[0])
            .map_err(|e| Error::Serialization(format!("Failed to deserialize KeyPackages: {}", e)))?;
        
        if bundles.is_empty() {
            return Err(Error::NotFound(format!("No KeyPackages available for user {}", user_id)));
        }
        
        // Return the first bundle (in production, we'd consume it)
        println!("✓ Fetched KeyPackage for user {} from DHT", user_id);
        Ok(bundles[0].clone())
    }
    
    /// Get a specific invite
    pub async fn get_invite(&self, space_id: &SpaceId, invite_id: &InviteId) -> Option<Invite> {
        let manager = self.space_manager.read().await;
        manager.get_invite(space_id, invite_id).cloned()
    }
    
    /// Get a Space by ID
    pub async fn get_space(&self, space_id: &SpaceId) -> Option<Space> {
        let manager = self.space_manager.read().await;
        manager.get_space(space_id).cloned()
    }
    
    /// List all Spaces
    pub async fn list_spaces(&self) -> Vec<Space> {
        let manager = self.space_manager.read().await;
        manager.list_spaces().into_iter().cloned().collect()
    }
    
    /// Add a member to a Space
    pub async fn add_member(
        &self,
        space_id: SpaceId,
        user_id: UserId,
        role: Role,
    ) -> Result<CrdtOp> {
        let mut manager = self.space_manager.write().await;
        let op = manager.add_member(
            space_id,
            user_id,
            role,
            self.user_id,
            &self.keypair,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// Add a member to a Space with MLS using a provided KeyPackage bundle
    /// 
    /// This method allows direct P2P KeyPackage exchange without DHT.
    /// Useful for 2-peer scenarios where DHT quorum cannot be achieved.
    pub async fn add_member_with_key_package_bundle(
        &self,
        space_id: SpaceId,
        user_id: UserId,
        role: Role,
        key_package_bundle: crate::mls::KeyPackageBundle,
    ) -> Result<CrdtOp> {
        println!("🔑 Adding member {} with provided KeyPackage...", user_id);
        
        // Step 1: Deserialize the KeyPackage
        let provider = self.mls_provider.read().await;
        let key_package = crate::mls::KeyPackageStore::deserialize_key_package(
            &key_package_bundle,
            &provider
        )?;
        
        // Step 2: Add member to MLS group and get messages to distribute
        let mut manager = self.space_manager.write().await;
        let (commit_msg, welcome_msg) = manager.add_member_with_mls(
            &space_id,
            user_id,
            role,
            key_package,
            &self.user_id,
            &provider,
        )?;
        drop(provider);
        drop(manager);
        
        println!("  ✓ Added to MLS group, epoch rotated");
        
        // Step 3: Serialize messages
        let commit_bytes = commit_msg.to_bytes()
            .map_err(|e| Error::Serialization(format!("Failed to serialize Commit: {}", e)))?;
        let welcome_bytes = welcome_msg.to_bytes()
            .map_err(|e| Error::Serialization(format!("Failed to serialize Welcome: {}", e)))?;
        
        // Step 4: Publish Commit to existing members via GossipSub
        let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
        {
            let mut network = self.network.write().await;
            network.publish(&space_topic, commit_bytes).await?;
        }
        println!("  ✓ Published Commit to existing members on {}", space_topic);
        
        // Step 5: Send Welcome message to new member via their user topic
        let user_topic = format!("user/{}/welcome", user_id);
        {
            let mut network = self.network.write().await;
            network.publish(&user_topic, welcome_bytes).await?;
        }
        println!("  ✓ Sent Welcome message to {} on {}", user_id, user_topic);
        
        // Step 6: Create and broadcast the CRDT AddMember operation
        let mut manager = self.space_manager.write().await;
        let op = manager.add_member(
            space_id,
            user_id,
            role,
            self.user_id,
            &self.keypair,
        )?;
        drop(manager);
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        println!("✅ Member {} added with MLS (P2P KeyPackage)", user_id);
        
        Ok(op)
    }
    
    /// Add a member to a Space with full MLS integration
    /// 
    /// This method:
    /// 1. Fetches the user's KeyPackage from DHT
    /// 2. Adds them to the MLS group (triggering key rotation)
    /// 3. Distributes the Welcome message to the new member
    /// 4. Distributes the Commit message to existing members
    /// 5. Creates and broadcasts the AddMember CRDT operation
    pub async fn add_member_with_mls(
        &self,
        space_id: SpaceId,
        user_id: UserId,
        role: Role,
    ) -> Result<CrdtOp> {
        // Step 1: Fetch the user's KeyPackage from DHT
        println!("🔑 Fetching KeyPackage for user {} from DHT...", user_id);
        let key_package_bundle = self.fetch_key_package_from_dht(&user_id).await?;
        
        // Step 2: Deserialize the KeyPackage
        let provider = self.mls_provider.read().await;
        let key_package = crate::mls::KeyPackageStore::deserialize_key_package(
            &key_package_bundle,
            &provider
        )?;
        
        // Step 3: Add member to MLS group and get messages to distribute
        let mut manager = self.space_manager.write().await;
        let (commit_msg, welcome_msg) = manager.add_member_with_mls(
            &space_id,
            user_id,
            role,
            key_package,
            &self.user_id,
            &provider,
        )?;
        
        // Step 4: Create CRDT operation
        let op = manager.add_member(
            space_id,
            user_id,
            role,
            self.user_id,
            &self.keypair,
        )?;
        drop(manager);
        
        // Step 5: Store operation
        self.store.put_op(&op)?;
        
        // Step 6: Broadcast the CRDT operation
        self.broadcast_op(&op).await?;
        
        // Step 7: Distribute MLS messages via GossipSub
        let space_topic = format!("space/{}/mls", hex::encode(&space_id.0[..8]));
        
        // Convert MLS messages to bytes - OpenMLS MlsMessageOut has to_bytes() method
        let commit_bytes = commit_msg.to_bytes()
            .map_err(|e| crate::Error::Serialization(format!("Failed to serialize Commit: {:?}", e)))?;
        let mut network = self.network.write().await;
        network.publish(&space_topic, commit_bytes).await?;
        println!("✓ Sent Commit message to existing members");
        
        // Serialize and send Welcome to new member (via direct topic)
        let welcome_topic = format!("user/{}/welcome", hex::encode(&user_id.0[..8]));
        let welcome_bytes = welcome_msg.to_bytes()
            .map_err(|e| crate::Error::Serialization(format!("Failed to serialize Welcome: {:?}", e)))?;
        network.publish(&welcome_topic, welcome_bytes).await?;
        println!("✓ Sent Welcome message to new member");
        
        drop(network);
        
        println!("✅ Successfully added member {} to Space with MLS", user_id);
        
        Ok(op)
    }
    
    /// Remove a member from a Space (kick)
    /// 
    /// This removes the member from the Space and triggers MLS key rotation
    /// so the kicked member can no longer decrypt new messages.
    pub async fn remove_member(
        &self,
        space_id: SpaceId,
        user_id: UserId,
    ) -> Result<CrdtOp> {
        let mut manager = self.space_manager.write().await;
        let provider = self.mls_provider.read().await;
        let op = manager.remove_member(
            space_id,
            user_id,
            self.user_id,
            &self.keypair,
            &provider,
        )?;
        drop(provider);
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// List all members of a Space
    pub async fn list_members(&self, space_id: &SpaceId) -> Vec<(UserId, Role)> {
        let manager = self.space_manager.read().await;
        if let Some(space) = manager.get_space(space_id) {
            space.members.iter().map(|(uid, role)| (*uid, *role)).collect()
        } else {
            vec![]
        }
    }
    
    /// Create a Channel in a Space
    pub async fn create_channel(
        &self,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
    ) -> Result<(Channel, CrdtOp)> {
        let channel_id = ChannelId::from_content(&space_id, &name, &self.user_id);
        
        // Get current epoch from Space
        let epoch = {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            space.epoch
        };
        
        let mut manager = self.channel_manager.write().await;
        let op = manager.create_channel(
            channel_id,
            space_id,
            name,
            description,
            self.user_id,
            &self.keypair,
            epoch,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        let channel = manager.get_channel(&channel_id)
            .ok_or_else(|| Error::NotFound(format!("Channel {:?} not found", channel_id)))?
            .clone();
        
        Ok((channel, op))
    }
    
    /// Get a Channel by ID
    pub async fn get_channel(&self, channel_id: &ChannelId) -> Option<Channel> {
        let manager = self.channel_manager.read().await;
        manager.get_channel(channel_id).cloned()
    }
    
    /// List Channels in a Space
    pub async fn list_channels(&self, space_id: &SpaceId) -> Vec<Channel> {
        let manager = self.channel_manager.read().await;
        manager.list_channels(space_id).into_iter().cloned().collect()
    }
    
    /// Create a Thread in a Channel
    pub async fn create_thread(
        &self,
        space_id: SpaceId,
        channel_id: ChannelId,
        title: Option<String>,
        first_message: String,
    ) -> Result<(Thread, CrdtOp)> {
        // Hash the first message content
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(first_message.as_bytes());
        let content_hash_array: [u8; 32] = hasher.finalize().into();
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let thread_id = ThreadId::from_content(
            &channel_id,
            &self.user_id,
            &content_hash_array,
            timestamp,
        );
        
        // Get current epoch from Space
        let epoch = {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            space.epoch
        };
        
        let mut manager = self.thread_manager.write().await;
        let op = manager.create_thread(
            thread_id,
            space_id,
            channel_id,
            title,
            first_message,
            self.user_id,
            &self.keypair,
            epoch,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        let thread = manager.get_thread(&thread_id)
            .ok_or_else(|| Error::NotFound(format!("Thread {:?} not found", thread_id)))?
            .clone();
        
        Ok((thread, op))
    }
    
    /// Get a Thread by ID
    pub async fn get_thread(&self, thread_id: &ThreadId) -> Option<Thread> {
        let manager = self.thread_manager.read().await;
        manager.get_thread(thread_id).cloned()
    }
    
    /// List Threads in a Channel
    pub async fn list_threads(&self, channel_id: &ChannelId) -> Vec<Thread> {
        let manager = self.thread_manager.read().await;
        manager.list_threads(channel_id).into_iter().cloned().collect()
    }
    
    /// Post a Message to a Thread
    pub async fn post_message(
        &self,
        space_id: SpaceId,
        thread_id: ThreadId,
        content: String,
    ) -> Result<(Message, CrdtOp)> {
        // Hash the message content
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash_array: [u8; 32] = hasher.finalize().into();
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let message_id = MessageId::from_content(
            &self.user_id,
            &thread_id,
            &content_hash_array,
            timestamp,
            None, // No parent ID for top-level message
        );
        
        // Get current epoch from Space
        let epoch = {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            space.epoch
        };
        
        let mut manager = self.thread_manager.write().await;
        let op = manager.post_message(
            message_id,
            thread_id,
            content,
            self.user_id,
            &self.keypair,
            epoch,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        let message = manager.get_message(&message_id)
            .ok_or_else(|| Error::NotFound(format!("Message {:?} not found", message_id)))?
            .clone();
        
        Ok((message, op))
    }
    
    /// Edit a Message
    pub async fn edit_message(
        &self,
        space_id: SpaceId,
        message_id: MessageId,
        new_content: String,
    ) -> Result<CrdtOp> {
        // Get current epoch from Space
        let epoch = {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            space.epoch
        };
        
        let mut manager = self.thread_manager.write().await;
        let op = manager.edit_message(
            message_id,
            new_content,
            self.user_id,
            &self.keypair,
            epoch,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        Ok(op)
    }
    
    /// Get a Message by ID
    pub async fn get_message(&self, message_id: &MessageId) -> Option<Message> {
        let manager = self.thread_manager.read().await;
        manager.get_message(message_id).cloned()
    }
    
    /// List Messages in a Thread
    pub async fn list_messages(&self, thread_id: &ThreadId) -> Vec<Message> {
        let manager = self.thread_manager.read().await;
        manager.list_messages(thread_id).into_iter().cloned().collect()
    }
    
    /// Store a blob (attachment, media)
    /// 
    /// Encrypts the data using a key derived from the user's keypair and returns
    /// the content-addressed hash along with metadata.
    /// 
    /// Optionally uploads to DHT for offline availability if space_id is provided.
    pub async fn store_blob(
        &self,
        data: &[u8],
        mime_type: Option<String>,
        filename: Option<String>,
    ) -> Result<crate::storage::indices::BlobMetadata> {
        // Derive encryption key from user's keypair
        // For user-specific blobs (attachments), we use a user-derived key
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"descord-user-blob-key-v1");
        hasher.update(&self.user_id.0);
        let key_bytes: [u8; 32] = hasher.finalize().into();
        
        // Store encrypted blob
        let hash = self.storage.store_blob(data, &key_bytes)?;
        
        // Create metadata
        let metadata = crate::storage::indices::BlobMetadata {
            hash,
            size: data.len() as u64,
            mime_type,
            filename,
            uploaded_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            uploader: self.user_id,
            thread_id: None, // User-uploaded blobs not tied to a thread
        };
        
        // Store metadata in index
        self.storage.store_blob_metadata(&hash, &metadata)?;
        
        tracing::info!(
            hash = %hash.to_hex(),
            size = data.len(),
            "Stored blob"
        );
        
        Ok(metadata)
    }
    
    /// Store a blob with DHT replication for a specific Space
    /// 
    /// This is used for Space-related content (messages, attachments) that should
    /// be available even when the uploader is offline.
    pub async fn store_blob_for_space(
        &self,
        space_id: &SpaceId,
        data: &[u8],
        mime_type: Option<String>,
        filename: Option<String>,
    ) -> Result<crate::storage::indices::BlobMetadata> {
        // Store locally first
        let metadata = self.store_blob(data, mime_type, filename).await?;
        
        // Derive encryption key for local blob
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"descord-user-blob-key-v1");
        hasher.update(&self.user_id.0);
        let key_bytes: [u8; 32] = hasher.finalize().into();
        
        // Load the locally-encrypted blob
        let blob_path = self.storage.blob_dir().join(metadata.hash.to_hex());
        let blob_bytes = std::fs::read(&blob_path)
            .context("Failed to read blob for DHT upload")?;
        let local_blob = crate::storage::blob::EncryptedBlob::from_bytes(&blob_bytes)?;
        
        // Upload to DHT (non-blocking, best effort)
        let result = self.dht_put_blob(space_id, &metadata.hash, &local_blob).await;
        if let Err(e) = result {
            // Don't fail if DHT upload fails (degraded mode)
            eprintln!("⚠ Failed to upload blob to DHT: {}", e);
        } else {
            tracing::info!(
                hash = %metadata.hash.to_hex(),
                space_id = %hex::encode(&space_id.0[..8]),
                "Uploaded blob to DHT"
            );
        }
        
        Ok(metadata)
    }
    
    /// Retrieve a blob by hash
    /// 
    /// Decrypts and returns the blob data. Verifies content integrity.
    pub async fn retrieve_blob(&self, hash: &crate::storage::BlobHash) -> Result<Vec<u8>> {
        // Derive the same encryption key
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"descord-user-blob-key-v1");
        hasher.update(&self.user_id.0);
        let key_bytes: [u8; 32] = hasher.finalize().into();
        
        // Try local storage first
        match self.storage.load_blob(hash, &key_bytes) {
            Ok(plaintext) => {
                tracing::debug!(
                    hash = %hash.to_hex(),
                    size = plaintext.len(),
                    "Retrieved blob from local storage"
                );
                Ok(plaintext.to_vec())
            }
            Err(_) => {
                // Not found locally - this is expected for user blobs only
                // For Space blobs, use retrieve_blob_for_space instead
                Err(Error::NotFound(format!("Blob {} not found", hash.to_hex())))
            }
        }
    }
    
    /// Retrieve a blob by hash with DHT fallback for a specific Space
    /// 
    /// Tries local storage first, then falls back to DHT if the blob is not available.
    pub async fn retrieve_blob_for_space(
        &self,
        space_id: &SpaceId,
        hash: &crate::storage::BlobHash,
    ) -> Result<Vec<u8>> {
        // Derive encryption key
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"descord-user-blob-key-v1");
        hasher.update(&self.user_id.0);
        let key_bytes: [u8; 32] = hasher.finalize().into();
        
        // Try local storage first
        match self.storage.load_blob(hash, &key_bytes) {
            Ok(plaintext) => {
                tracing::debug!(
                    hash = %hash.to_hex(),
                    "Retrieved blob from local storage"
                );
                Ok(plaintext.to_vec())
            }
            Err(_) => {
                // Not found locally - try DHT
                tracing::info!(
                    hash = %hash.to_hex(),
                    space_id = %hex::encode(&space_id.0[..8]),
                    "Blob not found locally, fetching from DHT"
                );
                
                match self.dht_get_blob(space_id, hash).await {
                    Ok(local_blob) => {
                        // Got it from DHT! Decrypt and store locally
                        let plaintext = local_blob.decrypt(&key_bytes)?;
                        
                        // Store locally for future access
                        let blob_bytes = local_blob.to_bytes()?;
                        let blob_path = self.storage.blob_dir().join(hash.to_hex());
                        std::fs::write(&blob_path, &blob_bytes)
                            .context("Failed to cache blob from DHT")?;
                        
                        tracing::info!(
                            hash = %hash.to_hex(),
                            "Retrieved blob from DHT and cached locally"
                        );
                        
                        Ok(plaintext.to_vec())
                    }
                    Err(e) => {
                        Err(Error::NotFound(format!(
                            "Blob {} not found locally or in DHT: {}",
                            hash.to_hex(),
                            e
                        )))
                    }
                }
            }
        }
    }
    
    /// Broadcast a CRDT operation to the network
    async fn broadcast_op(&self, op: &CrdtOp) -> Result<()> {
        let topic = format!("space/{}", ::hex::encode(&op.space_id.0[..8]));
        
        println!("📢 Broadcasting operation on topic: {}", topic);
        
        // Broadcast via GossipSub
        self.broadcast_op_on_topic(op, &topic).await?;
        
        // Store in DHT for offline sync
        // Note: We store each operation individually for now
        // TODO: Batch operations for efficiency
        let result = self.dht_put_operations(&op.space_id, vec![op.clone()]).await;
        if let Err(e) = result {
            // Don't fail if DHT storage fails (degraded mode)
            eprintln!("⚠ Failed to store operation in DHT: {}", e);
        }
        
        Ok(())
    }
    
    /// Broadcast a CRDT operation to a specific topic
    async fn broadcast_op_on_topic(&self, op: &CrdtOp, topic: &str) -> Result<()> {
        // Serialize the operation
        let op_bytes = minicbor::to_vec(op)
            .map_err(|e| Error::Serialization(format!("Failed to encode operation: {}", e)))?;
        
        // Check if this Space has an MLS group - if so, encrypt the operation
        let data = {
            let mut space_manager = self.space_manager.write().await;
            if let Some(mls_group) = space_manager.get_mls_group_mut(&op.space_id) {
                // Encrypt the operation as MLS application data
                let provider = self.mls_provider.read().await;
                let encrypted_msg = mls_group.encrypt_application_message(&op_bytes, &provider)?;
                drop(provider);
                
                // Serialize the encrypted MLS message
                let encrypted_bytes = encrypted_msg.to_bytes()
                    .map_err(|e| Error::Serialization(format!("Failed to serialize MLS message: {}", e)))?;
                
                // Format: [0x01][space_id (32 bytes)][encrypted_data]
                // The space_id is needed for decryption on the receive side
                let mut data = vec![0x01];
                data.extend_from_slice(&op.space_id.0);
                data.extend_from_slice(&encrypted_bytes);
                data
            } else {
                // No MLS group - send plaintext with marker (0x00)
                let mut data = vec![0x00];
                data.extend_from_slice(&op_bytes);
                data
            }
        };
        
        let mut network = self.network.write().await;
        
        // Attempt to publish, but don't fail if no peers are connected
        // This is expected in single-node scenarios and tests
        let result = network.publish(topic, data).await;
        
        // Record metrics
        if result.is_ok() {
            self.gossip_metrics.record_publish(topic).await;
        }
        
        result.or(Ok(()))
    }
    
    /// Broadcast raw data on a topic (for sync requests, etc.)
    async fn broadcast_raw(&self, topic: &str, data: Vec<u8>) -> Result<()> {
        let mut network = self.network.write().await;
        network.publish(topic, data).await
    }
    
    /// Handle a sync request from a peer by re-broadcasting all Space operations
    /// Subscribe to a Space's operation stream
    pub async fn subscribe_to_space(&self, space_id: &SpaceId) -> Result<()> {
        let topic = format!("space/{}", ::hex::encode(&space_id.0[..8]));
        println!("🔔 Subscribing to topic: {}", topic);
        let mut network = self.network.write().await;
        network.subscribe(&topic).await?;
        println!("✓ Subscribed to topic: {}", topic);
        
        Ok(())
    }
    
    /// Get network peer ID
    pub async fn peer_id(&self) -> libp2p::PeerId {
        let network = self.network.read().await;
        *network.local_peer_id()
    }
    
    /// Get listening addresses
    pub async fn listening_addrs(&self) -> Vec<libp2p::Multiaddr> {
        let network = self.network.read().await;
        network.listeners().await
    }
    
    /// Dial a peer directly
    pub async fn dial(&self, addr: libp2p::Multiaddr) -> Result<()> {
        let mut network = self.network.write().await;
        network.dial(addr).await
    }
    
    /// Process incoming network events
    pub async fn process_events(&self) -> Result<()> {
        let mut rx = self.network_rx.write().await;
        
        while let Some(event) = rx.recv().await {
            match event {
                NetworkEvent::MessageReceived { topic: _, data, source: _ } => {
                    // Decode CRDT operation
                    if let Ok(op) = minicbor::decode::<CrdtOp>(&data) {
                        self.handle_incoming_op(op).await?;
                    }
                }
                NetworkEvent::PeerConnected(peer_id) => {
                    println!("Peer connected: {}", peer_id);
                }
                NetworkEvent::PeerDisconnected(peer_id) => {
                    println!("Peer disconnected: {}", peer_id);
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// Handle an incoming CRDT operation
    pub async fn handle_incoming_op(&self, op: CrdtOp) -> Result<()> {
        // Store the operation
        self.store.put_op(&op)?;
        
        // Process based on operation type
        match &op.op_type {
            crate::crdt::OpType::CreateSpace(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_create_space(&op)?;
            }
            crate::crdt::OpType::UpdateSpaceVisibility(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_update_space_visibility(&op)?;
            }
            crate::crdt::OpType::CreateInvite(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_create_invite(&op)?;
            }
            crate::crdt::OpType::RevokeInvite(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_revoke_invite(&op)?;
            }
            crate::crdt::OpType::UseInvite(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_use_invite(&op)?;
            }
            crate::crdt::OpType::RemoveMember(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_remove_member(&op)?;
            }
            crate::crdt::OpType::CreateChannel(_) => {
                let mut manager = self.channel_manager.write().await;
                manager.process_create_channel(&op)?;
            }
            crate::crdt::OpType::CreateThread(_) => {
                let mut manager = self.thread_manager.write().await;
                manager.process_create_thread(&op)?;
            }
            crate::crdt::OpType::PostMessage(_) => {
                let mut manager = self.thread_manager.write().await;
                manager.process_post_message(&op)?;
            }
            crate::crdt::OpType::EditMessage(_) => {
                let mut manager = self.thread_manager.write().await;
                manager.process_edit_message(&op)?;
            }
            _ => {
                // Other operations can be added as needed
            }
        }
        
        Ok(())
    }
    
    /// Apply a remote operation (for testing and manual operation sync)
    pub async fn apply_remote_op(&self, op: &CrdtOp) -> Result<()> {
        self.handle_incoming_op(op.clone()).await
    }
    
    /// Get the network peer ID
    pub async fn network_peer_id(&self) -> String {
        let network = self.network.read().await;
        network.local_peer_id().to_string()
    }
    
    /// Get the network listening addresses
    pub async fn network_listeners(&self) -> Vec<String> {
        let network = self.network.read().await;
        network.listeners().await.iter().map(|a| a.to_string()).collect()
    }
    
    /// Dial a peer address
    pub async fn network_dial(&self, addr: &str) -> Result<()> {
        let multiaddr = addr.parse()
            .map_err(|e| Error::Network(format!("Invalid address {}: {}", addr, e)))?;
        let mut network = self.network.write().await;
        network.dial(multiaddr).await
    }
    
    /// Discover available relay servers from DHT
    pub async fn discover_relays(&self) -> Result<Vec<crate::network::relay::RelayInfo>> {
        let mut network = self.network.write().await;
        network.discover_relays().await
    }
    
    /// Connect to a relay server and reserve a relay slot
    pub async fn connect_to_relay(&self, relay_addr: &str) -> Result<()> {
        let multiaddr = relay_addr.parse()
            .map_err(|e| Error::Network(format!("Invalid relay address {}: {}", relay_addr, e)))?;
        let mut network = self.network.write().await;
        network.dial(multiaddr).await
    }
    
    /// Dial a peer through a relay (for IP privacy)
    /// 
    /// This establishes a connection via circuit relay, hiding both peers' IP addresses
    pub async fn dial_peer_via_relay(
        &self,
        relay_addr: &str,
        relay_peer_id: &str,
        target_peer_id: &str,
    ) -> Result<()> {
        let relay_multiaddr = relay_addr.parse()
            .map_err(|e| Error::Network(format!("Invalid relay address: {}", e)))?;
        
        let relay_id = relay_peer_id.parse()
            .map_err(|e| Error::Network(format!("Invalid relay peer ID: {}", e)))?;
        
        let target_id = target_peer_id.parse()
            .map_err(|e| Error::Network(format!("Invalid target peer ID: {}", e)))?;
        
        let mut network = self.network.write().await;
        network.dial_via_relay(relay_multiaddr, relay_id, target_id).await
    }
    
    /// Get relay-only addresses (circuit relay addresses, no direct IP)
    /// 
    /// Returns only /p2p-circuit addresses for privacy
    pub async fn relay_addresses(&self) -> Vec<String> {
        let network = self.network.read().await;
        let peer_id = network.local_peer_id();
        
        // Return p2p-circuit address format
        // Format: /p2p/{relay_peer_id}/p2p-circuit/p2p/{our_peer_id}
        vec![format!("/p2p-circuit/p2p/{}", peer_id)]
    }
    
    /// Auto-discover and connect to best available relay
    /// 
    /// Discovers relays from DHT and connects to the one with best reputation
    pub async fn auto_connect_relay(&self) -> Result<crate::network::relay::RelayInfo> {
        // Discover relays from DHT
        let relays = self.discover_relays().await?;
        
        if relays.is_empty() {
            return Err(Error::Network("No relays discovered".to_string()));
        }
        
        // Sort by reputation (highest first)
        let mut sorted_relays = relays;
        sorted_relays.sort_by(|a, b| {
            b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Connect to best relay
        let best_relay = &sorted_relays[0];
        
        // Pick first available address
        if let Some(addr) = best_relay.addresses.first() {
            let addr_str = addr.to_string();
            self.connect_to_relay(&addr_str).await?;
            println!("✓ Connected to relay: {} (reputation: {:.2})", 
                best_relay.peer_id, best_relay.reputation);
            
            // Store current relay
            *self.current_relay.write().await = Some(best_relay.clone());
            
            Ok(best_relay.clone())
        } else {
            Err(Error::Network("Best relay has no addresses".to_string()))
        }
    }
    
    /// Start automatic relay rotation
    /// 
    /// Periodically switches to a new relay for privacy
    /// - rotation_interval: How often to rotate relays (e.g., Duration::from_secs(300) for 5 minutes)
    pub async fn start_relay_rotation(&self, rotation_interval: Duration) -> Result<()> {
        // Stop any existing rotation task
        self.stop_relay_rotation().await;
        
        let client_clone = Arc::new(self.clone_for_rotation());
        let rotation_interval_clone = rotation_interval;
        
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(rotation_interval_clone);
            interval.tick().await; // Skip first immediate tick
            
            loop {
                interval.tick().await;
                
                println!("🔄 Relay rotation triggered");
                
                // Discover available relays
                match client_clone.discover_relays().await {
                    Ok(relays) if !relays.is_empty() => {
                        // Filter out current relay
                        let current_peer_id = {
                            let current = client_clone.current_relay.read().await;
                            current.as_ref().map(|r| r.peer_id.to_string())
                        };
                        
                        let mut available_relays: Vec<_> = relays.into_iter()
                            .filter(|r| Some(r.peer_id.to_string()) != current_peer_id)
                            .collect();
                        
                        if available_relays.is_empty() {
                            println!("⚠️ No alternative relays available for rotation");
                            continue;
                        }
                        
                        // Sort by reputation
                        available_relays.sort_by(|a, b| {
                            b.reputation.partial_cmp(&a.reputation).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        
                        // Connect to new best relay
                        let new_relay = &available_relays[0];
                        if let Some(addr) = new_relay.addresses.first() {
                            let addr_str = addr.to_string();
                            match client_clone.connect_to_relay(&addr_str).await {
                                Ok(_) => {
                                    println!("✓ Rotated to relay: {} (reputation: {:.2})", 
                                        new_relay.peer_id, new_relay.reputation);
                                    
                                    // Update current relay
                                    *client_clone.current_relay.write().await = Some(new_relay.clone());
                                }
                                Err(e) => {
                                    println!("❌ Relay rotation failed: {}", e);
                                }
                            }
                        }
                    }
                    Ok(_) => {
                        println!("⚠️ No relays discovered during rotation");
                    }
                    Err(e) => {
                        println!("❌ Relay discovery failed during rotation: {}", e);
                    }
                }
            }
        });
        
        *self.rotation_task.write().await = Some(task);
        println!("🔄 Relay rotation started (interval: {:?})", rotation_interval);
        
        Ok(())
    }
    
    /// Stop automatic relay rotation
    pub async fn stop_relay_rotation(&self) {
        let mut task = self.rotation_task.write().await;
        if let Some(handle) = task.take() {
            handle.abort();
            println!("🛑 Relay rotation stopped");
        }
    }
    
    /// Get current relay information
    pub async fn current_relay(&self) -> Option<crate::network::relay::RelayInfo> {
        self.current_relay.read().await.clone()
    }
    
    /// Get GossipSub metrics
    pub fn gossip_metrics(&self) -> Arc<crate::network::GossipMetrics> {
        Arc::clone(&self.gossip_metrics)
    }
    
    /// Print GossipSub metrics summary
    pub async fn print_gossip_metrics(&self) {
        self.gossip_metrics.print_summary().await;
    }
    
    /// Helper to clone necessary fields for rotation task
    fn clone_for_rotation(&self) -> ClientForRotation {
        ClientForRotation {
            network: Arc::clone(&self.network),
            current_relay: Arc::clone(&self.current_relay),
        }
    }
    
    // ===== DHT-BASED PEER DISCOVERY =====
    
    /// Advertise this peer's presence in a space via DHT
    /// 
    /// Publishes our relay address to DHT so other space members can find us
    /// Key format: /descord/space/{space_id}/peers
    /// Value: JSON with peer_id and relay_address (no IP exposed)
    pub async fn advertise_space_presence(&self, space_id: SpaceId) -> Result<()> {
        let relay_addrs = self.relay_addresses().await;
        if relay_addrs.is_empty() {
            return Err(Error::Network("No relay address available for advertisement".to_string()));
        }
        
        let peer_id = self.network_peer_id().await;
        
        // Create DHT key for this space
        let space_key = format!("/descord/space/{}/peers", hex::encode(&space_id.0));
        
        // Create advertisement value (peer_id + relay address)
        let advertisement = serde_json::json!({
            "peer_id": peer_id,
            "relay_address": relay_addrs[0],
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        let value_bytes = serde_json::to_vec(&advertisement)
            .map_err(|e| Error::Network(format!("Failed to serialize advertisement: {}", e)))?;
        
        // Publish to DHT
        let mut network = self.network.write().await;
        network.dht_put(space_key.as_bytes().to_vec(), value_bytes).await?;
        
        println!("📢 Advertised presence in space {} via DHT", hex::encode(&space_id.0[..8]));
        Ok(())
    }
    
    /// Discover peers in a space via DHT
    /// 
    /// Queries DHT for other peers advertising themselves in this space
    /// Returns list of (peer_id, relay_address) tuples
    pub async fn discover_space_peers(&self, space_id: SpaceId) -> Result<Vec<SpacePeerInfo>> {
        let space_key = format!("/descord/space/{}/peers", hex::encode(&space_id.0));
        
        let mut network = self.network.write().await;
        let results = network.dht_get(space_key.as_bytes().to_vec()).await?;
        
        let mut peers = Vec::new();
        for value_bytes in results {
            if let Ok(advertisement) = serde_json::from_slice::<serde_json::Value>(&value_bytes) {
                if let (Some(peer_id), Some(relay_addr)) = (
                    advertisement["peer_id"].as_str(),
                    advertisement["relay_address"].as_str(),
                ) {
                    // Skip ourselves
                    if peer_id != self.network_peer_id().await {
                        peers.push(SpacePeerInfo {
                            peer_id: peer_id.to_string(),
                            relay_address: relay_addr.to_string(),
                        });
                    }
                }
            }
        }
        
        println!("🔍 Discovered {} peers in space {}", peers.len(), hex::encode(&space_id.0[..8]));
        Ok(peers)
    }
    
    /// Connect to all discovered peers in a space via relay
    /// 
    /// Discovers peers via DHT and dials them through relay servers
    /// This enables automatic mesh network formation without any central coordination
    pub async fn connect_to_space_peers(&self, space_id: SpaceId) -> Result<usize> {
        let peers = self.discover_space_peers(space_id).await?;
        
        if peers.is_empty() {
            println!("ℹ️ No peers found in space {}", hex::encode(&space_id.0[..8]));
            return Ok(0);
        }
        
        let mut connected = 0;
        for peer in &peers {
            println!("📞 Dialing peer {} via relay...", &peer.peer_id[..16]);
            
            // Parse relay address to extract relay peer ID
            // Format: /ip4/x.x.x.x/tcp/xxxx/p2p/{relay_id}/p2p-circuit/p2p/{peer_id}
            if let Some(relay_id_start) = peer.relay_address.find("/p2p/") {
                let after_relay = &peer.relay_address[relay_id_start + 5..];
                if let Some(relay_id_end) = after_relay.find("/p2p-circuit") {
                    let relay_id = &after_relay[..relay_id_end];
                    
                    // Extract base relay address (without /p2p-circuit/p2p/{peer_id})
                    let relay_addr = &peer.relay_address[..relay_id_start + 5 + relay_id_end];
                    
                    match self.dial_peer_via_relay(relay_addr, relay_id, &peer.peer_id).await {
                        Ok(_) => {
                            println!("✓ Connected to peer {} via relay", &peer.peer_id[..16]);
                            connected += 1;
                        }
                        Err(e) => {
                            println!("⚠️ Failed to connect to peer {}: {}", &peer.peer_id[..16], e);
                        }
                    }
                } else {
                    println!("⚠️ Invalid relay address format for peer {}", &peer.peer_id[..16]);
                }
            } else {
                println!("⚠️ Cannot parse relay address for peer {}", &peer.peer_id[..16]);
            }
        }
        
        println!("🌐 Connected to {}/{} peers in space", connected, peers.len());
        Ok(connected)
    }
}

/// Minimal client clone for rotation background task
struct ClientForRotation {
    network: Arc<RwLock<NetworkNode>>,
    current_relay: Arc<RwLock<Option<crate::network::relay::RelayInfo>>>,
}

impl ClientForRotation {
    async fn discover_relays(&self) -> Result<Vec<crate::network::relay::RelayInfo>> {
        let mut network = self.network.write().await;
        network.discover_relays().await
    }
    
    async fn connect_to_relay(&self, relay_addr: &str) -> Result<()> {
        let multiaddr = relay_addr.parse()
            .map_err(|e| Error::Network(format!("Invalid relay address {}: {}", relay_addr, e)))?;
        let mut network = self.network.write().await;
        network.dial(multiaddr).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_create_client() {
        let keypair = Keypair::generate();
        let temp_dir = TempDir::new().unwrap();
        
        let config = ClientConfig {
            storage_path: temp_dir.path().to_path_buf(),
            listen_addrs: vec![],
            bootstrap_peers: vec![],
        };
        
        let client = Client::new(keypair, config);
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_space() {
        let keypair = Keypair::generate();
        let temp_dir = TempDir::new().unwrap();
        
        let config = ClientConfig {
            storage_path: temp_dir.path().to_path_buf(),
            listen_addrs: vec![],
            bootstrap_peers: vec![],
        };
        
        let client = Client::new(keypair, config).unwrap();
        
        let (space, _, _privacy_info) = client.create_space(
            "Test Space".to_string(),
            Some("A test space".to_string()),
        ).await.unwrap();
        
        assert_eq!(space.name, "Test Space");
        assert_eq!(space.owner, client.user_id());
    }
    
    #[tokio::test]
    async fn test_create_channel() {
        let keypair = Keypair::generate();
        let temp_dir = TempDir::new().unwrap();
        
        let config = ClientConfig {
            storage_path: temp_dir.path().to_path_buf(),
            listen_addrs: vec![],
            bootstrap_peers: vec![],
        };
        
        let client = Client::new(keypair, config).unwrap();
        
        let (space, _, _privacy_info) = client.create_space("Test Space".to_string(), None).await.unwrap();
        
        let (channel, _) = client.create_channel(
            space.id,
            "general".to_string(),
            Some("General discussion".to_string()),
        ).await.unwrap();
        
        assert_eq!(channel.name, "general");
        assert_eq!(channel.space_id, space.id);
    }
    
    #[tokio::test]
    async fn test_create_thread_and_post_message() {
        let keypair = Keypair::generate();
        let temp_dir = TempDir::new().unwrap();
        
        let config = ClientConfig {
            storage_path: temp_dir.path().to_path_buf(),
            listen_addrs: vec![],
            bootstrap_peers: vec![],
        };
        
        let client = Client::new(keypair, config).unwrap();
        
        let (space, _, _privacy_info) = client.create_space("Test Space".to_string(), None).await.unwrap();
        let (channel, _) = client.create_channel(space.id, "general".to_string(), None).await.unwrap();
        
        let (thread, _) = client.create_thread(
            space.id,
            channel.id,
            Some("Discussion".to_string()),
            "First message".to_string(),
        ).await.unwrap();
        
        assert_eq!(thread.title, Some("Discussion".to_string()));
        
        let messages = client.list_messages(&thread.id).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "First message");
    }
    
    #[tokio::test]
    async fn test_blob_storage() {
        let keypair = Keypair::generate();
        let temp_dir = TempDir::new().unwrap();
        
        let config = ClientConfig {
            storage_path: temp_dir.path().to_path_buf(),
            listen_addrs: vec![],
            bootstrap_peers: vec![],
        };
        
        let client = Client::new(keypair, config).unwrap();
        
        // Store a blob
        let data = b"Test attachment data";
        let metadata = client.store_blob(
            data,
            Some("text/plain".to_string()),
            Some("test.txt".to_string()),
        ).await.unwrap();
        
        assert_eq!(metadata.size, data.len() as u64);
        assert_eq!(metadata.mime_type, Some("text/plain".to_string()));
        assert_eq!(metadata.filename, Some("test.txt".to_string()));
        assert_eq!(metadata.uploader, client.user_id);
        
        // Retrieve the blob
        let retrieved = client.retrieve_blob(&metadata.hash).await.unwrap();
        assert_eq!(&retrieved[..], &data[..]);
    }
}
