# Permissions System Design

## Overview

A comprehensive permission system for Spaceway that handles:

1. **Space-level permissions** - Who can create channels, manage space, etc.
2. **Channel-level permissions** - Independent from space permissions
3. **Role-based access control** - Flexible role system
4. **MLS integration** - Permissions affect MLS group membership

## Current State

### Existing Roles (core/src/types.rs)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum Role {
    Admin,      // Full control
    Moderator,  // Can kick, manage content
    Member,     // Regular user
}

impl Role {
    pub fn is_admin(&self) -> bool { matches!(self, Role::Admin) }
    pub fn can_moderate(&self) -> bool { matches!(self, Role::Admin | Role::Moderator) }
}
```

**Issues:**

- ❌ Too simple - only 3 roles
- ❌ No granular permissions (can't give "create channels" without full admin)
- ❌ Space and channel use same Role enum (should be separate)
- ❌ No way to customize permissions per space/channel

---

## Proposed Design

### 1. Granular Permission System

#### Space Permissions

```rust
/// Space-level permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SpacePermissions {
    // Channel Management
    pub create_channels: bool,
    pub delete_channels: bool,
    pub manage_channels: bool,  // Edit name, description, settings

    // Member Management
    pub invite_members: bool,
    pub kick_members: bool,
    pub ban_members: bool,
    pub manage_roles: bool,     // Assign/remove roles

    // Content Management
    pub delete_messages: bool,
    pub pin_messages: bool,

    // Space Management
    pub manage_space: bool,     // Edit space name, description, settings
    pub view_audit_log: bool,

    // MLS Management
    pub manage_mls: bool,       // Force MLS mode, manage encryption
}

impl SpacePermissions {
    /// Administrator has all permissions
    pub fn admin() -> Self {
        Self {
            create_channels: true,
            delete_channels: true,
            manage_channels: true,
            invite_members: true,
            kick_members: true,
            ban_members: true,
            manage_roles: true,
            delete_messages: true,
            pin_messages: true,
            manage_space: true,
            view_audit_log: true,
            manage_mls: true,
        }
    }

    /// Moderator has content management permissions
    pub fn moderator() -> Self {
        Self {
            create_channels: true,   // Can create channels
            delete_channels: false,  // Cannot delete
            manage_channels: true,   // Can edit
            invite_members: true,
            kick_members: true,      // Can kick
            ban_members: false,      // Cannot ban (admin only)
            manage_roles: false,
            delete_messages: true,
            pin_messages: true,
            manage_space: false,
            view_audit_log: true,
            manage_mls: false,
        }
    }

    /// Regular member has minimal permissions
    pub fn member() -> Self {
        Self {
            create_channels: false,  // Configurable per space
            delete_channels: false,
            manage_channels: false,
            invite_members: true,    // Can create invites (if space allows)
            kick_members: false,
            ban_members: false,
            manage_roles: false,
            delete_messages: false,
            pin_messages: false,
            manage_space: false,
            view_audit_log: false,
            manage_mls: false,
        }
    }

    /// Custom role with specific permissions
    pub fn custom() -> Self {
        Self::member()  // Start with member defaults
    }
}
```

#### Channel Permissions

```rust
/// Channel-level permissions (independent from space permissions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ChannelPermissions {
    // Access Control
    pub view_channel: bool,      // Can see the channel exists
    pub read_messages: bool,     // Can read message history
    pub send_messages: bool,     // Can post messages

    // Content Management
    pub manage_messages: bool,   // Edit/delete own messages
    pub delete_others: bool,     // Delete others' messages
    pub create_threads: bool,
    pub manage_threads: bool,    // Archive, close threads

    // Channel Management
    pub manage_channel: bool,    // Edit channel settings
    pub invite_members: bool,    // Invite to this specific channel
    pub kick_members: bool,      // Remove from this channel only

    // Advanced
    pub mention_everyone: bool,  // @everyone mentions
    pub manage_webhooks: bool,
}

