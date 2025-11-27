# Quick Implementation Guide

## TL;DR - What to Do

Add these methods to `spaceway-core/src/client.rs` to make the dashboard work with real data.

---

## Step 1: Add Public Methods to Client

```rust
// In core/src/client.rs

impl Client {
    // 🟢 ESSENTIAL - Do these first

    pub async fn create_space(&self, name: String) -> Result<String, String> {
        // Use your internal SpaceManager to create a space
        // Return the space ID
        todo!()
    }

    pub async fn list_spaces(&self) -> Result<Vec<SpaceSnapshot>, String> {
        // Query your internal state
        // Return all spaces this client is a member of
        todo!()
    }

    pub async fn get_space(&self, space_id: &str) -> Result<SpaceSnapshot, String> {
        // Look up a specific space
        // Return space with channels, members, roles
        todo!()
    }

    pub async fn create_invite(&self, space_id: &str) -> Result<String, String> {
        // Generate an invite code for the space
        // Return the invite code (could be a UUID or encoded data)
        todo!()
    }

    pub async fn join_space(&self, invite_code: &str) -> Result<String, String> {
        // Parse invite, join the space
        // Return the space ID that was joined
        todo!()
    }

    // 🟡 IMPORTANT - Add after essentials work

    pub async fn create_channel(&self, space_id: &str, name: String) -> Result<String, String> {
        todo!()
    }

    pub async fn send_message(&self, channel_id: &str, content: String) -> Result<String, String> {
        todo!()
    }

    // 🔵 NICE TO HAVE - For visualization

    pub async fn get_connected_peers(&self) -> Result<Vec<String>, String> {
        // If using libp2p:
        // self.swarm.connected_peers().map(|p| p.to_string()).collect()
        todo!()
    }

    pub async fn get_dht_storage(&self) -> Result<Vec<DhtEntry>, String> {
        // Return what this client is storing in the DHT
        todo!()
    }
}
```

---

## Step 2: Add Required Types

```rust
// In core/src/client.rs or core/src/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSnapshot {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub members: Vec<MemberInfo>,
    pub channels: Vec<ChannelInfo>,
    pub roles: Vec<RoleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub user_id: String,
    pub role: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtEntry {
    pub key: String,
    pub value_type: String,
    pub size_bytes: usize,
}
```

---

## Step 3: Update Dashboard Backend

Once the APIs exist in spaceway-core, update `dashboard-backend/Cargo.toml`:

```toml
[dependencies]
spaceway-core = { path = "../../core" }
# ... existing dependencies
```

Then update `dashboard-backend/src/main.rs`:

```rust
use spaceway_core::{Client, ClientConfig, SpaceSnapshot, /* etc */};

#[tokio::main]
async fn main() {
    // Replace mock clients with real ones
    let alice = Client::new("Alice".to_string(), alice_config).await.unwrap();
    let bob = Client::new("Bob".to_string(), bob_config).await.unwrap();
    let charlie = Client::new("Charlie".to_string(), charlie_config).await.unwrap();

    let state = AppState {
        alice: Arc::new(Mutex::new(alice)),
        bob: Arc::new(Mutex::new(bob)),
        charlie: Arc::new(Mutex::new(charlie)),
        state_snapshot: Arc::new(RwLock::new(DashboardState::default())),
    };

    // ... rest of setup
}

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
            // ✅ Use real API instead of mock
            match client.create_space(name.clone()).await {
                Ok(space_id) => format!("Created space '{}' with ID: {}", name, space_id),
                Err(e) => return Json(json!({ "success": false, "error": e })),
            }
        }
        // ... handle other actions
    };

    Json(json!({ "success": true, "message": result }))
}

async fn build_client_snapshot(client: &Client) -> ClientSnapshot {
    let spaces = client.list_spaces().await.unwrap_or_default();

    ClientSnapshot {
        name: client.name().to_string(),
        user_id: client.user_id(),
        spaces: spaces.into_iter().map(|space| SpaceInfo {
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
        }).collect(),
        dht_storage: client.get_dht_storage().await.unwrap_or_default(),
        mls_groups: vec![], // TODO: client.get_mls_groups().await
        connected_peers: client.get_connected_peers().await.unwrap_or_default(),
    }
}
```

---

## Step 4: Test Integration

```bash
# Terminal 1: Start backend
cd dashboard-backend
cargo run

# Should now say "REAL backend" instead of "MOCK backend"
```

Look for:

```
🚀 Starting Dashboard Backend
✅ Alice initialized: user_id=abc123...
✅ Bob initialized: user_id=def456...
✅ Charlie initialized: user_id=789xyz...
🎯 Dashboard backend listening on http://127.0.0.1:3030
```

```bash
# Terminal 2: Frontend (already running)
# Just refresh http://localhost:5173
```

---

## Testing Checklist

Once implemented, verify:

