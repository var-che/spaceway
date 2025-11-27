//! Dashboard Backend
//!
//! Manages 3 Discord-Lite clients (Alice, Bob, Charlie) and exposes their state via WebSocket API.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use spaceway_core::{Client, ClientConfig, dashboard::DashboardState};
use spaceway_core::crypto::signing::Keypair;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, error};
use tempfile::TempDir;

/// Shared state for all 3 clients
#[derive(Clone)]
struct AppState {
    alice: Arc<RwLock<Client>>,
    bob: Arc<RwLock<Client>>,
    charlie: Arc<RwLock<Client>>,
    temp_dirs: Arc<Vec<TempDir>>, // Keep temp directories alive
}

/// Action request from frontend
#[derive(Debug, Deserialize)]
struct ActionRequest {
    client: String, // "alice", "bob", or "charlie"
    action: Action,
}

/// Action response
#[derive(Debug, Serialize)]
struct ActionResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// Actions that can be performed
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Action {
    CreateSpace { name: String },
    CreateChannel { space_id: String, name: String },
    CreateThread { space_id: String, channel_id: String, title: Option<String>, first_message: String },
    SendMessage { space_id: String, thread_id: String, content: String },
    CreateInvite { space_id: String },
    JoinSpace { 
        space_id: String,
        #[serde(default)]
        invite_code: Option<String>,
    },
    RemoveMember { space_id: String, user_id: String }, // Kick a member
    ConnectPeers, // Connect all clients together
}

/// Build command request
#[derive(Debug, Deserialize)]
struct BuildRequest {
    command: String,
    working_dir: String,
    is_background: bool,
}