impl ChannelPermissions {
    pub fn admin() -> Self {
        Self {
            view_channel: true,
            read_messages: true,
            send_messages: true,
            manage_messages: true,
            delete_others: true,
            create_threads: true,
            manage_threads: true,
            manage_channel: true,
            invite_members: true,
            kick_members: true,
            mention_everyone: true,
            manage_webhooks: true,
        }
    }

    pub fn moderator() -> Self {
        Self {
            view_channel: true,
            read_messages: true,
            send_messages: true,
            manage_messages: true,
            delete_others: true,
            create_threads: true,
            manage_threads: true,
            manage_channel: false,
            invite_members: true,
            kick_members: true,
            mention_everyone: true,
            manage_webhooks: false,
        }
    }

    pub fn member() -> Self {
        Self {
            view_channel: true,
            read_messages: true,
            send_messages: true,
            manage_messages: true,   // Own messages
            delete_others: false,
            create_threads: true,
            manage_threads: false,
            manage_channel: false,
            invite_members: false,
            kick_members: false,
            mention_everyone: false,
            manage_webhooks: false,
        }
    }

    pub fn read_only() -> Self {
        Self {
            view_channel: true,
            read_messages: true,
            send_messages: false,  // Cannot send
            manage_messages: false,
            delete_others: false,
            create_threads: false,
            manage_threads: false,
            manage_channel: false,
            invite_members: false,
            kick_members: false,
            mention_everyone: false,
            manage_webhooks: false,
        }
    }
}
```

---

### 2. Role System Redesign

#### Separate Space and Channel Roles

```rust
/// Space-level role (who you are in the Space)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct SpaceRole {
    pub name: String,           // "Admin", "Moderator", "Member", "Helper", etc.
    pub permissions: SpacePermissions,
    pub color: Option<u32>,     // Display color (RGB)
    pub position: u32,          // Role hierarchy (higher = more powerful)
}

/// Channel-level role (who you are in a specific Channel)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ChannelRole {
    pub name: String,
    pub permissions: ChannelPermissions,
    pub color: Option<u32>,
}

impl SpaceRole {
    pub fn admin() -> Self {
        Self {
            name: "Admin".to_string(),
            permissions: SpacePermissions::admin(),
            color: Some(0xFF5555), // Red
            position: 1000,
        }
    }

    pub fn moderator() -> Self {
        Self {
            name: "Moderator".to_string(),
            permissions: SpacePermissions::moderator(),
            color: Some(0x5555FF), // Blue
            position: 500,
        }
    }

    pub fn member() -> Self {
        Self {
            name: "Member".to_string(),
            permissions: SpacePermissions::member(),
            color: None,
            position: 0,
        }
    }
}

impl ChannelRole {
    pub fn admin() -> Self {
        Self {
            name: "Admin".to_string(),
            permissions: ChannelPermissions::admin(),
            color: Some(0xFF5555),
        }
    }

    pub fn member() -> Self {
        Self {
            name: "Member".to_string(),
            permissions: ChannelPermissions::member(),
            color: None,
        }
    }
}
```

---

### 3. Updated Data Structures

#### Space

```rust
pub struct Space {
    pub id: SpaceId,
    pub name: String,
    pub description: Option<String>,
    pub owner: UserId,

    /// Member roles (user_id -> role_id)
    pub members: HashMap<UserId, RoleId>,

    /// Custom roles in this space
    pub roles: HashMap<RoleId, SpaceRole>,

    /// Default role for new members
    pub default_role: RoleId,

    /// Space-wide settings
    pub settings: SpaceSettings,

    // ...existing fields...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceSettings {
    /// Can members create channels? (if not, only mods/admins)
    pub members_can_create_channels: bool,

    /// Can members invite others?
    pub members_can_invite: bool,

    /// Require MLS for all channels in this space?
    pub require_channel_mls: bool,

    /// Default channel permissions for new channels
    pub default_channel_permissions: ChannelPermissions,
}

impl Space {
    /// Check if user has a specific permission
    pub fn has_permission(&self, user_id: &UserId, check: impl Fn(&SpacePermissions) -> bool) -> bool {
        if self.owner == *user_id {
            return true;  // Owner always has all permissions
        }

        if let Some(role_id) = self.members.get(user_id) {
            if let Some(role) = self.roles.get(role_id) {
                return check(&role.permissions);
            }
        }
        false
    }

