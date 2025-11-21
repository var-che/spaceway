# Descord Feature Roadmap - Privacy-Focused Discord Alternative

## 📦 Current Version: 0.1.0 (Beta)

**Release**: November 21, 2025  
**Status**: Beta - Ready for small group testing  
**Network**: 2+ peer P2P sync working, DHT optional

### Version History
- **0.1.0** (Nov 21, 2025): Initial beta with P2P sync protocol
  - GossipSub real-time messaging
  - Peer-to-peer historical data sync
  - Space invites with 8-char codes
  - CLI application with network commands
  - Dual-index storage for fast queries

---

## Current Architecture ✅

**Data Model**: Space → Channels → Threads → Messages (hierarchical structure)
- ✅ Spaces (like Discord servers/guilds)
- ✅ Channels (like Discord channels)
- ✅ Threads (topic-based discussions)
- ✅ Messages (content)
- ✅ Basic roles (Admin, Moderator, Member)
- ✅ CRDT-based synchronization
- ✅ E2E encryption via MLS
- ✅ P2P networking via libp2p
- ✅ **Real-time GossipSub messaging**
- ✅ **P2P sync protocol (SYNC_REQUEST)**
- ✅ **2-peer network support**

---

## Critical Missing Features for Discord Parity

### 1. **Access Control & Privacy** 🔐 [HIGH PRIORITY]

#### 1.1 Space Visibility & Discovery ✅ **COMPLETED**
**Status**: Fully implemented with SpaceVisibility enum
**Implementation**:
- ✅ **Public Spaces**: Globally discoverable via DHT/directory
  - Listed in a public directory
  - Anyone can join without invite
  - Search by name/tags
  
- ✅ **Private Spaces**: Invite-only, not discoverable
  - Not listed anywhere publicly
  - Requires invite link/code
  - SpaceId not published to DHT
  
- ✅ **Hidden Spaces**: Maximum privacy
  - Not discoverable even if you have the ID
  - Requires out-of-band invite with cryptographic proof
  - No metadata leaks

**Test Coverage**: 6 tests passing

**Implementation Approach**:
```rust
pub enum SpaceVisibility {
    Public,        // Listed in public directory, anyone can join
    Private,       // Invite-only, not listed
    Hidden,        // Requires cryptographic invitation
}

pub struct Space {
    // ... existing fields ...
    pub visibility: SpaceVisibility,
    pub invite_code: Option<String>,  // For private spaces
    pub discoverable: bool,            // DHT publication flag
}
```

**Technical Details**:
- Public: Publish space metadata (encrypted) to DHT under `global/spaces/{hash}`
- Private: Only share SpaceId via invite links, not published
- Hidden: Require HMAC-based invitation tokens that prove knowledge of shared secret

---

#### 1.2 Invite System ✅ **COMPLETED**
**Status**: Fully functional with 8-character codes, expiration, and permissions
**Implementation**:
- ✅ **Invite Links**: One-time or permanent invite URLs
- ✅ **Invite Codes**: Short alphanumeric codes (8 characters)
- ✅ **Invite Permissions**: Who can create invites (admins only, moderators, everyone)
- ✅ **Invite Expiration**: Time-based or use-count limits
- ✅ **Invite Revocation**: Ability to revoke active invites
- ✅ **Auto-join**: Using invite automatically adds member to space

**Test Coverage**: 11 tests passing

**Implementation**:
```rust
pub struct Invite {
    pub id: InviteId,
    pub space_id: SpaceId,
    pub creator: UserId,
    pub code: String,              // "ABcd123X" format
    pub max_uses: Option<u32>,     // None = unlimited
    pub expires_at: Option<u64>,   // Unix timestamp
    pub uses: u32,                 // Current use count
    pub created_at: u64,
}

pub struct InvitePermissions {
    pub who_can_invite: InviteCreatorRole,  // Admin | Moderator | Everyone
    pub max_age_hours: Option<u32>,
    pub max_uses_default: u32,
}

pub enum InviteCreatorRole {
    AdminOnly,
    AdminAndModerator,
    Everyone,
}
```

---

### 1.3 Networking & Synchronization ✅ **MOSTLY COMPLETE**
**Status**: Core P2P working, needs optimization for scale