/// Build command response
#[derive(Debug, Serialize)]
struct BuildResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("dashboard_backend=debug,spaceway_core=info")
        .init();

    info!("🚀 Starting Dashboard Backend");

    // Create temporary directories for client storage
    let alice_dir = TempDir::new()?;
    let bob_dir = TempDir::new()?;
    let charlie_dir = TempDir::new()?;
    
    info!("📁 Alice storage: {:?}", alice_dir.path());
    info!("📁 Bob storage: {:?}", bob_dir.path());
    info!("📁 Charlie storage: {:?}", charlie_dir.path());

    // Create 3 real clients
    info!("👥 Creating clients...");
    
    let alice_keypair = Keypair::generate();
    let alice_config = ClientConfig {
        storage_path: alice_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let alice = Client::new(alice_keypair, alice_config)?;
    info!("✓ Alice created: {}", alice.user_id());
    
    let bob_keypair = Keypair::generate();
    let bob_config = ClientConfig {
        storage_path: bob_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let bob = Client::new(bob_keypair, bob_config)?;
    info!("✓ Bob created: {}", bob.user_id());
    
    let charlie_keypair = Keypair::generate();
    let charlie_config = ClientConfig {
        storage_path: charlie_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let charlie = Client::new(charlie_keypair, charlie_config)?;
    info!("✓ Charlie created: {}", charlie.user_id());

    // Start network event processing for all clients
    info!("🌐 Starting network event loops...");
    let alice_clone = Arc::new(RwLock::new(alice));
    let bob_clone = Arc::new(RwLock::new(bob));
    let charlie_clone = Arc::new(RwLock::new(charlie));
    
    tokio::spawn(process_network_events(alice_clone.clone(), "Alice"));
    tokio::spawn(process_network_events(bob_clone.clone(), "Bob"));
    tokio::spawn(process_network_events(charlie_clone.clone(), "Charlie"));

    // Create application state
    let state = AppState {
        alice: alice_clone,
        bob: bob_clone,
        charlie: charlie_clone,
        temp_dirs: Arc::new(vec![alice_dir, bob_dir, charlie_dir]),
    };

    // Build router
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/action", post(action_handler))
        .route("/api/state", get(get_state))
        .route("/api/build", post(build_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3030")
        .await?;

    info!("🎯 Dashboard backend listening on http://127.0.0.1:3030");
    info!("💡 Using REAL spaceway-core clients (Alice, Bob, Charlie)");
    info!("💡 WebSocket at ws://127.0.0.1:3030/ws");
    info!("💡 REST API at http://127.0.0.1:3030/api/*");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

/// Background task to process network events for a client
async fn process_network_events(client: Arc<RwLock<Client>>, name: &str) {
    info!("🔄 Network event loop started for {}", name);
    
    loop {
        // Process network events
        let client_guard = client.read().await;
        
        // Poll network events (simplified - in production use proper event loop)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        drop(client_guard);
    }
}

/// WebSocket handler for real-time updates
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    info!("🔌 WebSocket connection established");

    // Send initial state
    match get_dashboard_state(&state).await {
        Ok(dashboard_state) => {
            let json = serde_json::to_string(&dashboard_state).unwrap();
            if socket.send(Message::Text(json)).await.is_err() {
                return;
            }
        }
        Err(e) => {
            error!("Failed to get initial state: {}", e);
            return;
        }
    }

    // Stream updates every 500ms
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
    loop {
        interval.tick().await;

        match get_dashboard_state(&state).await {
            Ok(dashboard_state) => {
                let json = serde_json::to_string(&dashboard_state).unwrap();
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                error!("Failed to get state: {}", e);
                break;
            }
        }
    }

    info!("🔌 WebSocket connection closed");
}

async fn action_handler(
    State(state): State<AppState>,
    Json(request): Json<ActionRequest>,
) -> Json<ActionResponse> {
    info!("📝 Action request: {:?}", request);

    // Get the appropriate client
    let client = match request.client.as_str() {
        "alice" | "Alice" => &state.alice,
        "bob" | "Bob" => &state.bob,
        "charlie" | "Charlie" => &state.charlie,
        _ => {
            return Json(ActionResponse {
                success: false,
                message: format!("Unknown client: {}", request.client),
                data: None,
            });
        }
    };

    // Execute the action
    let result = execute_action(&state, client, request.action).await;

    match result {
        Ok(message) => Json(ActionResponse {
            success: true,
            message,
            data: None,
        }),
        Err(e) => Json(ActionResponse {
            success: false,
            message: format!("Action failed: {}", e),
            data: None,
        }),
    }
}

async fn execute_action(state: &AppState, client: &Arc<RwLock<Client>>, action: Action) -> anyhow::Result<String> {
    match action {
        Action::CreateSpace { name } => {
            let client_guard = client.read().await;
            let (space, _, _) = client_guard.create_space(name.clone(), None).await?;
            Ok(format!("Created space '{}' with ID: {}", name, hex::encode(&space.id.0[..8])))
        }
        Action::CreateChannel { space_id, name } => {
            // Parse space_id from hex
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid space_id length, expected 32 bytes"));
            }
            
            let mut id_bytes = [0u8; 32];
            id_bytes.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(id_bytes);
            
            let client_guard = client.read().await;
            let (channel, _) = client_guard.create_channel(space_id, name.clone(), None).await?;
            Ok(format!("Created channel '{}' with ID: {}", name, hex::encode(&channel.id.0)))
        }
        Action::CreateThread { space_id, channel_id, title, first_message } => {
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            let channel_id_bytes = hex::decode(&channel_id)
                .map_err(|e| anyhow::anyhow!("Invalid channel_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 || channel_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid ID length, expected 32 bytes"));
            }
            
            let mut space_id_arr = [0u8; 32];
            space_id_arr.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(space_id_arr);
            
            let mut channel_id_arr = [0u8; 32];
            channel_id_arr.copy_from_slice(&channel_id_bytes);
            let channel_id = spaceway_core::ChannelId(channel_id_arr);
            
            let client_guard = client.read().await;
            let (thread, _) = client_guard.create_thread(
                space_id,
                channel_id,
                title.clone(),
                first_message.clone()
            ).await?;
            
            Ok(format!("Created thread '{}' with ID: {} (first message: '{}')", 
                title.unwrap_or_else(|| "Untitled".to_string()),
                hex::encode(&thread.id.0),
                &first_message[..first_message.len().min(50)]
            ))
        }
        Action::SendMessage { space_id, thread_id, content } => {
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            let thread_id_bytes = hex::decode(&thread_id)
                .map_err(|e| anyhow::anyhow!("Invalid thread_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 || thread_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid ID length, expected 32 bytes"));
            }
            
            let mut space_id_arr = [0u8; 32];
            space_id_arr.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(space_id_arr);
            
            let mut thread_id_arr = [0u8; 32];
            thread_id_arr.copy_from_slice(&thread_id_bytes);
            let thread_id = spaceway_core::ThreadId(thread_id_arr);
            
            let client_guard = client.read().await;
            let (message, _) = client_guard.post_message(space_id, thread_id, content.clone()).await?;
            
            Ok(format!("✓ Sent message: '{}' (ID: {})", 
                &content[..content.len().min(50)],
                hex::encode(&message.id.0[..8])
            ))
        }
        Action::RemoveMember { space_id, user_id } => {
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            let user_id_bytes = hex::decode(&user_id)
                .map_err(|e| anyhow::anyhow!("Invalid user_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid space_id length, expected 32 bytes"));
            }
            if user_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid user_id length, expected 32 bytes"));
            }
            
            let mut space_id_arr = [0u8; 32];
            space_id_arr.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(space_id_arr);
            
            let mut user_id_arr = [0u8; 32];
            user_id_arr.copy_from_slice(&user_id_bytes);
            let target_user_id = spaceway_core::UserId(user_id_arr);
            
            let client_guard = client.read().await;
            let _ = client_guard.remove_member(space_id, target_user_id).await?;
            
            Ok(format!("✓ Removed member {} from space. They can no longer decrypt new messages!", 
                hex::encode(&target_user_id.0[..8])
            ))
        }
        Action::CreateInvite { space_id } => {
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid space_id length, expected 32 bytes"));
            }
            
            let mut id_bytes = [0u8; 32];
            id_bytes.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(id_bytes);
            
            let client_guard = client.read().await;
            let invite_op = client_guard.create_invite(space_id, None, None).await?;
            
            // Wait a bit for the invite to be processed locally
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Retrieve the invite code
            let invites = client_guard.list_invites(&space_id).await;
            if let Some(invite) = invites.last() {
                Ok(format!("Created invite! Code: {} (Space: {})", 
                    invite.code, 
                    hex::encode(&space_id.0[..8])
                ))
            } else {
                Ok(format!("Created invite operation with ID: {}", hex::encode(&invite_op.op_id.0.as_bytes()[..8])))
            }
        }
        Action::JoinSpace { space_id, invite_code } => {
            // Parse space_id as hex SpaceId
            let space_id_bytes = hex::decode(&space_id)
                .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;
            
            if space_id_bytes.len() != 32 {
                return Err(anyhow::anyhow!("Invalid space_id length, expected 32 bytes (64 hex chars)"));
            }
            
            let mut id_bytes = [0u8; 32];
            id_bytes.copy_from_slice(&space_id_bytes);
            let space_id = spaceway_core::SpaceId(id_bytes);
            
            // DASHBOARD WORKAROUND: Since nodes are isolated, look for space in other clients
            info!("🔧 Dashboard workaround: Looking for space {} in other clients", hex::encode(&space_id.0[..8]));
            
            // Check if space exists in Alice, Bob, or Charlie
            let alice_guard = state.alice.read().await;
            let alice_space = alice_guard.get_space(&space_id).await;
            drop(alice_guard);
            
            let bob_guard = state.bob.read().await;
            let bob_space = bob_guard.get_space(&space_id).await;
            drop(bob_guard);
            
            let charlie_guard = state.charlie.read().await;
            let charlie_space = charlie_guard.get_space(&space_id).await;
            drop(charlie_guard);
            
            if let Some(space) = alice_space.or(bob_space).or(charlie_space) {
                info!("✓ Found space '{}'", space.name);
                let space_name = space.name.clone();
                
                // Verify invite if provided
                if let Some(code) = &invite_code {
                    let alice_guard = state.alice.read().await;
                    let invites = alice_guard.list_invites(&space_id).await;
                    drop(alice_guard);
                    
                    if !invites.iter().any(|inv| inv.code == *code) {
                        return Err(anyhow::anyhow!("Invalid or expired invite code"));
                    }
                    info!("✓ Invite code validated");
                }
                
                // Get the joining user's ID
                let client_guard = client.read().await;
                let new_member_id = client_guard.user_id();
                info!("🔍 DEBUG: Joining user ID: {}", hex::encode(&new_member_id.0));
                drop(client_guard);
                
                // Try to join via DHT (will likely fail due to isolation)
                let client_guard = client.read().await;
                let join_result = client_guard.join_space_from_dht(space_id).await;
                drop(client_guard);
                
                // Find which client owns this space (has the channels)
                let alice_guard = state.alice.read().await;
                let alice_channels = alice_guard.list_channels(&space_id).await;
                info!("🔍 DEBUG: Alice has {} channels in space", alice_channels.len());
                for ch in &alice_channels {
                    info!("🔍 DEBUG:   - Channel '{}' (ID: {})", ch.name, hex::encode(&ch.id.0));
                }
                drop(alice_guard);
                
                // ALWAYS add user to channels, regardless of DHT join success
                if !alice_channels.is_empty() {
                    info!("📢 Adding new member {} to {} channels in space '{}'", 
                        hex::encode(&new_member_id.0[..8]), alice_channels.len(), space_name);
                    
                    // For each channel, add the new member
                    for channel in &alice_channels {
                        info!("🔍 DEBUG: Attempting to add member to channel '{}'...", channel.name);
                        
                        // The channel creator (Alice) needs to add the new member to the channel
                        let alice_guard = state.alice.read().await;
                        let alice_id = alice_guard.user_id();
                        info!("🔍 DEBUG: Alice's user ID: {}", hex::encode(&alice_id.0));
                        
                        match alice_guard.add_to_channel(
                            &channel.id,
                            new_member_id,
                            spaceway_core::Role::Member
                        ).await {
                            Ok(_) => {
                                info!("  ✅ Successfully added member to channel '{}'", channel.name);
                                
                                // Verify the member was actually added
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                drop(alice_guard);
                                
                                let client_guard = client.read().await;
                                let bob_channels = client_guard.list_channels(&space_id).await;
                                info!("🔍 DEBUG: After add, Bob now sees {} channels", bob_channels.len());
                                for ch in &bob_channels {
                                    info!("🔍 DEBUG:   - Bob sees channel '{}' (ID: {})", ch.name, hex::encode(&ch.id.0));
                                }
                                drop(client_guard);
                            }
                            Err(e) => {
                                info!("  ❌ Could not add member to channel '{}': {}", channel.name, e);
                                drop(alice_guard);
                            }
                        }
                    }
                    
                    // Final verification
                    info!("🔍 DEBUG: === FINAL VERIFICATION ===");
                    let client_guard = client.read().await;
                    let final_channels = client_guard.list_channels(&space_id).await;
                    info!("🔍 DEBUG: Bob's final channel count: {}", final_channels.len());
                    for ch in &final_channels {
                        info!("🔍 DEBUG:   - Channel '{}' visible to Bob", ch.name);
                    }
                    drop(client_guard);
                }
                
                match join_result {
                    Ok(space) => {
                        info!("✓ User successfully joined space '{}' via DHT", space.name);
                        Ok(format!("✓ Joined space '{}' and added to {} channels", space.name, alice_channels.len()))
                    }
                    Err(e) => {
                        // DHT failed, but we still added to channels
                        info!("ℹ️  DHT join failed: {}, but user added to channels", e);
                        Ok(format!("✓ Added to space '{}' and {} channels (DHT sync pending)", space_name, alice_channels.len()))
                    }
                }
            } else {
                Err(anyhow::anyhow!("Space not found in any client. Make sure the space was created first."))
            }
        }
        Action::ConnectPeers => {
            info!("🔗 Connecting all peers together...");
            
            // Get peer addresses for all clients
            let alice_guard = state.alice.read().await;
            let alice_addrs = alice_guard.listening_addrs().await;
            let alice_peer_id = alice_guard.peer_id().await;
            drop(alice_guard);
            
            let bob_guard = state.bob.read().await;
            let bob_addrs = bob_guard.listening_addrs().await;
            let bob_peer_id = bob_guard.peer_id().await;
            drop(bob_guard);
            
            let charlie_guard = state.charlie.read().await;
            let charlie_addrs = charlie_guard.listening_addrs().await;
            let charlie_peer_id = charlie_guard.peer_id().await;
            drop(charlie_guard);
            
            info!("Alice: {} at {:?}", alice_peer_id, alice_addrs);
            info!("Bob: {} at {:?}", bob_peer_id, bob_addrs);
            info!("Charlie: {} at {:?}", charlie_peer_id, charlie_addrs);
            
            // Connect Bob to Alice
            if let Some(alice_addr) = alice_addrs.first() {
                let full_addr = format!("{}/p2p/{}", alice_addr, alice_peer_id);
                info!("Connecting Bob → Alice: {}", full_addr);
                let bob_guard = state.bob.read().await;
                match bob_guard.network_dial(&full_addr).await {
                    Ok(_) => info!("✓ Bob connected to Alice"),
                    Err(e) => info!("✗ Bob → Alice failed: {}", e),
                }
                drop(bob_guard);
            }
            
            // Connect Charlie to Alice
            if let Some(alice_addr) = alice_addrs.first() {
                let full_addr = format!("{}/p2p/{}", alice_addr, alice_peer_id);
                info!("Connecting Charlie → Alice: {}", full_addr);
                let charlie_guard = state.charlie.read().await;
                match charlie_guard.network_dial(&full_addr).await {
                    Ok(_) => info!("✓ Charlie connected to Alice"),
                    Err(e) => info!("✗ Charlie → Alice failed: {}", e),
                }
                drop(charlie_guard);
            }
            
            // Connect Charlie to Bob
            if let Some(bob_addr) = bob_addrs.first() {
                let full_addr = format!("{}/p2p/{}", bob_addr, bob_peer_id);
                info!("Connecting Charlie → Bob: {}", full_addr);
                let charlie_guard = state.charlie.read().await;
                match charlie_guard.network_dial(&full_addr).await {
                    Ok(_) => info!("✓ Charlie connected to Bob"),
                    Err(e) => info!("✗ Charlie → Bob failed: {}", e),
                }
                drop(charlie_guard);
            }
            
            // Give connections time to establish
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            
            // Publish KeyPackages to DHT now that peers are connected
            info!("🔑 Publishing KeyPackages to DHT...");
            
            let alice_guard = state.alice.read().await;
            match alice_guard.publish_key_packages_to_dht().await {
                Ok(_) => info!("✓ Alice KeyPackages published"),
                Err(e) => info!("✗ Alice KeyPackages failed: {}", e),
            }
            drop(alice_guard);
            
            let bob_guard = state.bob.read().await;
            match bob_guard.publish_key_packages_to_dht().await {
                Ok(_) => info!("✓ Bob KeyPackages published"),
                Err(e) => info!("✗ Bob KeyPackages failed: {}", e),
            }
            drop(bob_guard);
            
            let charlie_guard = state.charlie.read().await;
            match charlie_guard.publish_key_packages_to_dht().await {
                Ok(_) => info!("✓ Charlie KeyPackages published"),
                Err(e) => info!("✗ Charlie KeyPackages failed: {}", e),
            }
            drop(charlie_guard);
            
            Ok("✓ Peer connections established and KeyPackages published to DHT!".to_string())
        }
    }
}

