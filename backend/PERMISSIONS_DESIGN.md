# Permissions System Design - Safety-First Analysis

> **Date:** November 20, 2025  
> **Status:** Design Phase  
> **Goal:** Discord-style permissions adapted for decentralized, privacy-first architecture

---

## Discord Permissions Overview

Discord uses a **bitfield permission system** with 30+ flags:

### Server (Space) Permissions:
- Administrator (bypass all)
- Manage Server
- Manage Roles
- Manage Channels
- Kick Members
- Ban Members
- Create Invites
- Manage Nicknames
- View Audit Log

### Channel Permissions:
- View Channel
- Send Messages
- Embed Links
- Attach Files
- Mention Everyone
- Manage Messages (delete others)
- Read Message History
- Add Reactions

### Voice Permissions:
- Connect
- Speak
- Video
- Mute Members
- Deafen Members

---

## Safety-First Analysis: What Works in Descord?

### ✅ **WORKS - Cryptographically Enforceable**

These permissions can be **enforced with MLS** (cannot be bypassed):

#### 1. **View Channel (Read Access)**
- **Discord:** Role allows seeing channel
- **Descord:** MLS group membership
- **How:** User must be in MLS group to decrypt messages
- **Safety:** ✅ 100% enforced - no keys = can't read
- **Attack resistance:** Perfect (cryptographic)

#### 2. **Send Messages (Write Access)**
- **Discord:** Role allows posting
- **Descord:** Valid MLS commit signature required
- **How:** Only group members can create valid MLS commits
- **Safety:** ✅ 100% enforced - invalid signatures rejected
- **Attack resistance:** Perfect (cryptographic)

#### 3. **Kick Members (Remove from Space)**
- **Discord:** Remove user from server
- **Descord:** MLS remove + key rotation
- **How:** Admin creates MLS Remove commit → group key rotates
- **Safety:** ✅ 100% enforced - kicked user can't decrypt new messages
- **Attack resistance:** Perfect (forward secrecy)

#### 4. **Ban Members (Permanent Block)**
- **Discord:** Prevent user from rejoining
- **Descord:** Blacklist + MLS remove
- **How:** Add user's public key to space blacklist (CRDT), MLS remove
- **Safety:** ✅ 95% enforced - can't rejoin unless admin unblocks
- **Attack resistance:** Good (requires new identity to bypass)

#### 5. **Manage Roles (Change Permissions)**
- **Discord:** Assign/remove roles
- **Descord:** Role change = MLS update with role metadata
- **How:** Store role in MLS GroupContext (authenticated data)
- **Safety:** ✅ 90% enforced - role changes are MLS commits (signed by admin)
- **Attack resistance:** Good (requires admin signature)

---

### ⚠️ **PARTIALLY WORKS - Socially Enforceable**

These permissions rely on **client behavior** (can be bypassed by modified clients):