#### Completed ✅
- ✅ **GossipSub**: Real-time pub/sub messaging
  - Topic-based routing (per-Space topics)
  - Message signing and verification
  - 2-peer network support (mesh_n=2, mesh_n_low=1)
  - Explicit peering for small networks
  
- ✅ **P2P Sync Protocol**: Historical data sync
  - SYNC_REQUEST/response via GossipSub
  - Automatic operation re-broadcast
  - Works without DHT (2+ peers)
  
- ✅ **Dual Storage Index**:
  - Operations indexed by op_id (deduplication)
  - Operations indexed by space_id (fast queries)
  
- ✅ **Connection Management**:
  - Configurable listen addresses (--port)
  - Bootstrap peers (--bootstrap)
  - Direct peer dialing (/ip4/IP/tcp/PORT/p2p/PEER_ID)
  - Connection events (PeerConnected/Disconnected)

#### In Progress 🔨
- ⏳ **DHT Optimization**: Currently requires 3+ peers for quorum
  - Need to make DHT work better with 2 peers
  - Or make DHT fully optional (use P2P sync as primary)
  
- ⏳ **Relay Support**: Circuit Relay v2 for NAT traversal
  - Infrastructure in place but needs testing
  - Need public relay servers

#### Next Steps 📋
- [ ] **Connection Quality**: Track peer latency, message success rates
- [ ] **Bandwidth Management**: Rate limiting, message size limits
- [ ] **Peer Discovery**: Better mechanisms beyond bootstrap
- [ ] **Network Resilience**: Automatic reconnection, failover
- [ ] **Metrics Dashboard**: Network health monitoring

**Priority**: LOW - Networking works well enough for beta testing

---

### 2. **Permissions System** 🛡️ [HIGH PRIORITY]

**Current State**: Basic roles (Admin, Moderator, Member) with simple can_moderate() check

**Needed**: Granular permissions like Discord

#### 2.1 Space-Level Permissions
```rust
pub struct SpacePermissions {
    // General
    pub view_channels: bool,
    pub manage_channels: bool,
    pub manage_roles: bool,
    pub manage_space: bool,        // Edit name, description
    pub kick_members: bool,
    pub ban_members: bool,
    pub create_invites: bool,
    
    // Communication
    pub send_messages: bool,
    pub send_attachments: bool,
    pub mention_everyone: bool,
    pub add_reactions: bool,
    
    // Moderation
    pub manage_messages: bool,      // Delete others' messages
    pub manage_threads: bool,
    pub moderate_members: bool,     // Timeout, mute
}
```

#### 2.2 Channel-Level Permission Overrides
```rust
pub struct ChannelPermissionOverride {
    pub role_or_user: PermissionTarget,
    pub allow: PermissionSet,       // Explicitly allowed
    pub deny: PermissionSet,        // Explicitly denied
}

pub enum PermissionTarget {
    Role(Role),
    User(UserId),
}
```

#### 2.3 Role Hierarchy
```rust
pub struct RoleDefinition {
    pub id: RoleId,
    pub name: String,
    pub color: Option<String>,      // Hex color #FF5733
    pub position: u32,              // Higher = more authority
    pub permissions: SpacePermissions,
    pub mentionable: bool,
}

// Multiple roles per user, sorted by position
pub struct MemberRoles {
    pub user_id: UserId,
    pub roles: Vec<RoleId>,
}
```

**Implementation Strategy**:
- Each role has a position (0-999), higher position = higher authority
- Permission resolution: User permissions = Union of all role permissions
- Channel overrides: Deny > Allow > Role permissions
- Store in CRDT: Role changes are operations that need consensus

---

### 3. **Direct Messages & Group DMs** 💬 [HIGH PRIORITY]

**Current State**: None - all communication is space-based

**Needed**:
- [ ] **1-on-1 Direct Messages**: Private encrypted chat between two users
- [ ] **Group DMs**: Private group (2-10 people) without full space overhead
- [ ] **DM Privacy**: No metadata leaks, ephemeral keys

**Implementation**:
```rust
pub struct DirectMessage {
    pub id: DMId,
    pub participants: Vec<UserId>,  // 2 for DM, 2-10 for group DM
    pub mls_group: MlsGroup,        // Separate MLS group
    pub created_at: u64,
}

// DMs are NOT part of spaces
// Stored separately in client state
pub struct ClientDMManager {
    dms: HashMap<DMId, DirectMessage>,
    messages: HashMap<DMId, Vec<Message>>,
}
```