    /// Can user create channels?
    pub fn can_create_channels(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.create_channels)
    }

    /// Can user kick members?
    pub fn can_kick_members(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.kick_members)
    }
}
```

#### Channel

```rust
pub struct Channel {
    pub id: ChannelId,
    pub space_id: SpaceId,
    pub name: String,
    pub description: Option<String>,
    pub creator: UserId,

    /// Channel-specific member roles (user_id -> role_id)
    /// Only populated if different from space membership
    pub members: HashMap<UserId, RoleId>,

    /// Custom roles for this channel
    pub roles: HashMap<RoleId, ChannelRole>,

    /// Default role for new channel members
    pub default_role: RoleId,

    /// Channel-specific settings
    pub settings: ChannelSettings,

    /// MLS group (always present - Phase 2)
    pub mls_group: Option<MlsGroup>,  // Will remove Option later

    // ...existing fields...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSettings {
    /// Who can access this channel?
    pub access_mode: ChannelAccessMode,

    /// Slow mode (minimum seconds between messages)
    pub slow_mode: Option<u32>,

    /// Is this a read-only announcement channel?
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelAccessMode {
    /// All space members can see and join
    Public,

    /// Space members can see, but need invite to join
    Private,

    /// Invisible to non-members (invite only)
    Secret,

    /// Based on space role (e.g., only Moderators+)
    RoleBased { min_position: u32 },
}

impl Channel {
    /// Check if user has channel permission
    pub fn has_permission(&self, user_id: &UserId, check: impl Fn(&ChannelPermissions) -> bool) -> bool {
        // Channel-specific role overrides space role
        if let Some(role_id) = self.members.get(user_id) {
            if let Some(role) = self.roles.get(role_id) {
                return check(&role.permissions);
            }
        }

        // Fall back to default channel permissions
        if let Some(role) = self.roles.get(&self.default_role) {
            return check(&role.permissions);
        }

        false
    }

    /// Can user send messages?
    pub fn can_send_messages(&self, user_id: &UserId) -> bool {
        if self.settings.read_only {
            return false;
        }
        self.has_permission(user_id, |p| p.send_messages)
    }