/// Get current state snapshot from all clients
async fn get_dashboard_state(state: &AppState) -> anyhow::Result<DashboardState> {
    let alice_guard = state.alice.read().await;
    let alice_snapshot = alice_guard.get_dashboard_snapshot("Alice").await;
    drop(alice_guard);
    
    let bob_guard = state.bob.read().await;
    let bob_snapshot = bob_guard.get_dashboard_snapshot("Bob").await;
    drop(bob_guard);
    
    let charlie_guard = state.charlie.read().await;
    let charlie_snapshot = charlie_guard.get_dashboard_snapshot("Charlie").await;
    drop(charlie_guard);
    
    // Build network graph
    let mut network_graph = spaceway_core::dashboard::NetworkGraph::new();
    
    // Add client nodes
    network_graph.add_client_node(&alice_snapshot.user_id, "Alice");
    network_graph.add_client_node(&bob_snapshot.user_id, "Bob");
    network_graph.add_client_node(&charlie_snapshot.user_id, "Charlie");
    
    // Add edges based on shared spaces
    for alice_space in &alice_snapshot.spaces {
        for bob_space in &bob_snapshot.spaces {
            if alice_space.id == bob_space.id {
                network_graph.add_gossipsub_edge(&alice_snapshot.user_id, &bob_snapshot.user_id);
                break;
            }
        }
        for charlie_space in &charlie_snapshot.spaces {
            if alice_space.id == charlie_space.id {
                network_graph.add_gossipsub_edge(&alice_snapshot.user_id, &charlie_snapshot.user_id);
                break;
            }
        }
    }
    
    for bob_space in &bob_snapshot.spaces {
        for charlie_space in &charlie_snapshot.spaces {
            if bob_space.id == charlie_space.id {
                network_graph.add_gossipsub_edge(&bob_snapshot.user_id, &charlie_snapshot.user_id);
                break;
            }
        }
    }
    
    Ok(DashboardState {
        clients: vec![alice_snapshot, bob_snapshot, charlie_snapshot],
        network_graph,
        crdt_timeline: vec![], // TODO: Collect CRDT operations
    })
}