**Privacy Considerations**:
- Each DM is a separate MLS group (not tied to any space)
- DM discovery: Only via direct peer connection, not DHT
- Message transport: Direct peer-to-peer when online, relay when offline
- No "DM list" published anywhere - purely local state

---

### 4. **Voice & Video** 🎙️ [MEDIUM PRIORITY]

**Current State**: Text-only

**Needed**:
- [ ] **Voice Channels**: Real-time voice communication
- [ ] **Video Calls**: 1-on-1 and group video
- [ ] **Screen Sharing**: Desktop/window sharing
- [ ] **E2E Encrypted Audio/Video**: Using WebRTC with DTLS-SRTP

**Implementation Approach**:
```rust
pub enum ChannelType {
    Text,
    Voice,
    Forum,      // Thread-based like current implementation
}

pub struct VoiceChannel {
    pub id: ChannelId,
    pub space_id: SpaceId,
    pub channel_type: ChannelType,
    pub bitrate: u32,           // Audio quality
    pub user_limit: Option<u32>, // Max participants
    pub rtc_region: Option<String>,
}

pub struct VoiceState {
    pub user_id: UserId,
    pub channel_id: ChannelId,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,      // Moderator muted
    pub server_deaf: bool,
}
```

**Technical Stack**:
- **WebRTC** for peer-to-peer media
- **Selective Forwarding Unit (SFU)** for group calls (decentralized relay nodes)
- **Opus** codec for audio
- **VP9/AV1** codec for video
- **DTLS-SRTP** for E2E encryption

