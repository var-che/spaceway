# Descord Version History

## Version Format: MAJOR.MINOR.PATCH

- **MAJOR**: Breaking changes to protocol or data structures
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, optimizations

---

## 0.1.0 - Initial Beta (November 21, 2025)

**Status**: Beta - Ready for small group testing (2-10 users)

### What's Working ✅
- **P2P Networking**
  - GossipSub real-time messaging
  - Peer-to-peer sync protocol (SYNC_REQUEST)
  - 2-peer network support
  - Explicit peering for small networks
  - Configurable listen addresses and bootstrap peers
  
- **Core Features**
  - Space/Channel/Thread/Message hierarchy
  - CRDT-based state synchronization
  - E2E encryption via MLS
  - RocksDB persistence with dual indexing
  - Space visibility (Public/Private/Hidden)
  - Invite system with 8-character codes
  - Basic roles (Admin/Moderator/Member)
  
- **CLI Application**
  - Interactive REPL interface
  - Network commands (connect, network, invite, join)
  - Message sending and viewing
  - Space/channel/thread management

### Known Limitations ⚠️
- No message history pagination (only recent messages shown)
- No notifications or unread tracking
- No user profiles or avatars
- No direct messages (DMs)
- DHT requires 3+ peers (falls back to P2P sync)
- No relay servers for NAT traversal yet

### Technical Details
- **Dependencies**: libp2p 0.53, tokio 1.35, RocksDB, MLS (OpenMLS)
- **Storage**: Dual-indexed operations (by op_id and space_id)
- **Network**: GossipSub mesh (mesh_n=2, mesh_n_low=1)
- **Sync**: SYNC_REQUEST via GossipSub topics

### Breaking Changes from Pre-0.1
- Operations now stored with space_id index
- NetworkNode.new_with_config() requires listen_addrs parameter
- GossipSub mesh configuration changed for 2-peer support

---

## Upcoming Versions

### 0.2.0 - Public Beta (Target: December 2025)
**Focus**: Essential features for usability

**Planned Features**:
- Message pagination and history
- Desktop notifications
- User profiles with avatars
- Direct messages (1-on-1 and groups)
- Improved permissions system
- Unread message tracking

**Estimated Timeline**: 3-6 weeks

---

### 0.3.0 - Feature Parity (Target: Q1 2026)
**Focus**: Rich communication features

**Planned Features**:
- File attachments and media
- Custom emoji and reactions
- Voice channels (audio only)
- Message threading
- Moderation tools (ban, timeout, message deletion)

---

### 1.0.0 - Production Ready (Target: Q2 2026)
**Focus**: Stability, security audits, scalability

**Requirements for 1.0**:
- Security audit completed
- Multi-device sync working
- Voice/video calls stable
- Tested with 100+ user communities
- Mobile apps (iOS/Android)
- Public relay infrastructure
- Comprehensive documentation

---

## Version Compatibility

### Protocol Versioning
Descord uses semantic versioning for protocol compatibility:

- **0.x.x**: Beta versions - breaking changes possible
- **1.x.x**: Stable - backward compatible within major version

### Data Migration
- Database schema changes trigger automatic migration
- Operations from older clients are forward-compatible
- CRDT ensures eventual consistency across versions

### Network Compatibility
- Peers must have same **MAJOR** version to communicate
- **MINOR** version differences are compatible (feature negotiation)
- **PATCH** versions are fully compatible