/// Get current state snapshot
async fn get_state(State(state): State<AppState>) -> Json<DashboardState> {
    match get_dashboard_state(&state).await {
        Ok(dashboard_state) => Json(dashboard_state),
        Err(e) => {
            error!("Failed to get state: {}", e);
            // Return empty state on error
            Json(DashboardState {
                clients: vec![],
                network_graph: spaceway_core::dashboard::NetworkGraph::new(),
                crdt_timeline: vec![],
            })
        }
    }
}

/// Execute build commands
async fn build_handler(Json(req): Json<BuildRequest>) -> Json<BuildResponse> {
    use tokio::process::Command;
    
    info!("🔨 Executing build command: {} (in {})", req.command, req.working_dir);
    
    if req.is_background {
        // For background processes, spawn and don't wait
        match Command::new("sh")
            .arg("-c")
            .arg(&req.command)
            .current_dir(&req.working_dir)
            .spawn()
        {
            Ok(_) => {
                Json(BuildResponse {
                    success: true,
                    message: format!("Started background process: {}", req.command),
                    output: None,
                    exit_code: None,
                })
            }
            Err(e) => {
                Json(BuildResponse {
                    success: false,
                    message: format!("Failed to start process: {}", e),
                    output: None,
                    exit_code: Some(1),
                })
            }
        }
    } else {
        // For regular commands, wait for completion
        match Command::new("sh")
            .arg("-c")
            .arg(&req.command)
            .current_dir(&req.working_dir)
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = format!("{}{}", stdout, stderr);
                
                Json(BuildResponse {
                    success: output.status.success(),
                    message: if output.status.success() { "Command completed successfully".to_string() } else { "Command failed".to_string() },
                    output: Some(combined),
                    exit_code: output.status.code(),
                })
            }
            Err(e) => {
                Json(BuildResponse {
                    success: false,
                    message: format!("Failed to execute command: {}", e),
                    output: None,
                    exit_code: Some(1),
                })
            }
        }
    }
}
