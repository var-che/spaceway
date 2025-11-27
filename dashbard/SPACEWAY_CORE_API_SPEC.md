# Spaceway Core API Specification for Dashboard Integration

## Overview

This document specifies the public API that `spaceway-core`'s `Client` struct needs to expose for the dashboard to provide real-time visualization and control.

**Goal**: Enable the dashboard to:

1. Create and manage spaces, channels, and messages
2. Query client state (spaces, members, channels)
3. Handle invitations and membership
4. Observe network topology and CRDT operations

## Current Problem

The dashboard backend is in **mock mode** because `spaceway-core::Client` doesn't expose high-level operations. The dashboard needs:

```rust
// ❌ What we tried (doesn't exist):
client.create_space(name) -> Space
client.list_spaces() -> Vec<Space>
client.get_space(id) -> Space

// ✅ What we need (see below):
// Public async methods on Client
```

---

## Required API Methods

### 1. Client Initialization

```rust
impl Client {
    /// Create a new client with a generated keypair
    ///
    /// # Arguments
    /// * `name` - Human-readable name for the client (e.g., "Alice")
    /// * `config` - Client configuration (storage paths, network settings)
    ///
    /// # Returns
    /// A new Client instance ready to join the network
    pub async fn new(name: String, config: ClientConfig) -> Result<Self, ClientError>;

    /// Get the client's user ID (public key hex or similar identifier)
    pub fn user_id(&self) -> String;

    /// Get the client's display name
    pub fn name(&self) -> &str;
}
```

**Dashboard Usage**:

```rust
// In main.rs:
let alice = Client::new("Alice".to_string(), config_alice).await?;
let bob = Client::new("Bob".to_string(), config_bob).await?;
let charlie = Client::new("Charlie".to_string(), config_charlie).await?;
```

---

### 2. Space Management

```rust
impl Client {
    /// Create a new space owned by this client
    ///
    /// # Arguments
    /// * `name` - Display name for the space (e.g., "Dev Team")
    ///
    /// # Returns
    /// The space ID (UUID or similar unique identifier)
    pub async fn create_space(&self, name: String) -> Result<SpaceId, ClientError>;

    /// List all spaces this client is a member of
    ///
    /// # Returns
    /// Vector of space snapshots with basic info
    pub async fn list_spaces(&self) -> Result<Vec<SpaceSnapshot>, ClientError>;

    /// Get detailed information about a specific space
    ///
    /// # Arguments
    /// * `space_id` - The unique identifier for the space
    ///
    /// # Returns
    /// Complete space information including channels, members, roles
    pub async fn get_space(&self, space_id: &SpaceId) -> Result<SpaceSnapshot, ClientError>;

    /// Delete a space (owner only)
    ///
    /// # Arguments
    /// * `space_id` - The space to delete
    pub async fn delete_space(&self, space_id: &SpaceId) -> Result<(), ClientError>;
}
```

**Required Types**:

```rust
/// Unique identifier for a space (could be Uuid, String, or custom type)
pub type SpaceId = String; // or uuid::Uuid

/// Snapshot of a space's state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSnapshot {
    pub id: SpaceId,
    pub name: String,
    pub owner: String, // User ID of the owner
    pub members: Vec<MemberInfo>,
    pub channels: Vec<ChannelInfo>,
    pub roles: Vec<RoleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub user_id: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub joined_at: u64, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub priority: u32,
}
```

**Dashboard Usage**:

```rust
// Action: Create Space
let space_id = alice.create_space("Dev Team".to_string()).await?;

// Query state
let spaces = alice.list_spaces().await?;
let space_details = alice.get_space(&space_id).await?;
```

---

### 3. Channel Management

```rust
impl Client {
    /// Create a new channel in a space
    ///
    /// # Arguments
    /// * `space_id` - The space to create the channel in
    /// * `name` - Display name for the channel (e.g., "general")
    ///
    /// # Returns
    /// The channel ID
    pub async fn create_channel(
        &self,
        space_id: &SpaceId,
        name: String,
    ) -> Result<ChannelId, ClientError>;

    /// Get detailed information about a channel
    ///
    /// # Arguments
    /// * `channel_id` - The unique identifier for the channel
    pub async fn get_channel(&self, channel_id: &ChannelId) -> Result<ChannelSnapshot, ClientError>;

    /// Delete a channel (requires permission)
    ///
    /// # Arguments
    /// * `channel_id` - The channel to delete
    pub async fn delete_channel(&self, channel_id: &ChannelId) -> Result<(), ClientError>;
}
```