    /// Can user kick from this channel?
    pub fn can_kick_members(&self, user_id: &UserId) -> bool {
        self.has_permission(user_id, |p| p.kick_members)
    }
}
```

---

### 4. Permission Inheritance Model

**Key Principle:** Channel permissions are **independent** but can **inherit** from space

```
Space Role (Moderator)
├── Space Permissions: kick_members = true
├── Default Channel Permissions: kick_members = true
│
└── Channel "general" (inherits space)
    └── User gets: kick_members = true (inherited)

└── Channel "private" (custom override)
    └── User gets custom role: kick_members = false (channel-specific)
```

**Implementation:**

```rust
impl Channel {
    /// Get effective permissions for a user in this channel
    pub fn get_effective_permissions(
        &self,
        user_id: &UserId,
        space: &Space,
    ) -> ChannelPermissions {
        // 1. Check for channel-specific role override
        if let Some(channel_role_id) = self.members.get(user_id) {
            if let Some(role) = self.roles.get(channel_role_id) {
                return role.permissions;
            }
        }

        // 2. Check space role and convert to channel permissions
        if let Some(space_role_id) = space.members.get(user_id) {
            if let Some(space_role) = space.roles.get(space_role_id) {
                // Admins get full channel permissions
                if space_role.permissions.manage_space {
                    return ChannelPermissions::admin();
                }

                // Moderators get moderator channel permissions
                if space_role.permissions.kick_members {
                    return ChannelPermissions::moderator();
                }
            }
        }

        // 3. Fall back to default channel permissions
        self.roles.get(&self.default_role)
            .map(|r| r.permissions)
            .unwrap_or(ChannelPermissions::member())
    }
}
```

---

## 5. Use Cases

### Use Case 1: Create Channel

```rust
// User tries to create a channel
async fn create_channel(
    client: &Client,
    space_id: SpaceId,
    name: String,
) -> Result<Channel> {
    let space = client.get_space(&space_id).await?;

    // Check permission
    if !space.can_create_channels(&client.user_id) {
        return Err(Error::Permission("You don't have permission to create channels"));
    }

    // Create channel
    client.create_channel(space_id, name, None).await
}
```

### Use Case 2: Kick from Channel (Not Space)

```rust
// Kick user from a specific channel only
async fn kick_from_channel(
    client: &Client,
    channel_id: ChannelId,
    target_user: UserId,
) -> Result<()> {
    let channel = client.get_channel(&channel_id).await?;
    let space = client.get_space(&channel.space_id).await?;

    // Check channel-level permission (NOT space permission)
    if !channel.can_kick_members(&client.user_id) {
        return Err(Error::Permission("You cannot kick members from this channel"));
    }

    // Check role hierarchy (can't kick someone with higher role)
    let kicker_position = get_role_position(&space, &channel, &client.user_id);
    let target_position = get_role_position(&space, &channel, &target_user);

    if target_position >= kicker_position {
        return Err(Error::Permission("Cannot kick users with equal or higher roles"));
    }

    // Kick from channel MLS group only (Phase 2)
    client.kick_from_channel(&channel_id, &target_user).await?;

    // User is still in space, can still see other channels!
    Ok(())
}
```

### Use Case 3: Private Channel with Custom Permissions

```rust
// Create a private channel with custom role
async fn create_private_channel(
    client: &Client,
    space_id: SpaceId,
) -> Result<Channel> {
    let mut channel = client.create_channel(
        space_id,
        "private-team".to_string(),
        None,
    ).await?;

    // Make it private
    channel.settings.access_mode = ChannelAccessMode::Secret;

    // Create custom "Team Member" role for this channel
    let team_role = ChannelRole {
        name: "Team Member".to_string(),
        permissions: ChannelPermissions {
            view_channel: true,
            read_messages: true,
            send_messages: true,
            create_threads: true,
            // More permissive than regular members
            delete_others: true,  // Can delete in this channel
            ..ChannelPermissions::member()
        },
        color: Some(0x00FF00), // Green
    };

    let role_id = RoleId::new();
    channel.roles.insert(role_id, team_role);
    channel.default_role = role_id;

    Ok(channel)
}
```

---

## 6. Migration Strategy

### Phase 2.5: Permissions System

**Step 1: Add new permission types (non-breaking)**

```rust
// Add alongside existing Role enum
pub enum Role {
    Admin,
    Moderator,
    Member,
}

// New types
pub struct SpacePermissions { /* ... */ }
pub struct ChannelPermissions { /* ... */ }
pub struct SpaceRole { /* ... */ }
pub struct ChannelRole { /* ... */ }
```

**Step 2: Update Space and Channel structs**

```rust
pub struct Space {
    // Old (deprecated)
    pub members: HashMap<UserId, Role>,  // Keep for backwards compat