**Decentralized Voice Architecture**:
- No central voice server
- Mesh network for small groups (< 5 people)
- SFU relay for larger groups (volunteers act as relays)
- End-to-end encrypted streams (relay can't decrypt)

---

### 5. **Rich Media & Attachments** 📎 [MEDIUM PRIORITY]

**Current State**: Blob storage exists but limited

**Needed**:
- [ ] **File Uploads**: Images, videos, documents
- [ ] **File Size Limits**: Per-role configurable
- [ ] **Image Previews**: Thumbnails, inline rendering
- [ ] **Link Previews**: Open Graph metadata
- [ ] **Emoji System**: Custom emoji per space
- [ ] **Reactions**: React to messages with emoji
- [ ] **Stickers**: Animated/static stickers

**Implementation**:
```rust
pub struct Attachment {
    pub id: AttachmentId,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub url: String,            // IPFS CID or blob hash
    pub thumbnail: Option<String>,
    pub width: Option<u32>,     // For images/videos
    pub height: Option<u32>,
}

pub struct Message {
    // ... existing fields ...
    pub attachments: Vec<Attachment>,
    pub embeds: Vec<Embed>,
    pub reactions: HashMap<String, Vec<UserId>>, // emoji -> users
}

pub struct CustomEmoji {
    pub id: EmojiId,
    pub space_id: SpaceId,
    pub name: String,           // :customname:
    pub image_hash: BlobHash,
    pub animated: bool,
}
```

---

### 6. **Moderation Tools** 🔨 [HIGH PRIORITY]

**Current State**: Basic member add/remove

**Needed**:
- [ ] **Message Deletion**: Moderators delete messages
- [ ] **Message Editing History**: Track edits for moderation
- [ ] **User Timeout/Mute**: Temporary communication ban
- [ ] **User Ban**: Permanent removal with optional expiry
- [ ] **Audit Logs**: Track all moderation actions
- [ ] **Automod**: Automated content filtering
- [ ] **Report System**: Users report violations

**Implementation**:
```rust
pub struct Timeout {
    pub user_id: UserId,
    pub space_id: SpaceId,
    pub until: u64,             // Unix timestamp
    pub reason: Option<String>,
    pub moderator: UserId,
}

pub struct Ban {
    pub user_id: UserId,
    pub space_id: SpaceId,
    pub reason: Option<String>,
    pub moderator: UserId,
    pub expires_at: Option<u64>, // None = permanent
    pub delete_messages: bool,   // Delete user's message history
}

pub struct AuditLogEntry {
    pub id: AuditId,
    pub action: AuditAction,
    pub actor: UserId,
    pub target: Option<UserId>,
    pub reason: Option<String>,
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

pub enum AuditAction {
    ChannelCreate,
    ChannelDelete,
    MemberKick,
    MemberBan,
    MemberUnban,
    MessageDelete,
    RoleCreate,
    RoleUpdate,
    // ... etc
}
```

**Privacy Concern**: Audit logs expose metadata
- **Solution**: Encrypt audit logs, only visible to admins
- **CRDT Challenge**: Deletion is "tombstone" not true deletion
- **Approach**: Messages marked as deleted, content zeroed, only hash kept

---

### 7. **Presence & Status** 👤 [LOW PRIORITY]

**Current State**: None

**Needed**:
- [ ] **Online/Offline Status**: User availability
- [ ] **Custom Status**: "Playing XYZ", "Studying", etc.
- [ ] **Do Not Disturb**: Suppress notifications
- [ ] **Invisible Mode**: Appear offline while online

**Privacy Tradeoff**:
- **Option A**: Presence broadcast via gossipsub (metadata leak)
- **Option B**: No global presence, only per-space presence
- **Option C**: Presence only shared with mutual contacts

**Implementation** (Option B - Privacy-First):
```rust
pub struct UserPresence {
    pub user_id: UserId,
    pub space_id: SpaceId,       // Per-space presence
    pub status: PresenceStatus,
    pub custom_status: Option<String>,
    pub last_seen: u64,
}

pub enum PresenceStatus {
    Online,
    Idle,       // No activity for 10+ minutes
    DoNotDisturb,
    Invisible,   // Appears offline to others
    Offline,
}
```

---

### 8. **Notifications** 🔔 [MEDIUM PRIORITY]

**Current State**: None

**Needed**:
- [ ] **Desktop Notifications**: System notifications
- [ ] **Push Notifications**: Mobile push (privacy-preserving)
- [ ] **Mentions**: @user, @role, @everyone
- [ ] **Notification Settings**: Per-channel, per-space granularity
- [ ] **Unread Indicators**: Unread message counts

**Privacy Challenge**: Push notifications require centralized service
**Solution**: Use privacy-preserving push relay (UnifiedPush protocol)

```rust
pub struct NotificationSettings {
    pub space_id: SpaceId,
    pub mentions: NotificationLevel,
    pub all_messages: NotificationLevel,
    pub muted_channels: Vec<ChannelId>,
}

pub enum NotificationLevel {
    All,
    MentionsOnly,
    Nothing,
}

pub enum MentionType {
    User(UserId),
    Role(RoleId),
    Everyone,       // @everyone - requires permission
    Here,           // @here - only online users
}
```

---

### 9. **Search & Discovery** 🔍 [LOW PRIORITY]

**Current State**: None

**Needed**:
- [ ] **Message Search**: Full-text search within channels
- [ ] **Space Discovery**: Browse public spaces
- [ ] **Tag System**: Categorize spaces
- [ ] **Search Privacy**: Encrypted search indices

**Implementation**:
```rust
// Local encrypted search index
pub struct SearchIndex {
    // Inverted index: term -> message IDs
    index: HashMap<String, Vec<MessageId>>,
}

// Public space directory (optional feature)
pub struct SpaceDirectory {
    pub spaces: Vec<PublicSpaceInfo>,
}

pub struct PublicSpaceInfo {
    pub id: SpaceId,
    pub name: String,
    pub description: Option<String>,
    pub member_count: u32,
    pub tags: Vec<String>,
    pub icon_hash: Option<BlobHash>,
}
```

---

### 10. **Multi-Device Sync** 📱 [HIGH PRIORITY]

**Current State**: Single device per account

**Needed**:
- [ ] **Multiple Devices**: Desktop + Mobile + Web
- [ ] **E2E Encrypted Sync**: Message history across devices
- [ ] **Device Management**: List and revoke devices
- [ ] **QR Code Pairing**: Easy device linking

**Implementation** (MLS Multi-Device):
```rust
pub struct DeviceIdentity {
    pub device_id: DeviceId,
    pub user_id: UserId,
    pub device_key: PublicKey,
    pub device_name: String,     // "Alice's iPhone"
    pub added_at: u64,
}

// Each device is separate MLS client in same groups
// User = Collection of devices
pub struct UserDevices {
    pub user_id: UserId,
    pub devices: Vec<DeviceIdentity>,
    pub primary_device: DeviceId,
}
```

**Sync Protocol**:
- Each device maintains full state
- Delta sync for efficiency (only recent changes)
- Conflict resolution via CRDT
- Device addition requires existing device approval (TOFU)

---

### 11. **Threading & Message Organization** 🧵 [MEDIUM PRIORITY]

**Current State**: Basic threads exist

**Needed**:
- [ ] **Thread Replies**: Inline reply threads (like Slack/Discord)
- [ ] **Pinned Messages**: Pin important messages to top
- [ ] **Message Bookmarks**: Personal message saves
- [ ] **Thread Notifications**: Follow/unfollow threads

```rust
pub struct MessageThread {
    pub parent_message_id: MessageId,
    pub replies: Vec<MessageId>,
    pub participant_count: u32,
}

pub struct PinnedMessage {
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub pinned_by: UserId,
    pub pinned_at: u64,
}

pub struct Bookmark {
    pub user_id: UserId,
    pub message_id: MessageId,
    pub bookmarked_at: u64,
}
```

---

### 12. **Offline & Sync** ⚡ [HIGH PRIORITY]

**Current State**: Basic CRDT convergence

**Needed**:
- [ ] **Offline Message Queue**: Send when reconnected
- [ ] **Background Sync**: Sync while app backgrounded
- [ ] **Conflict Resolution UI**: Show conflicts to user
- [ ] **Selective Sync**: Don't sync all spaces (bandwidth)

```rust
pub struct SyncStrategy {
    pub full_sync_spaces: Vec<SpaceId>,      // Always sync everything
    pub partial_sync_spaces: Vec<SpaceId>,   // Only recent messages
    pub archived_spaces: Vec<SpaceId>,       // Don't sync unless opened
}

pub struct OfflineQueue {
    pub pending_messages: Vec<Message>,
    pub pending_operations: Vec<CrdtOp>,
}
```

---

## 🎯 Implementation Priority (Updated Nov 21, 2025)

### Phase 0: Foundation ✅ **COMPLETED**
1. ✅ **Core Data Model** - Space/Channel/Thread/Message hierarchy
2. ✅ **CRDT Operations** - Conflict-free state synchronization
3. ✅ **P2P Networking** - libp2p with GossipSub + Kademlia DHT
4. ✅ **Storage Layer** - RocksDB with dual indexing
5. ✅ **E2E Encryption** - MLS (Message Layer Security)
6. ✅ **CLI Application** - Working terminal interface
7. ✅ **Space Visibility** - Public/Private/Hidden modes
8. ✅ **Invite System** - 8-char codes with expiration
9. ✅ **P2P Sync Protocol** - Historical data sync via SYNC_REQUEST
10. ✅ **2-Peer Networks** - GossipSub optimized for small groups

**Current State**: v0.1.0 - Beta ready for small group testing (2-10 users)

---

### Phase 1 (MVP+): Essential Features for Beta Testing
**Target**: v0.2.0 - Public Beta
**Timeline**: Next 2-4 weeks

**Blockers for Public Beta**:
1. 🔴 **Message Persistence & History** [CRITICAL]
   - [ ] Message pagination (load older messages)
   - [ ] Message search within channels
   - [ ] Unread message tracking
   - **Why Critical**: Users can't scroll back through history
   - **Effort**: 3-5 days

2. 🔴 **Basic Notifications** [CRITICAL]
   - [ ] Desktop notifications for new messages
   - [ ] @mention detection and alerts
   - [ ] Unread badges per channel/space
   - **Why Critical**: Users miss messages when app is backgrounded
   - **Effort**: 2-3 days

3. 🟡 **User Profiles** [HIGH]
   - [ ] Display names (separate from username)
   - [ ] User avatars (profile pictures)
   - [ ] User status (online/offline/away)
   - [ ] "About me" / bio
   - **Why Important**: Basic social features expected by users
   - **Effort**: 3-4 days

4. 🟡 **Direct Messages** [HIGH]
   - [ ] 1-on-1 DMs
   - [ ] Group DMs (2-10 people)
   - [ ] DM list UI
   - **Why Important**: Core Discord feature, users expect it
   - **Effort**: 5-7 days

5. 🟢 **Permissions Refinement** [MEDIUM]
   - [ ] Role-based permissions (beyond Admin/Mod/Member)
   - [ ] Channel permission overrides
   - [ ] "View-only" channels
   - **Why Important**: Necessary for larger communities
   - **Effort**: 4-5 days

**Total Estimated Effort**: 17-28 days (3-6 weeks for one developer)

---

### Phase 2: Rich Communication
**Target**: v0.3.0 - Feature Parity
**Timeline**: 1-2 months after v0.2.0
6. Voice channels (text-to-voice first)
7. File attachments & media
8. Custom emoji & reactions
9. Message threading & organization
10. Multi-device sync

### Phase 3: Discovery & Scale
11. Space discovery (public directory)
12. Search functionality
13. Audit logs & advanced moderation
14. Push notifications
15. Presence system

### Phase 4: Advanced Features
16. Video calls & screen sharing
17. Bots & integrations
18. Webhooks
19. API for third-party clients

---

## CRDT Challenges & Solutions

### Challenge 1: Permission Changes
**Problem**: User kicked from space but hasn't received operation yet
**Solution**: Epoch-based MLS - kicked user's operations rejected by epoch mismatch

### Challenge 2: Message Deletion
**Problem**: Can't truly delete in CRDT (append-only)
**Solution**: Tombstone markers, content replaced with hash proof

### Challenge 3: Invite Revocation
**Problem**: Revoking invite while someone is using it
**Solution**: Invite operations include creation timestamp, check validity at operation time

### Challenge 4: Role Hierarchy Conflicts
**Problem**: Two admins simultaneously change role hierarchy
**Solution**: Last-write-wins (LWW) based on HLC timestamp, higher timestamp wins

---

## Privacy Analysis

### Metadata Exposed:
- ❌ Participant count in spaces (via MLS group size)
- ❌ Timing of messages (via timestamps)
- ❌ Network graph (who connects to whom)

### Metadata Protected:
- ✅ Message content (E2E encrypted)
- ✅ Who messaged whom (relay-based transport)
- ✅ Space membership (encrypted member lists)
- ✅ Attachment content (encrypted blobs)

### Privacy Recommendations:
1. **Always use relays** for network transport (hide IP addresses)
2. **Pad message sizes** to prevent traffic analysis
3. **Random delays** for message sending (timing obfuscation)
4. **Onion routing** for maximum privacy (optional Tor integration)

---

---

## Privacy Analysis & Current IP Exposure ⚠️

### Current Privacy Status

#### ✅ What's Protected:
- **Message Content**: E2E encrypted via MLS
- **Attachment Content**: Encrypted blobs
- **Space Membership**: Encrypted member lists (not published)
- **Operation Signatures**: Cryptographically authenticated
- **Invite Codes**: Random 8-character alphanumeric (not guessable)

#### ⚠️ **CRITICAL: IP Addresses ARE Currently Exposed**

**Current Network Architecture**:
```
User A (IP: 1.2.3.4) <---> Direct P2P Connection <---> User B (IP: 5.6.7.8)
                              via libp2p TCP
```

**What's Visible to Other Participants**:
1. ❌ **Your IP address** - Directly visible to all peers you connect to
2. ❌ **Connection timing** - When you come online/offline
3. ❌ **Peer graph** - Who you're connected to via DHT routing
4. ❌ **Message timing** - Exact timestamps of when you send messages
5. ❌ **Space participation** - Can infer spaces you're in by connection patterns

**Current libp2p Transport Stack**:
- Transport: TCP (exposes IP directly)
- Encryption: Noise protocol (transport layer only)
- DHT: Kademlia (broadcasts PeerID ↔ IP mapping)
- GossipSub: Propagates messages with source PeerID

### Privacy Threat Model

**Who Can See What**:

| Attacker Type | What They Can See | Risk Level |
|---------------|-------------------|------------|
| **Space Member** | Your IP, online status, message timing | HIGH ⚠️ |
| **Network Observer** (ISP) | Encrypted traffic patterns, connection graph | MEDIUM |
| **DHT Participant** | Your PeerID, IP, what topics you're interested in | HIGH ⚠️ |
| **Malicious Peer** | All of the above + can correlate across spaces | CRITICAL 🚨 |

### Recommended Privacy Improvements (Priority Order)

#### 1. ✅ **Relay Network Integrated** (Was CRITICAL)
**Previous Risk**: Anyone you chat with knows your IP address  
**Status**: Circuit Relay v2 transport integrated into swarm  
**What Changed**:
- ✅ Custom transport composition with OrTransport(relay, tcp)
- ✅ Relay client behavior added to DescordBehaviour
- ✅ dial_via_relay() API available
- ✅ Event handling for circuit establishment
- ✅ All 87 tests passing

**Privacy Status**: ⚠️ **IPs still exposed** (no relay servers deployed)

**Next Steps**:
1. Deploy relay server: `cargo install libp2p-relay-server`
2. Add relay connection on startup: `listen_on_relay(relay_addr)`
3. Use relay circuits for Private/Hidden spaces

**See**: `backend/RELAY_COMPLETE.md` for full implementation details

---

**Original Options (for reference)**:

**Option A: Tor Integration** (Maximum Privacy)
```rust
// Route all libp2p traffic through Tor
- Use libp2p-tor-transport
- All connections via .onion addresses
- Complete IP anonymity
- Trade-off: Higher latency (~200-500ms)
```

**Option B: Relay Network** (Balance Privacy/Performance)
```rust
// Implement Circuit Relay Protocol
User A <--> Relay Node <--> User B
        (IP hidden)     (IP hidden)

// Already have scaffold: create_relay_server() exists
// Need: Deploy trusted relay nodes
```

**Option C: Mix Network** (Research-Grade)
```rust
// Use mix-net like Nym or Katzenpost
- Messages batched and shuffled
- Traffic analysis resistant
- Trade-off: Complex, higher latency
```

**Recommendation**: Start with **Option B (Relay)**, add **Option A (Tor)** as optional mode

---

#### 2. **HIGH: Remove DHT for Private/Hidden Spaces** 🟠
**Current Risk**: Kademlia DHT broadcasts your PeerID and space interests

**Solution**:
```rust
pub enum SpaceVisibility {
    Public,   // OK to use DHT
    Private,  // NO DHT - invite-only peer exchange
    Hidden,   // NO DHT - out-of-band peer discovery
}

// Implementation:
impl Space {
    pub fn should_use_dht(&self) -> bool {
        matches!(self.visibility, SpaceVisibility::Public)
    }
}
```

**Changes Needed**:
- Private/Hidden spaces: Disable Kademlia for those topics
- Peer discovery: Only via invite exchange (include peer multiaddr in invite)
- Bootstrap: Use trusted relay nodes instead of DHT

---

#### 3. **MEDIUM: Padding & Timing Obfuscation** 🟡
**Current Risk**: Message sizes and timing reveal activity patterns

**Solution**:
```rust
// Pad messages to fixed sizes
const MESSAGE_SIZES: &[usize] = &[512, 1024, 4096, 16384];

fn pad_message(data: Vec<u8>) -> Vec<u8> {
    let target_size = MESSAGE_SIZES.iter()
        .find(|&&s| s >= data.len())
        .unwrap_or(&MESSAGE_SIZES[MESSAGE_SIZES.len() - 1]);
    
    let mut padded = data;
    padded.resize(*target_size, 0);
    padded
}

// Random delay before sending (0-2 seconds)
async fn send_with_delay(msg: Message) {
    let delay = rand::thread_rng().gen_range(0..2000);
    tokio::time::sleep(Duration::from_millis(delay)).await;
    send(msg).await;
}
```

---

#### 4. **LOW: Metadata Minimization** 🟢
**Current Risk**: Timestamps, member counts leak information

**Solutions**:
- Fuzzy timestamps (round to nearest hour for non-critical data)
- Don't broadcast member counts publicly
- Minimize space metadata in Public discovery

---

### Implementation Roadmap for Privacy

**Immediate (Next Sprint)**:
1. ✅ Document current IP exposure (this section)
2. ⏳ Implement relay protocol (libp2p Circuit Relay)
3. ⏳ Deploy 2-3 trusted relay nodes
4. ⏳ Make relay usage default for Private/Hidden spaces

**Short-Term (1-2 months)**:
5. Add Tor transport as optional privacy mode
6. Implement DHT filtering (disable for Private/Hidden)
7. Add message padding (fixed sizes)
8. Add timing obfuscation (random delays)

**Long-Term (3-6 months)**:
9. Mix network integration (Nym/Katzenpost)
10. Decoy traffic (cover traffic to hide activity)
11. Forward secrecy improvements (rotate MLS epochs more frequently)

---

### Privacy vs. Performance Trade-offs

| Feature | Privacy Gain | Performance Cost | Recommendation |
|---------|--------------|------------------|----------------|
| **Tor Transport** | HIGH - Complete IP anonymity | HIGH - 3-5x latency | Optional mode for high-threat users |
| **Relay Network** | MEDIUM - Hides IP from peers | LOW - 1.5x latency | **DEFAULT for Private/Hidden** |
| **No DHT** | MEDIUM - Reduces metadata leaks | MEDIUM - Slower peer discovery | **Required for Private/Hidden** |
| **Message Padding** | LOW - Prevents traffic analysis | LOW - 10-20% bandwidth | Enable by default |
| **Timing Delays** | LOW - Obscures activity patterns | LOW - 0-2s delay per message | Optional (user preference) |

---

### Cleaning Up Disk Space 🧹

**Current Project Size**: ~12 GB in `target/` folder

**Safe to Delete**:
```powershell
# Delete build artifacts (safe - will rebuild on next compile)
cargo clean

# This will free up ~12 GB
```

**Prevent Future Buildup**:
```powershell
# Add to .gitignore (if not already present)
target/
**/*.rs.bk
Cargo.lock  # Only if not publishing library

# Configure cargo to use shared target directory
# Add to ~/.cargo/config.toml:
[build]
target-dir = "C:/cargo-target"  # Single shared build cache
```

**Recommended Maintenance**:
```powershell
# Run periodically to clean old builds
cargo clean --release  # Keep debug builds for faster iteration
```

---

## Privacy Analysis

### Metadata Exposed:
- ⚠️ **IP Addresses** - Directly visible to connected peers (CRITICAL)
- ❌ Participant count in spaces (via connection patterns)
- ❌ Timing of messages (via timestamps)
- ❌ Network graph (who connects to whom via DHT)
- ❌ Online/offline patterns (via connection events)

### Metadata Protected:
- ✅ Message content (E2E encrypted)
- ✅ Who messaged whom (in Private/Hidden spaces after relay implementation)
- ✅ Space membership (encrypted member lists)
- ✅ Attachment content (encrypted blobs)
- ✅ Invite codes (cryptographically random)

### Privacy Recommendations for Users:
1. **Use VPN or Tor** - Until relay network is deployed
2. **Use Hidden spaces** - For sensitive communities
3. **Disable DHT discovery** - Use invite-only mode (to be implemented)
4. **Be aware**: Current version exposes IP to space members

---

## Questions to Answer

1. **How to handle space discovery without exposing metadata?**
   - Option A: Public DHT directory (exposes existence)
   - Option B: Invite-only (no discovery)
   - Option C: Federated directory servers (semi-centralized)

2. **How to implement voice without centralized servers?**
   - Option A: Pure P2P mesh (doesn't scale)
   - Option B: Volunteer SFU relays (requires trust)
   - Option C: Paid relay infrastructure (sustainable)

3. **How to handle content moderation with E2E encryption?**
   - Cannot scan messages server-side
   - Client-side scanning (privacy violation)
   - User reports + human moderators (current best practice)

4. **How to scale to 1000+ member spaces efficiently?**
   - MLS scales well (logarithmic key updates)
   - GossipSub scales well (probabilistic broadcast)
   - Storage challenge: Who stores all messages?
   - Solution: Sharded storage, voluntary pinning

---

## Next Steps

1. **Implement Space Visibility** (Public/Private/Hidden)
2. **Build Invite System** (codes, expiration, permissions)
3. **Design Permissions CRDT** (role changes, overrides)
4. **Add Direct Message support** (separate from spaces)
5. **Create moderation operations** (ban, timeout, delete)

Each feature needs:
- CRDT operation types
- Network gossip protocol
- MLS group management
- Privacy impact analysis
- Implementation plan
