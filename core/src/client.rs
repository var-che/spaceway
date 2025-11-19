//! Client API for Descord
//!
//! High-level API for interacting with Spaces, Channels, Threads, and Messages.
//! Integrates CRDT operations, MLS encryption, and P2P networking.

use crate::crdt::CrdtOp;
use crate::crypto::signing::Keypair;
use crate::forum::{Space, SpaceManager, Channel, ChannelManager, Thread, ThreadManager, Message};
use crate::mls::provider::{create_provider, DescordProvider};
use crate::network::{NetworkNode, NetworkEvent};
use crate::storage::{Store, BlobStorage, BlobMetadata};
use crate::types::*;
use crate::{Error, Result};

use std::path::PathBuf;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

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
    
    /// Blob storage
    blob_storage: Arc<RwLock<BlobStorage>>,
    
    /// Network node
    network: Arc<RwLock<NetworkNode>>,
    
    /// Network event receiver
    network_rx: Arc<RwLock<mpsc::UnboundedReceiver<NetworkEvent>>>,
    
    /// Storage backend
    store: Arc<Store>,
    
    /// MLS provider
    mls_provider: DescordProvider,
}

impl Client {
    /// Create a new client with the given keypair and configuration
    pub fn new(keypair: Keypair, config: ClientConfig) -> Result<Self> {
        let user_id = keypair.user_id();
        
        // Create storage
        let store = Arc::new(Store::open(&config.storage_path)?);
        
        // Create managers
        let space_manager = Arc::new(RwLock::new(SpaceManager::new()));
        let channel_manager = Arc::new(RwLock::new(ChannelManager::new()));
        let thread_manager = Arc::new(RwLock::new(ThreadManager::new()));
        let blob_storage = Arc::new(RwLock::new(BlobStorage::new()));
        
        // Create network
        let (network_node, network_rx) = NetworkNode::new()?;
        let network = Arc::new(RwLock::new(network_node));
        let network_rx = Arc::new(RwLock::new(network_rx));
        
        // Create MLS provider
        let mls_provider = create_provider();
        
        Ok(Self {
            keypair,
            user_id,
            space_manager,
            channel_manager,
            thread_manager,
            blob_storage,
            network,
            network_rx,
            store,
            mls_provider,
        })
    }
    
    /// Start the client (network and event processing)
    pub async fn start(&self) -> Result<()> {
        // Spawn event processing task
        let space_manager = Arc::clone(&self.space_manager);
        let channel_manager = Arc::clone(&self.channel_manager);
        let thread_manager = Arc::clone(&self.thread_manager);
        let store = Arc::clone(&self.store);
        let network_rx = Arc::clone(&self.network_rx);
        
        tokio::spawn(async move {
            loop {
                let event_opt = {
                    let mut rx = network_rx.write().await;
                    rx.recv().await
                };
                
                if let Some(event) = event_opt {
                    match event {
                        NetworkEvent::MessageReceived { topic: _, data, source: _ } => {
                            // Decode CRDT operation
                            if let Ok(op) = minicbor::decode::<CrdtOp>(&data) {
                                // Store the operation
                                let _ = store.put_op(&op);
                                
                                // Process based on operation type
                                match &op.op_type {
                                    crate::crdt::OpType::CreateSpace(_) => {
                                        let mut manager = space_manager.write().await;
                                        let _ = manager.process_create_space(&op);
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
                        }
                        NetworkEvent::PeerConnected(peer_id) => {
                            println!("Peer connected: {}", peer_id);
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
    ) -> Result<(Space, CrdtOp)> {
        let space_id = SpaceId(uuid::Uuid::new_v4());
        
        let mut manager = self.space_manager.write().await;
        let op = manager.create_space(
            space_id,
            name,
            description,
            self.user_id,
            &self.keypair,
            &self.mls_provider,
        )?;
        
        // Store operation
        self.store.put_op(&op)?;
        
        // Broadcast operation
        self.broadcast_op(&op).await?;
        
        let space = manager.get_space(&space_id)
            .ok_or_else(|| Error::NotFound(format!("Space {:?} not found", space_id)))?
            .clone();
        
        Ok((space, op))
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
    
    /// Create a Channel in a Space
    pub async fn create_channel(
        &self,
        space_id: SpaceId,
        name: String,
        description: Option<String>,
    ) -> Result<(Channel, CrdtOp)> {
        let channel_id = ChannelId(uuid::Uuid::new_v4());
        
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
        let thread_id = ThreadId(uuid::Uuid::new_v4());
        
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
        let message_id = MessageId(uuid::Uuid::new_v4());
        
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
    pub async fn store_blob(
        &self,
        data: &[u8],
        mime_type: Option<String>,
        filename: Option<String>,
    ) -> Result<BlobMetadata> {
        let storage = self.blob_storage.write().await;
        let metadata = storage.store(data, mime_type, filename)?;
        
        // Store blob in persistent storage
        self.store.put_blob(&metadata.hash, data)?;
        
        Ok(metadata)
    }
    
    /// Retrieve a blob by hash
    pub async fn retrieve_blob(&self, hash: &ContentHash) -> Result<Vec<u8>> {
        // Try in-memory first
        let storage = self.blob_storage.read().await;
        if let Ok(data) = storage.retrieve(hash) {
            return Ok(data);
        }
        
        // Fall back to persistent storage
        self.store.get_blob(hash)?
            .ok_or_else(|| Error::NotFound(format!("Blob {:?} not found", hash)))
    }
    
    /// Broadcast a CRDT operation to the network
    async fn broadcast_op(&self, op: &CrdtOp) -> Result<()> {
        let topic = format!("space/{}", op.space_id.0);
        let data = minicbor::to_vec(op)
            .map_err(|e| Error::Serialization(format!("Failed to encode operation: {}", e)))?;
        
        let mut network = self.network.write().await;
        
        // Attempt to publish, but don't fail if no peers are connected
        // This is expected in single-node scenarios and tests
        let _ = network.publish(&topic, data).await;
        
        Ok(())
    }
    
    /// Subscribe to a Space's operation stream
    pub async fn subscribe_to_space(&self, space_id: &SpaceId) -> Result<()> {
        let topic = format!("space/{}", space_id.0);
        let mut network = self.network.write().await;
        network.subscribe(&topic).await?;
        
        Ok(())
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
    async fn handle_incoming_op(&self, op: CrdtOp) -> Result<()> {
        // Store the operation
        self.store.put_op(&op)?;
        
        // Process based on operation type
        match &op.op_type {
            crate::crdt::OpType::CreateSpace(_) => {
                let mut manager = self.space_manager.write().await;
                manager.process_create_space(&op)?;
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
        
        let (space, _op) = client.create_space(
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
        
        let (space, _) = client.create_space("Test Space".to_string(), None).await.unwrap();
        
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
        
        let (space, _) = client.create_space("Test Space".to_string(), None).await.unwrap();
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
        
        let data = b"Test attachment data";
        let metadata = client.store_blob(
            data,
            Some("text/plain".to_string()),
            Some("test.txt".to_string()),
        ).await.unwrap();
        
        let retrieved = client.retrieve_blob(&metadata.hash).await.unwrap();
        assert_eq!(retrieved, data);
    }
}