    // New
    pub member_roles: HashMap<UserId, RoleId>,
    pub roles: HashMap<RoleId, SpaceRole>,
}

impl Space {
    /// Migrate old Role to new SpaceRole
    pub fn migrate_roles(&mut self) {
        for (user_id, old_role) in &self.members {
            let space_role = match old_role {
                Role::Admin => SpaceRole::admin(),
                Role::Moderator => SpaceRole::moderator(),
                Role::Member => SpaceRole::member(),
            };

            let role_id = RoleId::new();
            self.roles.insert(role_id, space_role);
            self.member_roles.insert(*user_id, role_id);
        }
    }
}
```

**Step 3: Update APIs to use new permissions**

```rust
// Old API (deprecated)
pub fn can_kick(&self, user: &UserId) -> bool {
    matches!(self.members.get(user), Some(Role::Admin | Role::Moderator))
}

// New API
pub fn has_permission(&self, user: &UserId, check: impl Fn(&SpacePermissions) -> bool) -> bool {
    // Implementation from above
}
```

**Step 4: Remove old Role enum** (breaking change)

---

## 7. Security Considerations

### MLS Integration

**Problem:** Permissions affect who can be in MLS groups

```rust
// When user loses permission to view channel:
async fn on_permission_revoked(
    channel_id: &ChannelId,
    user_id: &UserId,
) -> Result<()> {
    // Must remove from channel MLS group!
    channel_manager.remove_member_with_mls(
        channel_id,
        user_id,
        &admin_id,
        &provider,
    ).await?;

    // Rotate keys so they can't decrypt future messages
    Ok(())
}
```

### Role Hierarchy

**Prevent privilege escalation:**

```rust
pub fn can_assign_role(
    &self,
    assigner_id: &UserId,
    target_id: &UserId,
    new_role_id: &RoleId,
) -> bool {
    let assigner_role = self.get_role(assigner_id)?;
    let new_role = self.roles.get(new_role_id)?;

    // Can't assign role higher than your own
    if new_role.position >= assigner_role.position {
        return false;
    }

    // Can't modify users with higher or equal role
    if let Some(target_role_id) = self.member_roles.get(target_id) {
        if let Some(target_role) = self.roles.get(target_role_id) {
            if target_role.position >= assigner_role.position {
                return false;
            }
        }
    }

    true
}
```

---

## 8. Comparison with Discord

| Feature                | Discord                    | Spaceway (Proposed)             |
| ---------------------- | -------------------------- | ------------------------------- |
| Space Roles            | Server-wide roles          | ✅ Same (SpaceRole)             |
| Channel Roles          | Role overrides per channel | ✅ Same (ChannelRole)           |
| Permission Bits        | 53 permission flags        | ✅ Simplified (12-15 per level) |
| Role Hierarchy         | Position-based             | ✅ Same                         |
| Permission Inheritance | Complex, many edge cases   | ✅ Simpler, explicit            |
| Default Permissions    | @everyone role             | ✅ default_role field           |
| Channel Categories     | Group channels             | ⏳ Future                       |

---

## 9. Implementation Priority

### Phase 2.5 (Next - Permissions Foundation)

1. ✅ **Add permission structs** (SpacePermissions, ChannelPermissions)
2. ✅ **Update Space struct** with roles HashMap
3. ✅ **Update Channel struct** with independent roles
4. ✅ **Migration helper** (Role → SpaceRole)
5. ✅ **Permission check methods** (has_permission, can_create_channels, etc.)

### Phase 3 (Future - Advanced Permissions)

6. ⏳ **Custom roles UI** (create, edit, delete roles)
7. ⏳ **Role assignment API** (assign role to user)
8. ⏳ **Permission overwrites** (per-channel role overrides)
9. ⏳ **Audit log** (track permission changes)
10. ⏳ **Role templates** (presets for common roles)

---

## 10. Recommendations

### For Your Project (Immediate)

**Option A: Simple (Recommended for MVP)**

- Keep current Role enum for now
- Add `members_can_create_channels: bool` to Space settings
- Add `can_kick: bool` to Channel (independent from space kick)
- Phase 2 complete → Focus on stability first

**Option B: Full Permissions (Better long-term)**

- Implement SpacePermissions + ChannelPermissions now
- Migrate existing roles
- More work upfront, but cleaner architecture

### My Recommendation

**Do Option A first**, then gradually add Option B:

1. **Now (Phase 2.5a):**

   ```rust
   pub struct SpaceSettings {
       pub members_can_create_channels: bool,
       pub members_can_invite: bool,
   }

   pub struct Channel {
       // Independent kick permission
       pub moderators: HashSet<UserId>,  // Can kick from THIS channel
   }
   ```

2. **Later (Phase 3):**
   - Full permission system
   - Custom roles
   - Fine-grained control

This gives you immediate flexibility without over-engineering early.

---

## Questions for You

1. **Complexity vs Features:** Start simple (Option A) or go full permissions now (Option B)?

2. **Channel Independence:** Should channel mods (can kick from channel) also need space moderator role? Or fully independent?

3. **Role Hierarchy:** Important for v1? Or can defer to later?

4. **Private Channels:** High priority? Affects MLS auto-join logic

5. **Removing Moderator Role:** Keep 3 roles (Admin/Mod/Member) or allow custom roles from start?

Let me know your preferences and I can help implement the chosen approach!