- [ ] **Create Space**: Alice creates "Dev Team" → Appears in Alice's panel
- [ ] **Network Graph**: Alice node appears with correct ID
- [ ] **Create Channel**: Alice creates "general" → Shows under space
- [ ] **Create Invite**: Alice generates invite → Copy invite code
- [ ] **Join Space**: Bob uses invite → Appears in space members
- [ ] **Network Connection**: Alice ↔ Bob edge appears in graph
- [ ] **Send Message**: Bob sends "Hello" → Message count increases
- [ ] **Multi-client**: Charlie joins → All 3 nodes in network graph
- [ ] **State Sync**: Changes propagate via WebSocket within 500ms

---

## Common Implementation Patterns

### Pattern 1: Using Existing Managers

If you already have SpaceManager, ChannelManager:

```rust
impl Client {
    pub async fn create_space(&self, name: String) -> Result<String, String> {
        let mut space_manager = self.space_manager.lock().await;
        let space_id = space_manager.create(name).await
            .map_err(|e| e.to_string())?;
        Ok(space_id.to_string())
    }

    pub async fn list_spaces(&self) -> Result<Vec<SpaceSnapshot>, String> {
        let space_manager = self.space_manager.lock().await;
        let spaces = space_manager.list_all();

        Ok(spaces.into_iter().map(|space| SpaceSnapshot {
            id: space.id.to_string(),
            name: space.name,
            owner: space.owner.to_string(),
            members: space.members.iter().map(|m| MemberInfo {
                user_id: m.user_id.to_string(),
                role: m.role.clone(),
                permissions: m.permissions.clone(),
            }).collect(),
            channels: space.channels.iter().map(|c| ChannelInfo {
                id: c.id.to_string(),
                name: c.name.clone(),
                message_count: c.message_count,
            }).collect(),
            roles: space.roles.clone(),
        }).collect())
    }
}
```

### Pattern 2: Using Storage Layer

If you have a storage abstraction:

```rust
impl Client {
    pub async fn get_space(&self, space_id: &str) -> Result<SpaceSnapshot, String> {
        let space = self.storage.get_space(space_id).await
            .ok_or_else(|| format!("Space not found: {}", space_id))?;

        Ok(SpaceSnapshot {
            id: space.id,
            name: space.name,
            owner: space.owner,
            members: self.storage.get_space_members(space_id).await?,
            channels: self.storage.get_space_channels(space_id).await?,
            roles: self.storage.get_space_roles(space_id).await?,
        })
    }
}
```

### Pattern 3: Using CRDT State

If your spaces are CRDTs:

```rust
impl Client {
    pub async fn create_space(&self, name: String) -> Result<String, String> {
        let space_id = uuid::Uuid::new_v4().to_string();

        // Create CRDT operation
        let op = CrdtOp::CreateSpace {
            id: space_id.clone(),
            name,
            owner: self.user_id(),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        // Apply locally and broadcast
        self.apply_operation(op).await?;
        self.broadcast_operation(op).await?;

        Ok(space_id)
    }
}
```

---

## Minimal Working Example

To get started quickly, here's the absolute minimum:

```rust
// core/src/client.rs

use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct Client {
    user_id: String,
    name: String,
    spaces: Arc<Mutex<HashMap<String, Space>>>, // Simple in-memory storage
}

impl Client {
    pub async fn create_space(&self, name: String) -> Result<String, String> {
        let space_id = uuid::Uuid::new_v4().to_string();
        let space = Space {
            id: space_id.clone(),
            name,
            owner: self.user_id.clone(),
            members: vec![],
            channels: vec![],
        };

        self.spaces.lock().await.insert(space_id.clone(), space);
        Ok(space_id)
    }

    pub async fn list_spaces(&self) -> Result<Vec<SpaceSnapshot>, String> {
        let spaces = self.spaces.lock().await;
        Ok(spaces.values().map(|s| s.to_snapshot()).collect())
    }
}

struct Space {
    id: String,
    name: String,
    owner: String,
    members: Vec<String>,
    channels: Vec<Channel>,
}

impl Space {
    fn to_snapshot(&self) -> SpaceSnapshot {
        SpaceSnapshot {
            id: self.id.clone(),
            name: self.name.clone(),
            owner: self.owner.clone(),
            members: self.members.iter().map(|uid| MemberInfo {
                user_id: uid.clone(),
                role: "member".to_string(),
                permissions: vec![],
            }).collect(),
            channels: self.channels.iter().map(|c| ChannelInfo {
                id: c.id.clone(),
                name: c.name.clone(),
                message_count: c.messages.len(),
            }).collect(),
            roles: vec![],
        }
    }
}
```

This gives you a working prototype that the dashboard can use immediately!

---

## Help & Support

Stuck? Check:

1. **Full Spec**: See `SPACEWAY_CORE_API_SPEC.md` for detailed API design
2. **Dashboard Code**: Look at `dashboard-backend/src/main.rs` for usage examples
3. **Type Definitions**: Check what the dashboard expects in each struct
4. **Test First**: Write unit tests for each method before integrating

---

## Success Criteria

You know it's working when:

1. ✅ `cargo build` succeeds in both `core` and `dashboard-backend`
2. ✅ Backend starts with "✅ Alice initialized: user_id=..."
3. ✅ Frontend shows "● Connected" (green)
4. ✅ Creating a space makes it appear in the panel
5. ✅ Network graph shows nodes and edges
6. ✅ CRDT timeline shows operations

**Good luck! 🚀**