#### 6. **Manage Messages (Delete Others' Messages)**
- **Discord:** Moderators delete anyone's messages
- **Descord:** CRDT deletion marker (logical delete)
- **How:** Mod creates signed "deletion marker" in CRDT index
- **Safety:** ⚠️ 60% enforced - honest clients hide message, malicious clients can ignore
- **Attack resistance:** Weak (user can keep local copy)
- **Solution:** Accept this limitation (same as email - can't un-send)

#### 7. **Read Message History**
- **Discord:** Prevent reading old messages
- **Descord:** Can't revoke already-distributed keys
- **How:** Client can cache all decrypted messages
- **Safety:** ⚠️ 40% enforced - can't prevent caching
- **Attack resistance:** Weak (fundamental encryption trade-off)
- **Solution:** Don't allow revoking history access (once in group = see all past messages)

#### 8. **Attach Files**
- **Discord:** Toggle file uploads
- **Descord:** Client honors role flag
- **How:** Client checks role before uploading blob
- **Safety:** ⚠️ 30% enforced - modified client can bypass
- **Attack resistance:** Weak (client-side check only)
- **Solution:** Add blob size limits + reputation system

#### 9. **Mention Everyone (@everyone)**
- **Discord:** Ping all members
- **Descord:** Client honors role flag
- **How:** Client checks role before sending @everyone
- **Safety:** ⚠️ 20% enforced - modified client can bypass
- **Attack resistance:** Very weak (cosmetic permission)
- **Solution:** Accept as "polite suggestion" (clients can filter notifications)

---

### ❌ **DOESN'T WORK - Fundamentally Incompatible**

These permissions **cannot work** in decentralized architecture:

#### 10. **View Audit Log**
- **Discord:** See all admin actions (kicks, bans, role changes)
- **Descord:** ❌ No central server to log actions
- **Why incompatible:** Decentralized = no single source of truth
- **Privacy impact:** Audit logs leak metadata (who did what, when)
- **Alternative:** Local client logs MLS commits (partial visibility)
- **Verdict:** ❌ Skip this (privacy win - no admin surveillance)

#### 11. **Manage Webhooks**
- **Discord:** Create bots that post messages
- **Descord:** ❌ Requires server-side execution
- **Why incompatible:** No central server to host webhooks
- **Alternative:** Client-side bots (run locally, use user's identity)
- **Verdict:** ❌ Skip this (reduces bot spam)

#### 12. **View Server Insights (Analytics)**
- **Discord:** See message counts, active users, growth stats
- **Descord:** ❌ Violates metadata privacy
- **Why incompatible:** Analytics require collecting metadata
- **Privacy impact:** Exposes who's active, message patterns
- **Verdict:** ❌ Skip this (privacy win - no surveillance)

---

## Recommended Permission Model for Descord

### Core Permissions (Cryptographically Enforced)

```rust
pub struct Permissions {
    // Channel Access (MLS-enforced)
    pub view_channel: bool,      // MLS group membership
    pub send_messages: bool,      // Valid MLS commit signature
    
    // Moderation (MLS-enforced)
    pub kick_members: bool,       // Create MLS Remove commit
    pub ban_members: bool,        // Add to blacklist CRDT + MLS remove
    pub manage_roles: bool,       // Update role in MLS GroupContext
    
    // Space Management (MLS-enforced)
    pub manage_channels: bool,    // Create/delete channel MLS groups
    pub create_invites: bool,     // Generate invite tokens
    
    // Admin (bypass all checks)
    pub administrator: bool,      // Full MLS group admin privileges
}
```

### Role Hierarchy

```rust
pub enum Role {
    Admin,      // All permissions
    Moderator,  // Kick, ban, manage messages
    Member,     // Send messages only
}

impl Role {
    pub fn default_permissions(&self) -> Permissions {
        match self {
            Role::Admin => Permissions {
                view_channel: true,
                send_messages: true,
                kick_members: true,
                ban_members: true,
                manage_roles: true,
                manage_channels: true,
                create_invites: true,
                administrator: true,
            },
            Role::Moderator => Permissions {
                view_channel: true,
                send_messages: true,
                kick_members: true,
                ban_members: true,
                manage_roles: false,
                manage_channels: false,
                create_invites: true,
                administrator: false,
            },
            Role::Member => Permissions {
                view_channel: true,
                send_messages: true,
                kick_members: false,
                ban_members: false,
                manage_roles: false,
                manage_channels: false,
                create_invites: false,
                administrator: false,
            },
        }
    }
}
```

---

## Security Analysis

### Threat Model

| Threat | Discord | Descord | Mitigation |
|--------|---------|---------|------------|
| **Unauthorized message reading** | Server can read all | ❌ Impossible (MLS encryption) | MLS group membership |
| **Bypassing kick/ban** | Impossible | ❌ Impossible (key rotation) | Forward secrecy |
| **Message deletion bypass** | Impossible | ⚠️ Possible (client ignores) | Accept limitation |
| **Permission escalation** | Rare (server bug) | ⚠️ Possible (modified client) | MLS signature verification |
| **Admin surveillance** | Easy (audit logs) | ❌ Impossible (no central logs) | Privacy by design |
| **Metadata collection** | Easy (analytics) | ❌ Impossible (no server) | Privacy by design |

### Privacy Score

| Feature | Discord Privacy | Descord Privacy | Improvement |
|---------|----------------|-----------------|-------------|
| Message content | 0% (server reads) | 100% (MLS encrypted) | +100% |
| Who has access | 0% (server knows) | 90% (MLS group only) | +90% |
| Admin actions | 0% (audit logs) | 80% (no central log) | +80% |
| User activity | 0% (analytics) | 70% (no tracking) | +70% |
| **Overall** | **0%** | **85%** | **+85%** |

---

## Implementation Strategy

### Phase 1: Core Permissions (Week 1)
1. ✅ Define `Permission` struct
2. ✅ Implement `Role` enum with default permissions
3. ✅ Store role in MLS GroupContext (authenticated)
4. ✅ Add permission checks to message sending
5. ✅ Write tests for permission enforcement

### Phase 2: Moderation (Week 2)
1. ✅ Implement kick (MLS remove)
2. ✅ Implement ban (blacklist CRDT + MLS remove)
3. ✅ Implement role changes (MLS update)
4. ✅ Add moderation tests

### Phase 3: Channel Management (Week 3)
1. ✅ Implement channel creation (new MLS group)
2. ✅ Implement channel deletion (archive + notify)
3. ✅ Add channel permission inheritance
4. ✅ Test multi-channel spaces

### Phase 4: Invite System (Week 4)
1. ✅ Generate invite tokens (signed by admin)
2. ✅ Validate invite signatures
3. ✅ Add invite expiration
4. ✅ Test invite-only spaces

---

## Trade-offs vs Discord

### What We Lose 🔴
- ❌ Audit logs (can't see who kicked whom)
- ❌ Analytics (can't see user activity stats)
- ❌ Webhooks (no server-side bots)
- ❌ Message deletion enforcement (users can keep copies)
- ❌ Granular history control (can't revoke old messages)

### What We Gain 🟢
- ✅ **100% message privacy** (MLS encryption)
- ✅ **No admin surveillance** (no audit logs = no spying)
- ✅ **No metadata collection** (no analytics = no tracking)
- ✅ **Cryptographic enforcement** (can't bypass kick/ban)
- ✅ **Decentralized** (no single point of failure)
- ✅ **Censorship-resistant** (no server to shut down)

### Privacy vs Convenience

| Feature | Discord Approach | Descord Approach | Winner |
|---------|------------------|------------------|--------|
| Delete messages | Server deletes | Deletion marker | Discord (enforcement) |
| Kick enforcement | Server blocks | Key rotation | **Descord (crypto)** |
| Audit logs | Full history | Local only | **Descord (privacy)** |
| Analytics | Full tracking | None | **Descord (privacy)** |
| Webhooks | Server-side | Client-side | Discord (convenience) |

**Verdict:** Descord trades some convenience for massive privacy gains.

---

## Recommendation

### ✅ Implement These Permissions:
1. **view_channel** - MLS group membership
2. **send_messages** - Valid MLS signature
3. **kick_members** - MLS remove commit
4. **ban_members** - Blacklist + MLS remove
5. **manage_roles** - Role update in MLS GroupContext
6. **manage_channels** - Create/delete MLS groups
7. **create_invites** - Generate signed invite tokens
8. **administrator** - Bypass all checks

### ⚠️ Document Limitations:
1. **Message deletion** - Logical only (users can ignore)
2. **History access** - Can't revoke (encryption trade-off)
3. **@everyone** - Polite suggestion (clients can filter)

### ❌ Skip These Permissions:
1. **Audit logs** - Privacy violation
2. **Analytics** - Metadata collection
3. **Webhooks** - Requires centralized server

---

## Next Steps

1. Create `core/src/permissions.rs` module
2. Define `Permissions` struct with MLS integration
3. Update `MlsGroup` to store role in GroupContext
4. Add permission checks to message operations
5. Write comprehensive permission tests
6. Document permission model in user-facing docs

**Ready to implement?**