**Required Types**:

```rust
pub type ChannelId = String; // or uuid::Uuid

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: ChannelId,
    pub name: String,
    pub space_id: SpaceId,
    pub message_count: usize,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSnapshot {
    pub id: ChannelId,
    pub name: String,
    pub space_id: SpaceId,
    pub messages: Vec<MessageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: u64,
}
```

**Dashboard Usage**:

```rust
// Action: Create Channel
let channel_id = alice.create_channel(&space_id, "general".to_string()).await?;

// Query
let channel = alice.get_channel(&channel_id).await?;
```

---

### 4. Invitation System

```rust
impl Client {
    /// Generate an invite code for a space
    ///
    /// # Arguments
    /// * `space_id` - The space to create an invite for
    /// * `max_uses` - Optional limit on number of uses (None = unlimited)
    /// * `expires_at` - Optional expiration timestamp (None = never expires)
    ///
    /// # Returns
    /// A shareable invite code
    pub async fn create_invite(
        &self,
        space_id: &SpaceId,
        max_uses: Option<u32>,
        expires_at: Option<u64>,
    ) -> Result<String, ClientError>;

    /// Join a space using an invite code
    ///
    /// # Arguments
    /// * `invite_code` - The invite code received from another user
    ///
    /// # Returns
    /// The space ID that was joined
    pub async fn join_space(&self, invite_code: &str) -> Result<SpaceId, ClientError>;

    /// Leave a space
    ///
    /// # Arguments
    /// * `space_id` - The space to leave
    pub async fn leave_space(&self, space_id: &SpaceId) -> Result<(), ClientError>;
}
```

**Dashboard Usage**:

```rust
// Action: Create Invite (Alice invites Bob)
let invite_code = alice.create_invite(&space_id, None, None).await?;

// Action: Join Space (Bob accepts invite)
let joined_space_id = bob.join_space(&invite_code).await?;
```

---

### 5. Messaging

```rust
impl Client {
    /// Send a message to a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel to send the message to
    /// * `content` - The message text
    ///
    /// # Returns
    /// The message ID
    pub async fn send_message(
        &self,
        channel_id: &ChannelId,
        content: String,
    ) -> Result<String, ClientError>;

    /// Get recent messages from a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel to query
    /// * `limit` - Maximum number of messages to return
    ///
    /// # Returns
    /// Vector of messages (newest first)
    pub async fn get_messages(
        &self,
        channel_id: &ChannelId,
        limit: usize,
    ) -> Result<Vec<MessageInfo>, ClientError>;
}
```

**Dashboard Usage**:

```rust
// Action: Send Message
let msg_id = alice.send_message(&channel_id, "Hello world!".to_string()).await?;

// Query
let messages = alice.get_messages(&channel_id, 50).await?;
```

---

### 6. Network & DHT Inspection

```rust
impl Client {
    /// Get list of currently connected peers
    ///
    /// # Returns
    /// Vector of peer IDs (libp2p PeerIds as strings)
    pub async fn get_connected_peers(&self) -> Result<Vec<String>, ClientError>;

    /// Get DHT storage statistics
    ///
    /// # Returns
    /// Information about what this client is storing in the DHT
    pub async fn get_dht_storage(&self) -> Result<Vec<DhtEntry>, ClientError>;

    /// Get MLS group information for all spaces
    ///
    /// # Returns
    /// Vector of MLS group snapshots (one per space)
    pub async fn get_mls_groups(&self) -> Result<Vec<MlsGroupInfo>, ClientError>;
}
```

