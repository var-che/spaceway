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
use std::collections::VecDeque;

/// Queued MLS message that failed to decrypt (e.g., due to epoch mismatch)
#[derive(Debug, Clone)]
struct PendingMlsMessage {
    /// Space ID this message belongs to
    space_id: SpaceId,
    /// Encrypted message data
    encrypted_data: Vec<u8>,
    /// Topic it was received on
    topic: String,
    /// When it was first queued
    queued_at: Instant,
}

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
    
    /// Queue for MLS messages that failed to decrypt (waiting for epoch update)
    pending_mls_messages: Arc<RwLock<VecDeque<PendingMlsMessage>>>,
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
        let mls_signer = Arc::new(mls_signer); // Wrap in Arc for sharing
        
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
            pending_mls_messages: Arc::new(RwLock::new(VecDeque::new())),
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
        let keypackage_store = Arc::clone(&self.keypackage_store); // Clone for Welcome processing
        let pending_mls_messages = Arc::clone(&self.pending_mls_messages); // Clone for queued message processing
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
                                
                                // Get the signer from our KeyPackageStore
                                // This is the SAME signer used when generating KeyPackages
                                // Critical: must use the same keypair that Alice expects!
                                let kp_store = keypackage_store.read().await;
                                let signer_arc = kp_store.signer();
                                drop(kp_store);
                                
                                // Use the shared provider that has the KeyPackage private keys
                                let provider = mls_provider.read().await;
                                match crate::mls::MlsGroup::from_welcome(
                                    data.clone(),
                                    user_id,
                                    signer_arc,  // Pass the Arc directly
                                    &provider
                                ) {
                                Ok(mls_group) => {
                                    let epoch = mls_group.current_epoch().0;
                                    println!("  ✓ Successfully joined MLS group (epoch {})", epoch);
                                    
                                    // Wrap in Option so we can move it conditionally
                                    let mut mls_group_opt = Some(mls_group);
                                        
                                        // Find which space or channel this Welcome is for
                                        let mut found = false;
                                        
                                        // First check spaces without MLS groups
                                        {
                                            let space_mgr = space_manager.read().await;
                                            let mut spaces = space_mgr.list_spaces();
                                            
                                            for space in spaces.iter() {
                                                if space_mgr.get_mls_group(&space.id).is_none() {
                                                    // This must be the space for this Welcome!
                                                    let space_id = space.id;
                                                    let space_name = space.name.clone();
                                                    drop(space_mgr);
                                                    
                                                    let mut space_mgr_mut = space_manager.write().await;
                                                    space_mgr_mut.store_mls_group(space_id, mls_group_opt.take().unwrap());
                                                    drop(space_mgr_mut);
                                                    
                                                    println!("  ✓ MLS group stored for space {} ({})", 
                                                        space_name, hex::encode(&space_id.0[..8]));
                                                    println!("  ✓ Can now decrypt messages in this space!");
                                                    
                                                    // Process queued messages for this space
                                                    let mut pending_queue = pending_mls_messages.write().await;
                                                    let queue_len = pending_queue.len();
                                                    if queue_len > 0 {
                                                        println!("  📬 Processing {} queued messages...", queue_len);
                                                        
                                                        // Drain messages for this space and try to decrypt them
                                                        let mut remaining = VecDeque::new();
                                                        let mut processed = 0;
                                                        
                                                        while let Some(pending_msg) = pending_queue.pop_front() {
                                                            if pending_msg.space_id == space_id {
                                                                // Try to decrypt now that we have the updated epoch
                                                                let mut space_mgr_mut = space_manager.write().await;
                                                                let provider = mls_provider.read().await;
                                                                
                                                                if let Some(mls_group) = space_mgr_mut.get_mls_group_mut(&space_id) {
                                                                    match mls_group.decrypt_application_message(&pending_msg.encrypted_data, &provider) {
                                                                        Ok(decrypted_bytes) => {
                                                                            println!("    ✓ Decrypted queued message ({} bytes)", decrypted_bytes.len());
                                                                            processed += 1;
                                                                            
                                                                            // Decode and process the operation
                                                                            if let Ok(op) = minicbor::decode::<CrdtOp>(&decrypted_bytes) {
                                                                                // Store and process the operation (same logic as regular messages)
                                                                                if op.verify_signature() {
                                                                                    if let Err(e) = store.put_op(&op) {
                                                                                        eprintln!("      ⚠️ Failed to store queued operation: {}", e);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            eprintln!("    ⚠️ Still can't decrypt queued message: {}", e);
                                                                            // Re-queue if still can't decrypt
                                                                            remaining.push_back(pending_msg);
                                                                        }
                                                                    }
                                                                }
                                                                drop(provider);
                                                                drop(space_mgr_mut);
                                                            } else {
                                                                // Different space, keep in queue
                                                                remaining.push_back(pending_msg);
                                                            }
                                                        }
                                                        
                                                        // Put back messages we couldn't process
                                                        *pending_queue = remaining;
                                                        println!("    ✓ Processed {}/{} queued messages", processed, queue_len);
                                                    }
                                                    drop(pending_queue);
                                                    
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        
                                        // If not a space Welcome, check if it's a channel Welcome
                                        if !found {
                                            println!("  🔍 Not a space Welcome, checking channels...");
                                            let mut target_channel_id: Option<(ChannelId, String)> = None;
                                            
                                            {
                                                // Check all channels across all spaces
                                                let space_mgr = space_manager.read().await;
                                                let spaces = space_mgr.list_spaces();
                                                let channel_mgr = channel_manager.read().await;
                                                
                                                'outer: for space in spaces.iter() {
                                                    let channels = channel_mgr.list_channels(&space.id);
                                                    
                                                    // Look for any channel without an MLS group
                                                    for channel in channels.iter() {
                                                        if channel_mgr.get_mls_group(&channel.id).is_none() {
                                                            // This must be the channel for this Welcome!
                                                            target_channel_id = Some((channel.id, channel.name.clone()));
                                                            found = true;
                                                            break 'outer;
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            if let Some((channel_id, channel_name)) = target_channel_id {
                                                let mut channel_mgr_mut = channel_manager.write().await;
                                                channel_mgr_mut.store_mls_group(channel_id, mls_group_opt.take().unwrap());
                                                drop(channel_mgr_mut);
                                                
                                                println!("  ✅ MLS group stored for channel {} ({})", 
                                                    channel_name, hex::encode(&channel_id.0[..8]));
                                                println!("  ✅ Can now participate in this channel!");
                                            } else {
                                                println!("  ⚠️ Couldn't find space or channel for this MLS group");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to process Welcome message: {}", e);
                                    }
                                }
                                
                                continue; // Don't try to decode as CrdtOp
                            }
                            
                            // Check if this is a Commit message (MLS protocol message for epoch updates)
                            // Commit messages don't have the 0x01 marker - they're raw OpenMLS messages
                            // Try to detect Commit by attempting to deserialize as MlsMessageIn
                            let is_commit_message = if data.first() != Some(&0x01) {
                                use openmls::prelude::tls_codec::Deserialize;
                                if let Ok(mls_msg) = openmls::framing::MlsMessageIn::tls_deserialize(&mut &data[..]) {
                                    if let Ok(_) = mls_msg.try_into_protocol_message() {
                                        // This looks like an OpenMLS protocol message (possibly Commit)
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            
                            if is_commit_message {
                                println!("  🔄 MLS Commit message detected - processing epoch update...");
                                
                                // We need to find which space this Commit is for
                                // The Commit itself doesn't contain the space_id, but we can try all our spaces
                                
                                // First, collect space IDs to avoid borrow issues
                                let space_ids: Vec<SpaceId> = {
                                    let mut space_mgr = space_manager.write().await;
                                    space_mgr.mls_groups_mut().map(|(id, _)| *id).collect()
                                };
                                
                                let mut processed = false;
                                let mut processed_space_id: Option<SpaceId> = None;
                                
                                // Try to process with each MLS group we're in
                                for space_id in space_ids {
                                    let mut space_mgr = space_manager.write().await;
                                    let provider = mls_provider.read().await;
                                    
                                    if let Some(mls_group) = space_mgr.get_mls_group_mut(&space_id) {
                                        match mls_group.process_commit_message(&data, &provider) {
                                            Ok(()) => {
                                                println!("  ✓ Commit processed for space {}", hex::encode(&space_id.0[..8]));
                                                processed = true;
                                                processed_space_id = Some(space_id);
                                                drop(provider);
                                                drop(space_mgr);
                                                break;
                                            }
                                            Err(_) => {
                                                // Not for this group, try next
                                                drop(provider);
                                                drop(space_mgr);
                                                continue;
                                            }
                                        }
                                    }
                                }
                                
                                // If we processed a Commit, try to decrypt queued messages for that space
                                if let Some(space_id) = processed_space_id {
                                    println!("  📬 Checking for queued messages to process...");
                                    let queued: Vec<PendingMlsMessage> = {
                                        let mut pending_queue = pending_mls_messages.write().await;
                                        pending_queue.drain(..).collect()
                                    };
                                    
                                    if !queued.is_empty() {
                                        println!("  📬 Processing {} queued messages...", queued.len());
                                        
                                        for queued_msg in queued {
                                            if queued_msg.space_id == space_id {
                                                // Try to decrypt this queued message
                                                let mut space_mgr = space_manager.write().await;
                                                let provider = mls_provider.read().await;
                                                
                                                if let Some(mls_group) = space_mgr.get_mls_group_mut(&space_id) {
                                                    match mls_group.decrypt_application_message(&queued_msg.encrypted_data, &provider) {
                                                        Ok(plaintext) => {
                                                            println!("  ✓ Decrypted queued message ({} bytes)", plaintext.len());
                                                            
                                                            // Decode the CrdtOp from the decrypted plaintext
                                                            if let Ok(op) = bincode::deserialize::<CrdtOp>(&plaintext) {
                                                                drop(provider);
                                                                drop(space_mgr);
                                                                
                                                                // Process the operation
                                                                // TODO: Can't call self.handle_incoming_op from spawned task
                                                                // Need to send op to a channel for processing
                                                                eprintln!("  ✓ Queued operation decoded, but can't process in spawned task");
                                                            }
                                                        }
                                                        Err(e) => {
                                                            // Still can't decrypt - re-queue
                                                            eprintln!("  ⚠️ Still can't decrypt queued message: {}", e);
                                                            let mut pending_queue = pending_mls_messages.write().await;
                                                            pending_queue.push_back(queued_msg);
                                                            drop(pending_queue);
                                                        }
                                                    }
                                                } else {
                                                    // MLS group not found - re-queue
                                                    let mut pending_queue = pending_mls_messages.write().await;
                                                    pending_queue.push_back(queued_msg);
                                                    drop(pending_queue);
                                                }
                                            } else {
                                                // Not for this space - re-queue
                                                let mut pending_queue = pending_mls_messages.write().await;
                                                pending_queue.push_back(queued_msg);
                                                drop(pending_queue);
                                            }
                                        }
                                    }
                                }
                                
                                if !processed {
                                    eprintln!("  ⚠️ Could not process Commit (no matching MLS group)");
                                }
                                
                                continue; // Don't try to decode as CrdtOp
                            }
                            
                            // Check for MLS encryption marker and decode the operation
                            let op = if data.first() == Some(&0x01) {
                                // Space-level MLS encryption
                                println!("  🔒 Space MLS-encrypted message detected");
                                
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
                                                    println!("  ✓ Decrypted Space MLS message ({} bytes)", plaintext.len());
                                                    plaintext
                                                }
                                                Err(e) => {
                                                    let error_str = format!("{:?}", e);
                                                    if error_str.contains("WrongEpoch") {
                                                        // Epoch mismatch - queue for retry after Welcome
                                                        eprintln!("  ⏸️  Message from future epoch - queuing for retry");
                                                        let mut pending_queue = pending_mls_messages.write().await;
                                                        pending_queue.push_back(PendingMlsMessage {
                                                            space_id,
                                                            encrypted_data: encrypted_data.to_vec(),
                                                            topic: topic.clone(),
                                                            queued_at: Instant::now(),
                                                        });
                                                        eprintln!("     (Queued: {} pending messages)", pending_queue.len());
                                                        drop(pending_queue);
                                                        continue;
                                                    } else {
                                                        eprintln!("  ⚠️ Failed to decrypt MLS message: {}", e);
                                                        eprintln!("     (You may have been removed from this Space)");
                                                        continue;
                                                    }
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
                            } else if data.first() == Some(&0x02) {
                                // Channel-level MLS encryption
                                println!("  🔒 Channel MLS-encrypted message detected");
                                
                                // Message format: [0x02][channel_id (32 bytes)][encrypted_data]
                                if data.len() < 33 {
                                    eprintln!("  ⚠️ Channel MLS message too short (need at least 33 bytes)");
                                    continue;
                                }
                                
                                // Extract channel_id from message
                                let channel_id_bytes: [u8; 32] = match data[1..33].try_into() {
                                    Ok(bytes) => bytes,
                                    Err(_) => {
                                        eprintln!("  ⚠️ Invalid channel_id in MLS message");
                                        continue;
                                    }
                                };
                                let channel_id = ChannelId(channel_id_bytes);
                                
                                // Get the encrypted data (after marker + channel_id)
                                let encrypted_data = &data[33..];
                                
                                // Decrypt using the channel's MLS group
                                let decrypted_bytes = {
                                    let mut channel_mgr = channel_manager.write().await;
                                    let provider = mls_provider.read().await;
                                    
                                    match channel_mgr.get_mls_group_mut(&channel_id) {
                                        Some(mls_group) => {
                                            match mls_group.decrypt_application_message(encrypted_data, &provider) {
                                                Ok(plaintext) => {
                                                    println!("  ✓ Decrypted Channel MLS message ({} bytes)", plaintext.len());
                                                    plaintext
                                                }
                                                Err(e) => {
                                                    eprintln!("  ⚠️ Failed to decrypt Channel MLS message: {}", e);
                                                    eprintln!("     (You may have been removed from this Channel)");
                                                    continue;
                                                }
                                            }
                                        }
                                        None => {
                                            eprintln!("  ⚠️ No MLS group found for channel_id {}", hex::encode(&channel_id.0[..8]));
                                            eprintln!("     (You may not be a member of this Channel)");
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
                                        crate::crdt::OpType::CreateInvite(_) => {
                                            let mut manager = space_manager.write().await;
                                            if let Err(e) = manager.process_create_invite(&op) {
                                                eprintln!("⚠️ Failed to process CreateInvite: {}", e);
                                            }
                                        }
                                        crate::crdt::OpType::RevokeInvite(_) => {
                                            let mut manager = space_manager.write().await;
                                            if let Err(e) = manager.process_revoke_invite(&op) {
                                                eprintln!("⚠️ Failed to process RevokeInvite: {}", e);
                                            }
                                        }
                                        crate::crdt::OpType::UseInvite(_) => {
                                            let mut manager = space_manager.write().await;
                                            if let Err(e) = manager.process_use_invite(&op) {
                                                eprintln!("⚠️ Failed to process UseInvite: {}", e);
                                            } else {
                                                println!("✓ Processed UseInvite: user joined space {}", op.space_id);
                                            }
                                        }
                                        crate::crdt::OpType::AddMember(_) => {
                                            // AddMember operations add a user to the space
                                            if let crate::crdt::OpType::AddMember(crate::crdt::OpPayload::AddMember { user_id, role }) = &op.op_type {
                                                let mut manager = space_manager.write().await;
                                                // Access spaces HashMap directly (SpaceManager::spaces is private, so use process_use_invite pattern)
                                                // For now, just log - AddMember is handled by MLS flow or use_invite
                                                println!("ℹ AddMember operation received for user {} on space {}", user_id, op.space_id);
                                                println!("  (Members are added via invite or MLS Welcome message)");
                                            }
                                        }
                                        crate::crdt::OpType::RemoveMember(_) => {
                                            let mut manager = space_manager.write().await;
                                            if let Err(e) = manager.process_remove_member(&op) {
                                                eprintln!("⚠️ Failed to process RemoveMember: {}", e);
                                            }
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
        // Use default membership mode (MLS) for backwards compatibility
        self.create_space_with_mode(name, description, visibility, SpaceMembershipMode::default()).await
    }

    /// Create a new Space with specific visibility and membership mode
    /// 
    /// This is the full-featured space creation API that allows specifying both:
    /// - Visibility: Who can discover and join the space
    /// - Membership Mode: Whether to use space-level MLS encryption or lightweight mode
    /// 
    /// Privacy Warning: This function returns privacy information that MUST be shown to the user
    /// before the space is created.
    pub async fn create_space_with_mode(
        &self,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
        membership_mode: SpaceMembershipMode,
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
        let op = manager.create_space_with_mode(
            space_id,
            name,
            description,
            visibility,
            membership_mode,
            self.user_id,
            &self.keypair,
            &provider,
        )?;
        drop(provider);
        
        // Get the space before dropping the lock
        let space = manager.get_space(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?
            .clone();
        
        // **CRITICAL**: Drop the lock BEFORE broadcasting to avoid deadlock
        // broadcast_op_on_topic needs to acquire space_manager lock for MLS encryption
        drop(manager);
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation on space topic
        self.broadcast_op(&op).await?;
        
        // Auto-subscribe to the space topic
        self.subscribe_to_space(&space_id).await?;
        
        // ALSO broadcast CreateSpace on discovery topic so peers can discover and join
        // This allows peers who aren't subscribed to the space yet to receive the initial CreateSpace op
        let _ = self.broadcast_op_on_topic(&op, "descord/space-discovery").await;
        
        // Store Space metadata in DHT for offline discovery
        // (space_manager lock already dropped above)
        if let Err(e) = self.dht_put_space(&space_id).await {
            eprintln!("⚠️  Failed to store Space in DHT: {}", e);
            // Non-fatal - space still created locally
        }
        
        // Print mode information
        if membership_mode.is_lightweight() {
            println!("ℹ️  Created LIGHTWEIGHT space - no space-level MLS group");
            println!("   Channels will provide E2EE when you create them.");
        } else {
            println!("ℹ️  Created MLS-encrypted space - space-level encryption enabled");
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
        let op = {
            let mut manager = self.space_manager.write().await;
            manager.update_space_visibility(
                space_id,
                visibility,
                self.user_id,
                &self.keypair,
            )?
        }; // Lock dropped here
        
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
        println!("🎫 [CLIENT::CREATE_INVITE] Called");
        println!("   Space: {}", hex::encode(&space_id.0[..8]));
        println!("   User: {}", hex::encode(&self.user_id.as_bytes()[..8]));
        
        let op = {
            let mut manager = self.space_manager.write().await;
            manager.create_invite(
                space_id,
                self.user_id,
                &self.keypair,
                max_uses,
                max_age_hours,
            )?
        }; // Lock dropped here
        
        println!("✓ [CLIENT::CREATE_INVITE] Operation created, broadcasting...");
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        println!("✓ [CLIENT::CREATE_INVITE] Complete");
        
        Ok(op)
    }
    
    /// Revoke an invite
    pub async fn revoke_invite(
        &self,
        space_id: SpaceId,
        invite_id: InviteId,
    ) -> Result<CrdtOp> {
        let op = {
            let mut manager = self.space_manager.write().await;
            manager.revoke_invite(
                space_id,
                invite_id,
                self.user_id,
                &self.keypair,
            )?
        }; // Lock dropped here
        
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
        
        let op = {
            let mut manager = self.space_manager.write().await;
            manager.use_invite(
                space_id,
                code,
                self.user_id,
                &self.keypair,
            )?
        }; // Lock dropped here
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        // Subscribe to space topic for future updates
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
        
        // Convert metadata to Space - use Space::new_with_mode to properly initialize roles
        let mut space = crate::forum::Space::new_with_mode(
            metadata.id,
            metadata.name.clone(),
            metadata.description.clone(),
            metadata.owner,
            metadata.visibility,
            SpaceMembershipMode::default(),
            metadata.created_at,
        );
        
        // Update fields that aren't set by constructor
        space.members = metadata.initial_members.clone();
        space.invites = std::collections::HashMap::new();
        space.invite_permissions = metadata.invite_permissions.clone();
        space.epoch = metadata.epoch;
        
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
        
        eprintln!("🔷 [DHT_PUT_OPS] START: Storing {} operations for space {}", 
                 ops.len(), hex::encode(&space_id.0[..8]));
        
        if ops.is_empty() {
            eprintln!("🔷 [DHT_PUT_OPS] Empty ops, returning early");
            return Ok(());
        }
        
        // First, fetch or create the index
        eprintln!("🔷 [DHT_PUT_OPS] Step 1: Acquiring network lock...");
        let mut network = self.network.write().await;
        eprintln!("🔷 [DHT_PUT_OPS] Step 1: ✓ Network lock acquired");
        
        let index_key = OperationBatchIndex::compute_dht_key(space_id);
        eprintln!("🔷 [DHT_PUT_OPS] Step 2: Fetching DHT index for key {}...", hex::encode(&index_key[..8]));
        
        let mut index = match network.dht_get(index_key.clone()).await {
            Ok(values) if !values.is_empty() => {
                eprintln!("🔷 [DHT_PUT_OPS] Step 2: ✓ Found existing index with {} values", values.len());
                OperationBatchIndex::from_bytes(&values[0])?
            }
            Ok(_) => {
                eprintln!("🔷 [DHT_PUT_OPS] Step 2: Creating new index (no values found)");
                OperationBatchIndex::new(*space_id)
            }
            Err(e) => {
                eprintln!("🔷 [DHT_PUT_OPS] Step 2: Creating new index (error: {})", e);
                // Create new index
                OperationBatchIndex::new(*space_id)
            }
        };
        
        // Get next sequence number
        let sequence = index.batch_sequences.last().copied().unwrap_or(0) + 1;
        eprintln!("🔷 [DHT_PUT_OPS] Step 3: Using sequence number {}", sequence);
        
        // Create operation batch
        eprintln!("🔷 [DHT_PUT_OPS] Step 4: Creating operation batch...");
        let batch = OperationBatch::new(*space_id, ops.clone(), sequence);
        
        // Encrypt batch
        eprintln!("🔷 [DHT_PUT_OPS] Step 5: Encrypting batch...");
        let encrypted = EncryptedOperationBatch::encrypt(&batch)?;
        eprintln!("🔷 [DHT_PUT_OPS] Step 5: ✓ Batch encrypted");
        
        // Store batch in DHT
        let batch_key = encrypted.dht_key();
        let batch_bytes = encrypted.to_bytes()?;
        eprintln!("🔷 [DHT_PUT_OPS] Step 6: Storing batch in DHT (key: {}, size: {} bytes)...", 
                 hex::encode(&batch_key[..8]), batch_bytes.len());
        network.dht_put(batch_key, batch_bytes).await?;
        eprintln!("🔷 [DHT_PUT_OPS] Step 6: ✓ Batch stored in DHT");
        
        // Update index
        eprintln!("🔷 [DHT_PUT_OPS] Step 7: Updating index...");
        index.add_batch(sequence, ops.len() as u32);
        
        // Store updated index
        let index_bytes = index.to_bytes()?;
        eprintln!("🔷 [DHT_PUT_OPS] Step 8: Storing updated index in DHT (size: {} bytes)...", index_bytes.len());
        network.dht_put(index_key, index_bytes).await?;
        eprintln!("🔷 [DHT_PUT_OPS] Step 8: ✓ Index stored in DHT");
        
        eprintln!("🔷 [DHT_PUT_OPS] END: ✓ Successfully stored {} operations in DHT (batch {})", ops.len(), sequence);
        
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
        // Use the same topic that members subscribe to: "space/{space_id}"
        let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
        
        // Convert MLS messages to bytes - OpenMLS MlsMessageOut has to_bytes() method
        let commit_bytes = commit_msg.to_bytes()
            .map_err(|e| crate::Error::Serialization(format!("Failed to serialize Commit: {:?}", e)))?;
        let mut network = self.network.write().await;
        
        // Attempt to send Commit (may fail if no peers subscribed to /mls topic - that's OK)
        match network.publish(&space_topic, commit_bytes).await {
            Ok(_) => println!("✓ Sent Commit message to existing members on {}", space_topic),
            Err(e) => println!("⚠️ Could not send Commit (no peers on {} topic): {}", space_topic, e),
        }
        
        // Serialize and send Welcome to new member (via direct topic)
        let welcome_topic = format!("user/{}/welcome", hex::encode(&user_id.0[..8]));
        let welcome_bytes = welcome_msg.to_bytes()
            .map_err(|e| crate::Error::Serialization(format!("Failed to serialize Welcome: {:?}", e)))?;
        
        match network.publish(&welcome_topic, welcome_bytes).await {
            Ok(_) => println!("✓ Sent Welcome message to {} on {}", hex::encode(&user_id.0[..8]), welcome_topic),
            Err(e) => {
                eprintln!("✗ Failed to send Welcome message to {}: {}", welcome_topic, e);
                eprintln!("  This means the new member won't be able to decrypt messages!");
            }
        }
        
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
        let (op, commit_msg_opt) = manager.remove_member(
            space_id,
            user_id,
            self.user_id,
            &self.keypair,
            &provider,
        )?;
        drop(provider);
        drop(manager);
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        // If we got a Commit message, broadcast it to remaining members
        if let Some(commit_msg) = commit_msg_opt {
            println!("  📡 Broadcasting Commit to remaining members...");
            let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
            let commit_bytes = commit_msg.to_bytes()
                .map_err(|e| Error::Serialization(format!("Failed to serialize Commit: {:?}", e)))?;
            
            let mut network = self.network.write().await;
            match network.publish(&space_topic, commit_bytes).await {
                Ok(_) => println!("  ✓ Commit broadcast - remaining members will update to new epoch"),
                Err(e) => eprintln!("  ⚠️ Could not broadcast Commit: {}", e),
            }
        }
        
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
        
        // Check permissions
        {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            
            // Owner bypasses all permission checks
            if space.owner != self.user_id {
                // Check if user has CREATE_CHANNELS permission
                if !space.can_create_channels(&self.user_id) {
                    return Err(Error::Rejected(
                        "Permission denied: You don't have CREATE_CHANNELS permission".to_string()
                    ));
                }
            }
        }
        
        // Get current epoch from Space
        let epoch = {
            let space_manager = self.space_manager.read().await;
            let space = space_manager.get_space(&space_id)
                .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?;
            space.epoch
        };
        
        let mut manager = self.channel_manager.write().await;
        let provider = self.mls_provider.read().await;
        
        // Create channel with MLS group
        let op = manager.create_channel_with_mls(
            channel_id,
            space_id,
            name,
            description,
            self.user_id,
            &self.keypair,
            epoch,
            true, // Always create channel-level MLS group
            Some(&provider),
        )?;
        
        drop(provider); // Release MLS provider lock
        
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
    
    /// Add a user to a Channel (with channel-level MLS encryption)
    pub async fn add_to_channel(
        &self,
        channel_id: &ChannelId,
        user_id: UserId,
        role: Role,
    ) -> Result<()> {
        // Get the user's key package from DHT using existing method
        let key_package_bundle = self.fetch_key_package_from_dht(&user_id).await?;
        
        // Serialize the key package bytes
        let key_package_bytes = &key_package_bundle.key_package_bytes;
        
        let provider = self.mls_provider.read().await;
        let mut manager = self.channel_manager.write().await;
        
        // Add to channel's MLS group (using self.user_id as the admin performing the action)
        let welcome_bytes = manager.add_member_with_mls(
            channel_id,
            user_id,
            role,
            key_package_bytes,
            &self.user_id,  // Pass the caller's user_id as admin
            &provider,
        ).map_err(|e| Error::Mls(format!("Failed to add member to channel: {}", e)))?;
        drop(provider);
        drop(manager);
        
        // Send Welcome message to the new member via their personal topic
        let user_topic = format!("user/{}/welcome", hex::encode(&user_id.0));
        {
            let mut network = self.network.write().await;
            network.publish(&user_topic, welcome_bytes).await?;
        }
        println!("  ✅ Sent channel Welcome message to {} on {}", hex::encode(&user_id.0[..8]), user_topic);
        
        Ok(())
    }
    
    /// Remove a user from a Channel (kicks from channel's MLS group only, not from space)
    pub async fn kick_from_channel(
        &self,
        channel_id: &ChannelId,
        user_id: &UserId,
    ) -> Result<()> {
        let provider = self.mls_provider.read().await;
        let mut manager = self.channel_manager.write().await;
        
        // Remove from channel's MLS group (using self.user_id as admin)
        let _commit_bytes = manager.remove_member_with_mls(
            channel_id,
            user_id,
            &self.user_id,
            &provider,
        ).map_err(|e| Error::Mls(format!("Failed to remove member from channel: {}", e)))?;
        
        // TODO: Broadcast Commit message to channel members via DHT
        
        Ok(())
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
        // Auto-join channel MLS group if needed (Phase 2: Per-channel encryption)
        {
            let thread_manager = self.thread_manager.read().await;
            if let Some(thread) = thread_manager.get_thread(&thread_id) {
                let channel_id = thread.channel_id;
                drop(thread_manager);
                
                // Check if user is in this channel's MLS group
                let channel_manager = self.channel_manager.read().await;
                if let Some(channel) = channel_manager.get_channel(&channel_id) {
                    let is_member = channel.is_member(&self.user_id);
                    let has_mls_group = channel_manager.get_mls_group(&channel_id).is_some();
                    
                    println!("  🔍 Channel auto-join check: is_member={}, has_mls_group={}", is_member, has_mls_group);
                    
                    drop(channel_manager);
                    
                    // Auto-add to channel MLS group if:
                    // 1. User is not yet a member of the channel
                    // 2. Channel has an MLS group (it should, they're always created)
                    if !is_member && has_mls_group {
                        println!("  🔐 Auto-joining channel MLS group...");
                        // Get user's key package from DHT
                        match self.fetch_key_package_from_dht(&self.user_id).await {
                            Ok(key_package_bundle) => {
                                let key_package_bytes = &key_package_bundle.key_package_bytes;
                                let provider = self.mls_provider.read().await;
                                let mut channel_mgr = self.channel_manager.write().await;
                                
                                // Add user to channel's MLS group
                                match channel_mgr.add_member_with_mls(
                                    &channel_id,
                                    self.user_id,
                                    Role::Member,
                                    key_package_bytes,
                                    &self.user_id,  // User is adding themselves (self-join)
                                    &provider,
                                ) {
                                    Ok(_welcome_bytes) => {
                                        println!("  ✓ Auto-joined channel MLS group");
                                        // TODO: Store Welcome message for offline sync
                                    }
                                    Err(e) => {
                                        eprintln!("  ⚠️ Failed to auto-join channel MLS: {}", e);
                                        // Continue anyway - user can still post to channel
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("  ⚠️ No key package found for auto-join: {}", e);
                                // Continue anyway
                            }
                        }
                    }
                }
            }
        }
        
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
        
        eprintln!("📢 [BROADCAST START] Broadcasting operation on topic: {}", topic);
        eprintln!("📢 [BROADCAST] Operation type: {:?}, space_id: {}", 
                 std::any::type_name_of_val(&op.op_type), hex::encode(&op.space_id.0[..8]));
        
        // Broadcast via GossipSub
        eprintln!("📢 [BROADCAST] Step 1: Calling broadcast_op_on_topic (GossipSub)...");
        self.broadcast_op_on_topic(op, &topic).await?;
        eprintln!("📢 [BROADCAST] Step 1: ✓ GossipSub broadcast completed");
        
        // Store in DHT for offline sync
        // Note: We store each operation individually for now
        // TODO: Batch operations for efficiency
        eprintln!("📢 [BROADCAST] Step 2: Calling dht_put_operations (DHT storage)...");
        let result = self.dht_put_operations(&op.space_id, vec![op.clone()]).await;
        match &result {
            Ok(_) => eprintln!("📢 [BROADCAST] Step 2: ✓ DHT storage completed"),
            Err(e) => eprintln!("📢 [BROADCAST] Step 2: ✗ DHT storage failed: {}", e),
        }
        if let Err(e) = result {
            // Don't fail if DHT storage fails (degraded mode)
            eprintln!("⚠ Failed to store operation in DHT: {}", e);
        }
        
        eprintln!("📢 [BROADCAST END] Broadcast operation completed");
        Ok(())
    }
    
    /// Broadcast a CRDT operation to a specific topic
    async fn broadcast_op_on_topic(&self, op: &CrdtOp, topic: &str) -> Result<()> {
        eprintln!("🔵 [GOSSIPSUB] START: Broadcasting to topic {}", topic);
        
        // Serialize the operation
        eprintln!("🔵 [GOSSIPSUB] Step A: Serializing operation...");
        let op_bytes = minicbor::to_vec(op)
            .map_err(|e| Error::Serialization(format!("Failed to encode operation: {}", e)))?;
        eprintln!("🔵 [GOSSIPSUB] Step A: ✓ Serialized {} bytes", op_bytes.len());
        
        // Check if this Space has an MLS group - if so, encrypt the operation
        eprintln!("🔵 [GOSSIPSUB] Step B: Acquiring space_manager lock...");
        let data = {
            // First check for channel-level MLS group (for operations in channels)
            let mut channel_encrypted = false;
            let mut data = Vec::new();
            
            // Check if this operation is for a specific channel
            if let Some(channel_id) = &op.channel_id {
                let mut channel_manager = self.channel_manager.write().await;
                if let Some(mls_group) = channel_manager.get_mls_group_mut(channel_id) {
                    eprintln!("🔵 [GOSSIPSUB] Step C: Channel MLS group found, encrypting...");
                    // Encrypt the operation as MLS application data using channel's group
                    let provider = self.mls_provider.read().await;
                    let encrypted_msg = mls_group.encrypt_application_message(&op_bytes, &provider)?;
                    drop(provider);
                    drop(channel_manager);
                    eprintln!("🔵 [GOSSIPSUB] Step C: ✓ Encrypted with channel MLS");
                    
                    // Serialize the encrypted MLS message
                    eprintln!("🔵 [GOSSIPSUB] Step D: Serializing encrypted message...");
                    let encrypted_bytes = encrypted_msg.to_bytes()
                        .map_err(|e| Error::Serialization(format!("Failed to serialize MLS message: {}", e)))?;
                    eprintln!("🔵 [GOSSIPSUB] Step D: ✓ Serialized {} bytes", encrypted_bytes.len());
                    
                    // Format: [0x02][channel_id (32 bytes)][encrypted_data]
                    // 0x02 indicates channel-level encryption
                    data = vec![0x02];
                    data.extend_from_slice(&channel_id.0);
                    data.extend_from_slice(&encrypted_bytes);
                    channel_encrypted = true;
                } else {
                    drop(channel_manager);
                }
            }
            
            // If not encrypted at channel level, check for space-level MLS
            if !channel_encrypted {
                let mut space_manager = self.space_manager.write().await;
                eprintln!("🔵 [GOSSIPSUB] Step B: ✓ Lock acquired, checking for MLS group...");
                
                if let Some(mls_group) = space_manager.get_mls_group_mut(&op.space_id) {
                    eprintln!("🔵 [GOSSIPSUB] Step C: Space MLS group found, encrypting...");
                    // Encrypt the operation as MLS application data
                    let provider = self.mls_provider.read().await;
                    let encrypted_msg = mls_group.encrypt_application_message(&op_bytes, &provider)?;
                    drop(provider);
                    eprintln!("🔵 [GOSSIPSUB] Step C: ✓ Encrypted");
                    
                    // Serialize the encrypted MLS message
                    eprintln!("🔵 [GOSSIPSUB] Step D: Serializing encrypted message...");
                    let encrypted_bytes = encrypted_msg.to_bytes()
                        .map_err(|e| Error::Serialization(format!("Failed to serialize MLS message: {}", e)))?;
                    eprintln!("🔵 [GOSSIPSUB] Step D: ✓ Serialized {} bytes", encrypted_bytes.len());
                    
                    // Format: [0x01][space_id (32 bytes)][encrypted_data]
                    // The space_id is needed for decryption on the receive side
                    data = vec![0x01];
                    data.extend_from_slice(&op.space_id.0);
                    data.extend_from_slice(&encrypted_bytes);
                } else {
                    eprintln!("🔵 [GOSSIPSUB] Step C: No MLS group, using plaintext");
                    // No MLS group - send plaintext with marker (0x00)
                    data = vec![0x00];
                    data.extend_from_slice(&op_bytes);
                }
            }
            
            data
        };
        eprintln!("🔵 [GOSSIPSUB] Step E: Data prepared ({} bytes), acquiring network lock...", data.len());
        
        let mut network = self.network.write().await;
        eprintln!("🔵 [GOSSIPSUB] Step E: ✓ Network lock acquired");
        
        // Attempt to publish, but don't fail if no peers are connected
        // This is expected in single-node scenarios and tests
        eprintln!("🔵 [GOSSIPSUB] Step F: Calling network.publish...");
        let result = network.publish(topic, data).await;
        eprintln!("🔵 [GOSSIPSUB] Step F: ✓ Publish returned: {:?}", result.is_ok());
        
        // Record metrics
        eprintln!("🔵 [GOSSIPSUB] Step G: Recording metrics...");
        if result.is_ok() {
            self.gossip_metrics.record_publish(topic).await;
        }
        eprintln!("🔵 [GOSSIPSUB] Step G: ✓ Metrics recorded");
        
        eprintln!("🔵 [GOSSIPSUB] END: Completed");
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
    
    // ===== DASHBOARD API =====
    
    /// Get a dashboard snapshot of this client's state
    /// 
    /// Returns a serializable snapshot containing:
    /// - All spaces this client is a member of
    /// - Channels within those spaces
    /// - Connected peers
    /// - DHT storage metadata
    /// - MLS group information
    /// 
    /// This is safe to expose - it contains no private keys or sensitive crypto material.
    pub async fn get_dashboard_snapshot(&self, client_name: &str) -> crate::dashboard::ClientSnapshot {
        use crate::dashboard::{ClientSnapshot, SpaceSnapshot, ChannelSnapshot, ThreadSnapshot, MessageSnapshot};
        
        let user_id_hex = hex::encode(&self.user_id.0);
        
        // Build spaces list with channels and threads
        let mut spaces: Vec<SpaceSnapshot> = Vec::new();
        
        {
            let space_manager = self.space_manager.read().await;
            let channel_manager = self.channel_manager.read().await;
            let thread_manager = self.thread_manager.read().await;
            
            for space in space_manager.list_spaces() {
                let mut snapshot = SpaceSnapshot::from_space(space);
                
                // Add channels for this space
                snapshot.channels = channel_manager.list_channels(&space.id)
                    .iter()
                    .map(|chan| {
                        let mut channel_snapshot = ChannelSnapshot::from_channel(chan);
                        
                        // Add threads for this channel
                        channel_snapshot.threads = thread_manager.list_threads(&chan.id)
                            .iter()
                            .map(|thread| {
                                ThreadSnapshot::from_thread(thread, Vec::new())
                            })
                            .collect();
                        
                        channel_snapshot
                    })
                    .collect();
                
                spaces.push(snapshot);
            }
        }
        
        // Now populate messages for each thread (need to release locks first)
        for space in &mut spaces {
            for channel in &mut space.channels {
                for thread in &mut channel.threads {
                    let thread_id_bytes = hex::decode(&thread.id).unwrap_or_default();
                    if thread_id_bytes.len() == 32 {
                        let mut id_arr = [0u8; 32];
                        id_arr.copy_from_slice(&thread_id_bytes);
                        let thread_id = crate::ThreadId(id_arr);
                        
                        let messages = self.list_messages(&thread_id).await;
                        thread.messages = messages
                            .iter()
                            .map(|msg| MessageSnapshot::from_message(msg))
                            .collect();
                    }
                }
            }
        }
        
        // Get connected peers
        let connected_peers = self.get_connected_peers().await;
        
        // Mock DHT storage (TODO: implement real DHT query)
        let dht_storage = vec![];
        
        // Mock MLS groups (TODO: query actual MLS group state)
        let mls_groups = vec![];
        
        ClientSnapshot {
            name: client_name.to_string(),
            user_id: user_id_hex,
            spaces,
            dht_storage,
            mls_groups,
            connected_peers,
        }
    }
    
    /// Get list of spaces as snapshots
    pub async fn list_spaces_snapshot(&self) -> Vec<crate::dashboard::SpaceSnapshot> {
        let space_manager = self.space_manager.read().await;
        space_manager.list_spaces()
            .iter()
            .map(|space| crate::dashboard::SpaceSnapshot::from_space(space))
            .collect()
    }
    
    /// Get a single space snapshot by ID
    pub async fn get_space_snapshot(&self, space_id: SpaceId) -> Option<crate::dashboard::SpaceSnapshot> {
        let space_manager = self.space_manager.read().await;
        space_manager.get_space(&space_id)
            .map(|space| {
                let mut snapshot = crate::dashboard::SpaceSnapshot::from_space(space);
                
                // We can't add channels here because we hold space_manager lock
                // Channels should be added separately by the caller
                snapshot
            })
    }
    
    /// Get connected peer IDs
    pub async fn get_connected_peers(&self) -> Vec<String> {
        let network = self.network.read().await;
        network.connected_peers().await
            .into_iter()
            .map(|peer_id| peer_id.to_string())
            .collect()
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