**Required Types**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtEntry {
    pub key: String,
    pub value_type: String, // "Space", "Channel", "Message", etc.
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlsGroupInfo {
    pub space_id: SpaceId,
    pub epoch: u64,
    pub member_count: usize,
    pub pending_proposals: usize,
}
```

**Dashboard Usage**:

```rust
// For visualization
let peers = alice.get_connected_peers().await?;
let dht_data = alice.get_dht_storage().await?;
let mls_groups = alice.get_mls_groups().await?;
```

---

### 7. CRDT Operation Timeline (Optional but Powerful)

```rust
impl Client {
    /// Subscribe to CRDT operation events
    ///
    /// # Returns
    /// A stream of CRDT operations as they occur
    pub fn subscribe_operations(&self) -> broadcast::Receiver<CrdtOperation>;
}
```

**Required Types**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtOperation {
    pub timestamp: u64,
    pub op_id: String,
    pub op_type: String, // "CreateSpace", "AddMember", "SendMessage", etc.
    pub author: String,
    pub space_id: Option<SpaceId>,
    pub channel_id: Option<ChannelId>,
    pub details: serde_json::Value, // Additional op-specific data
}
```

**Dashboard Usage**:

```rust
// Background task collects operations
let mut ops_rx = alice.subscribe_operations();
while let Ok(op) = ops_rx.recv().await {
    timeline.push(op);
}
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Space not found: {0}")]
    SpaceNotFound(SpaceId),

    #[error("Channel not found: {0}")]
    ChannelNotFound(ChannelId),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid invite code")]
    InvalidInvite,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("MLS error: {0}")]
    MlsError(String),
}
```

---

## Dashboard Integration Flow

Once these APIs are available, the dashboard will work like this:

### 1. Initialization (in `main.rs`)

```rust
// Create 3 clients
let alice = Client::new("Alice".to_string(), config_alice).await?;
let bob = Client::new("Bob".to_string(), config_bob).await?;
let charlie = Client::new("Charlie".to_string(), config_charlie).await?;

// Store in AppState
let state = AppState {
    alice: Arc::new(Mutex::new(alice)),
    bob: Arc::new(Mutex::new(bob)),
    charlie: Arc::new(Mutex::new(charlie)),
    state_snapshot: Arc::new(RwLock::new(DashboardState::default())),
};
```

### 2. Background State Polling

```rust
async fn update_state_loop(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_millis(500));

    loop {
        interval.tick().await;

        // Poll each client
        let alice = state.alice.lock().await;
        let bob = state.bob.lock().await;
        let charlie = state.charlie.lock().await;

        // Build snapshots
        let alice_snapshot = build_client_snapshot(&alice).await;
        let bob_snapshot = build_client_snapshot(&bob).await;
        let charlie_snapshot = build_client_snapshot(&charlie).await;

        drop(alice);
        drop(bob);
        drop(charlie);

        // Update shared state
        let mut snapshot = state.state_snapshot.write().await;
        snapshot.clients = vec![alice_snapshot, bob_snapshot, charlie_snapshot];
        snapshot.network_graph = build_network_graph(&snapshot.clients);
    }
}

async fn build_client_snapshot(client: &Client) -> ClientSnapshot {
    ClientSnapshot {
        name: client.name().to_string(),
        user_id: client.user_id(),
        spaces: client.list_spaces().await.unwrap_or_default()
            .into_iter()
            .map(|space| SpaceInfo {
                id: space.id,
                name: space.name,
                owner: space.owner,
                members: space.members,
                channels: space.channels.into_iter().map(|ch| ChannelInfo {
                    id: ch.id,
                    name: ch.name,
                    message_count: ch.message_count,
                }).collect(),
                role_count: space.roles.len(),
            })
            .collect(),
        dht_storage: client.get_dht_storage().await.unwrap_or_default(),
        mls_groups: client.get_mls_groups().await.unwrap_or_default(),
        connected_peers: client.get_connected_peers().await.unwrap_or_default(),
    }
}
```

### 3. Action Execution

```rust
async fn action_handler(
    State(state): State<AppState>,
    Json(request): Json<ActionRequest>,
) -> Json<serde_json::Value> {
    let client = match request.client.as_str() {
        "alice" => state.alice.lock().await,
        "bob" => state.bob.lock().await,
        "charlie" => state.charlie.lock().await,
        _ => return Json(json!({ "success": false, "error": "Unknown client" })),
    };

    let result = match request.action {
        Action::CreateSpace { name } => {
            match client.create_space(name.clone()).await {
                Ok(space_id) => format!("Created space '{}' with ID: {}", name, space_id),
                Err(e) => return Json(json!({ "success": false, "error": e.to_string() })),
            }
        }
        Action::CreateChannel { space_id, name } => {
            match client.create_channel(&space_id, name.clone()).await {
                Ok(channel_id) => format!("Created channel '{}' with ID: {}", name, channel_id),
                Err(e) => return Json(json!({ "success": false, "error": e.to_string() })),
            }
        }
        Action::CreateInvite { space_id } => {
            match client.create_invite(&space_id, None, None).await {
                Ok(invite) => format!("Invite code: {}", invite),
                Err(e) => return Json(json!({ "success": false, "error": e.to_string() })),
            }
        }
        Action::JoinSpace { invite_code } => {
            match client.join_space(&invite_code).await {
                Ok(space_id) => format!("Joined space: {}", space_id),
                Err(e) => return Json(json!({ "success": false, "error": e.to_string() })),
            }
        }
        Action::SendMessage { channel_id, content } => {
            match client.send_message(&channel_id, content).await {
                Ok(msg_id) => format!("Sent message: {}", msg_id),
                Err(e) => return Json(json!({ "success": false, "error": e.to_string() })),
            }
        }
    };

    Json(json!({ "success": true, "message": result }))
}
```

---

## Implementation Priority

### Phase 1 (Minimum Viable API):

1. ✅ `Client::new()` - Client initialization
2. ✅ `Client::user_id()` - Get client ID
3. ✅ `create_space()` - Space creation
4. ✅ `list_spaces()` - Query spaces
5. ✅ `get_space()` - Space details
6. ✅ `create_invite()` - Generate invites
7. ✅ `join_space()` - Accept invites

**Result**: Dashboard can visualize spaces and membership!

### Phase 2 (Full Feature Set):

8. ✅ `create_channel()` - Channel creation
9. ✅ `get_channel()` - Channel details
10. ✅ `send_message()` - Messaging
11. ✅ `get_messages()` - Message history
12. ✅ `get_connected_peers()` - Network topology
13. ✅ `get_dht_storage()` - DHT inspection

**Result**: Full dashboard functionality!

### Phase 3 (Advanced Features):

14. ✅ `subscribe_operations()` - CRDT timeline
15. ✅ `get_mls_groups()` - MLS inspection
16. ✅ Role/permission management APIs

**Result**: Professional-grade distributed system observatory!

---

## Testing the Integration

Once implemented, test with this workflow:

```bash
# Terminal 1: Start backend
cd dashboard-backend
cargo run

# Terminal 2: Start frontend
cd dashboard-frontend
npm run dev

# Browser: http://localhost:5173
```

Then execute:

1. **Alice creates space "Dev Team"** → See space appear in Alice's panel
2. **Alice creates channel "general"** → See channel under the space
3. **Alice creates invite** → Copy invite code
4. **Bob joins space** → See Bob appear in space members
5. **Network graph updates** → Shows Alice ↔ Bob connection
6. **Bob sends message** → See in channel message count
7. **CRDT timeline** → Shows all operations chronologically

---

## Benefits of This API Design

1. **Clean Separation**: Dashboard doesn't access internal Client state
2. **Type Safety**: Strong Rust types prevent errors
3. **Async/Await**: Non-blocking I/O for all operations
4. **Error Handling**: Explicit errors with context
5. **Extensible**: Easy to add new methods later
6. **Observable**: Dashboard can visualize everything
7. **Testable**: Each method can be unit tested

---

## Next Steps

1. **Review this spec** with the spaceway-core team
2. **Implement Phase 1 APIs** in `core/src/client.rs`
3. **Update dashboard backend** to use real Client instances
4. **Test integration** with the dashboard
5. **Iterate** based on what works/doesn't work

---

## Questions to Answer

Before implementing, clarify:

1. **ID Types**: Should we use `Uuid`, `String`, or custom types?
2. **Async Runtime**: Is `tokio` already used in spaceway-core?
3. **Serialization**: Is `serde` already a dependency?
4. **Error Types**: Should we use `thiserror` or `anyhow`?
5. **Internal APIs**: Do SpaceManager, ChannelManager already exist?
6. **Storage Layer**: How are spaces/channels currently stored?
7. **Network Layer**: Is libp2p already integrated?

---

## Contact

Questions or suggestions about this spec? Open an issue or discuss in:

- GitHub Issues
- Team chat
- Code review

Let's build an amazing dashboard! 🚀
