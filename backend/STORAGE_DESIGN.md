# Storage Layer Design

> **Created:** November 20, 2025  
> **Status:** Design Phase  
> **Priority:** Critical for MVP

---

## Overview

Descord needs persistent storage for:
1. **Messages** - Encrypted message content (blobs)
2. **CRDT State** - Convergence metadata for offline sync
3. **User Data** - Identity keys, space memberships
4. **Relay Info** - Cached relay reputation/addresses

### Design Goals

✅ **Privacy-First**
- All message content encrypted at rest
- Keys never stored in plaintext
- Content-addressed blobs (no metadata leakage)

✅ **Offline-First**
- CRDT-based conflict resolution
- Append-only message logs
- Causal ordering preservation

✅ **Performance**
- RocksDB for fast key-value access
- SHA256-based content addressing
- Lazy blob loading (fetch on demand)

---

## Architecture

### Storage Hierarchy

```
~/.descord/
├── identity/
│   ├── keypair.bin           # User's Ed25519 keypair (encrypted)
│   └── spaces.db             # Space memberships (RocksDB)
├── messages/
│   ├── blobs/                # Content-addressed encrypted blobs
│   │   ├── <sha256-hash-1>   # Message content (AES-256-GCM)
│   │   ├── <sha256-hash-2>
│   │   └── ...
│   └── indices.db            # Message indices (RocksDB)
├── crdt/
│   └── state.db              # CRDT convergence state (RocksDB)
└── network/
    └── relays.db             # Relay cache (RocksDB)
```

---

## 1. Message Storage

### Blob Format

**Encrypted Blob Structure:**
```rust
struct EncryptedBlob {
    nonce: [u8; 12],           // AES-GCM nonce
    ciphertext: Vec<u8>,       // Encrypted message content
    tag: [u8; 16],             // Authentication tag
}

// On disk: <nonce><ciphertext><tag>
// Total size: 12 + len(plaintext) + 16 bytes
```

**Plaintext Message (before encryption):**
```rust
struct MessageContent {
    author: UserId,            // 32 bytes (Ed25519 public key)
    timestamp: u64,            // 8 bytes (Unix timestamp)
    content: String,           // Variable length UTF-8
    attachments: Vec<BlobId>,  // References to other blobs
    signature: [u8; 64],       // Ed25519 signature
}
```

### Content Addressing

**Hash = SHA256(plaintext_content)**

- Deduplication: Same content = same hash = stored once
- Integrity: Hash mismatch = corruption/tampering detected
- Privacy: Hash reveals nothing about content (one-way function)

### Encryption Keys

**Per-Thread Key Derivation:**
```rust
thread_key = HKDF-SHA256(
    ikm = mls_group_secret,    // From MLS epoch secret
    salt = thread_id,          // Thread UUID as salt
    info = "descord-thread-v1" // Context string
)
```

**Key Properties:**
- ✅ Forward secrecy (MLS rotates epoch secrets)
- ✅ Thread isolation (different threads = different keys)
- ✅ Post-compromise security (MLS key rotation on member changes)

---

## 2. RocksDB Schema

### Messages Index DB

**Key-Value Structure:**

| Column Family | Key Format | Value Format | Purpose |
|---------------|-----------|--------------|---------|
| `thread_messages` | `<thread_id>:<timestamp>:<message_id>` | `<blob_hash>` | Thread message ordering |
| `user_messages` | `<user_id>:<timestamp>:<message_id>` | `<blob_hash>` | User's message history |
| `blob_metadata` | `<blob_hash>` | `<thread_id><author><timestamp><size>` | Blob lookup info |
| `message_refs` | `<message_id>` | `<blob_hash>` | Message ID → blob mapping |

**Example Queries:**

```rust
// Get all messages in a thread (chronological)
let iter = db.prefix_iterator(format!("thread_messages:{}:", thread_id));
for (key, blob_hash) in iter {
    let blob = load_blob(blob_hash)?;
    let message = decrypt_message(blob, thread_key)?;
    messages.push(message);
}

// Get user's recent messages
let iter = db.prefix_iterator(format!("user_messages:{}:", user_id))
    .take(100);  // Last 100 messages
```

### CRDT State DB

**CRDT Metadata:**

| Key | Value | Purpose |
|-----|-------|---------|
| `<thread_id>:vector_clock` | `{user1: 42, user2: 37, ...}` | Causal ordering |
| `<thread_id>:tombstones` | `Set<MessageId>` | Deleted messages |
| `<thread_id>:last_sync` | `Timestamp` | Last successful sync |

**Vector Clock Example:**
```rust
// Alice has seen 42 messages from herself, 37 from Bob
vector_clock = {
    alice_id: 42,
    bob_id: 37,
}

// When Alice creates message #43:
vector_clock[alice_id] += 1;  // Now 43

// When syncing with Bob, compare vector clocks
// to determine which messages are missing
```

### Relay Cache DB

**Relay Information:**

| Key | Value | Purpose |
|-----|-------|---------|
| `relay:<peer_id>` | `<RelayInfo JSON>` | Relay metadata |
| `reputation:<peer_id>` | `<ReputationScore>` | Relay quality metrics |
| `last_seen:<peer_id>` | `Timestamp` | When relay was last alive |

---

## 3. Privacy Guarantees

### What's Encrypted

✅ **Message Content** (100% private)
- Encrypted with per-thread AES-256-GCM keys
- Keys derived from MLS group secrets
- Only group members can decrypt

✅ **Attachments** (100% private)
- Same encryption as messages
- Content-addressed (hash of plaintext)
- Stored separately, referenced by hash

### What's NOT Encrypted (Metadata)

⚠️ **Message Indices** (40% private)
- Thread ID visible in DB keys
- Timestamps visible (causal ordering needs this)
- Message count per thread visible
- **Mitigation:** Encrypt DB at OS level (LUKS, FileVault, BitLocker)

⚠️ **CRDT State** (40% private)
- Vector clocks reveal user activity patterns
- Tombstones reveal deletion patterns
- **Mitigation:** Same as above (OS-level encryption)

### Threat Model

**Protected Against:**
- 🛡️ Disk forensics (if OS encryption enabled)
- 🛡️ Stolen laptop (encrypted blobs useless without keys)
- 🛡️ Malicious storage provider (blobs are encrypted)
- 🛡️ Database dumps (no plaintext message content)

**Not Protected Against:**
- ⚠️ Active memory forensics (keys in RAM during decryption)
- ⚠️ Malicious process on same machine (can read decrypted messages)
- ⚠️ Compromised MLS group (any member can decrypt)

**Mitigation:** Use OS-level full-disk encryption (mandatory for production)

---

## 4. CRDT Synchronization

### Message Convergence

**Append-Only Log:**
- Messages are never modified, only appended
- Deletion = append tombstone marker
- Edit = append new version with "replaces" field

**Conflict Resolution:**
```rust
// Two users create messages concurrently
// Alice: msg_a at timestamp T1, vector_clock = {alice: 10, bob: 5}
// Bob:   msg_b at timestamp T1, vector_clock = {alice: 9, bob: 6}

// After sync, both clients converge to same order:
if vector_clock_a.happens_before(vector_clock_b) {
    order = [msg_a, msg_b];  // Causal ordering
} else if vector_clock_b.happens_before(vector_clock_a) {
    order = [msg_b, msg_a];
} else {
    // Concurrent (neither happens before the other)
    // Break tie with deterministic rule
    order = [msg_a, msg_b].sort_by(|a, b| {
        (a.author, a.timestamp).cmp(&(b.author, b.timestamp))
    });
}
```

**Properties:**
- ✅ **Commutativity:** Alice→Bob sync = Bob→Alice sync
- ✅ **Associativity:** (A sync B) sync C = A sync (B sync C)
- ✅ **Idempotence:** Syncing twice = syncing once

---

## 5. Implementation Plan

### Phase 1: Basic Storage (Week 1)

1. **RocksDB Setup**
   - Create storage directory structure
   - Initialize RocksDB with column families
   - Basic key-value operations

2. **Blob Storage**
   - Content-addressed blob writer
   - Encryption with AES-256-GCM
   - SHA256 hashing for content addressing

3. **Message Persistence**
   - Save message to blob
   - Index message in RocksDB
   - Retrieve message by ID/thread

### Phase 2: CRDT Sync (Week 2)

1. **Vector Clock Implementation**
   - Per-user message counters
   - Happens-before logic
   - Causal ordering

2. **Conflict Resolution**
   - Concurrent message handling
   - Deterministic tie-breaking
   - Tombstone propagation

3. **Sync Protocol**
   - Compare vector clocks
   - Request missing messages
   - Apply updates atomically

### Phase 3: Optimization (Week 3)

1. **Lazy Loading**
   - Fetch blobs on demand (not on sync)
   - Thread preview (first N messages)
   - Pagination for large threads

2. **Relay Caching**
   - Persist relay reputation
   - Periodic re-advertisement
   - Stale relay pruning

3. **Compression**
   - Compress blobs before encryption
   - LZ4 for speed (not gzip)
   - Store compressed size in metadata

---

## 6. Testing Strategy

### Unit Tests

```rust
#[test]
fn test_blob_encryption_roundtrip() {
    let key = generate_key();
    let plaintext = b"Hello, world!";
    
    let blob = encrypt_blob(plaintext, &key)?;
    let hash = sha256(&plaintext);
    
    let decrypted = decrypt_blob(&blob, &key)?;
    assert_eq!(plaintext, decrypted);
    
    // Verify content addressing
    assert_eq!(hash, sha256(&decrypted));
}

#[test]
fn test_message_persistence() {
    let db = open_test_db()?;
    let thread_id = ThreadId::new();
    
    // Save message
    let msg = create_test_message();
    store_message(&db, thread_id, &msg)?;
    
    // Retrieve message
    let retrieved = get_message(&db, msg.id)?;
    assert_eq!(msg, retrieved);
}

#[test]
fn test_crdt_convergence() {
    let alice_db = open_test_db()?;
    let bob_db = open_test_db()?;
    
    // Alice creates message
    alice_db.add_message(msg_a)?;
    
    // Bob creates concurrent message
    bob_db.add_message(msg_b)?;
    
    // Sync both directions
    sync(&alice_db, &bob_db)?;
    sync(&bob_db, &alice_db)?;
    
    // Both should converge to same state
    assert_eq!(alice_db.messages(), bob_db.messages());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_offline_sync() {
    // Alice and Bob start online
    let mut alice = create_client("Alice").await?;
    let mut bob = create_client("Bob").await?;
    
    // Alice goes offline
    alice.disconnect().await?;
    
    // Bob sends messages while Alice is offline
    bob.send_message("Message 1").await?;
    bob.send_message("Message 2").await?;
    
    // Alice comes back online and syncs
    alice.connect().await?;
    alice.sync().await?;
    
    // Alice should see Bob's messages
    let messages = alice.get_messages().await?;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "Message 1");
    assert_eq!(messages[1].content, "Message 2");
}
```

---

## 7. Dependencies

### New Crates Needed

```toml
[dependencies]
# Storage
rocksdb = "0.22"              # Embedded key-value database
sha2 = "0.10"                 # SHA256 hashing
aes-gcm = "0.10"              # AES-256-GCM encryption
hkdf = "0.12"                 # Key derivation (HKDF)

# Compression (optional)
lz4 = "1.24"                  # Fast compression

# Serialization
bincode = "1.3"               # Binary encoding
serde = { version = "1.0", features = ["derive"] }
```

---

## 8. API Design

### Storage API

```rust
pub struct Storage {
    db: rocksdb::DB,
    blob_dir: PathBuf,
}

impl Storage {
    /// Open storage at path
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    
    /// Store a message (encrypts and indexes)
    pub fn store_message(
        &self,
        thread_id: ThreadId,
        message: &Message,
        key: &[u8; 32],
    ) -> Result<BlobHash>;
    
    /// Retrieve a message by ID
    pub fn get_message(
        &self,
        message_id: MessageId,
        key: &[u8; 32],
    ) -> Result<Message>;
    
    /// Get all messages in a thread
    pub fn get_thread_messages(
        &self,
        thread_id: ThreadId,
        key: &[u8; 32],
    ) -> Result<Vec<Message>>;
    
    /// Update CRDT vector clock
    pub fn update_vector_clock(
        &self,
        thread_id: ThreadId,
        user_id: UserId,
        counter: u64,
    ) -> Result<()>;
    
    /// Get CRDT state for sync
    pub fn get_crdt_state(
        &self,
        thread_id: ThreadId,
    ) -> Result<CrdtState>;
}
```

---

## 9. Security Considerations

### Key Management

⚠️ **Critical:** Thread keys must NEVER be stored in plaintext
- Derive from MLS epoch secrets (already in memory)
- Wipe keys from memory after use (`zeroize` crate)
- Use OS keychain for identity keys (optional)

### Blob Integrity

✅ **Tamper Detection:**
- SHA256 hash mismatch = reject blob
- AES-GCM authentication tag = detect modifications
- MLS signatures = verify author identity

### Database Corruption

✅ **Recovery:**
- RocksDB has WAL (write-ahead log)
- Atomic batch writes (all-or-nothing)
- Periodic backups (user-initiated)

---

## 10. Privacy Score Impact

### Before Storage Layer
- Message content: 100% private (MLS encrypted, in-memory only)
- Metadata: 40% private (timing/size visible on network)
- **Overall:** 85% privacy

### After Storage Layer (with OS encryption)
- Message content: 100% private (encrypted blobs + OS encryption)
- Metadata: 50% private (encrypted DB at rest, but indices visible to disk forensics without OS encryption)
- **Overall:** 85% privacy (maintained)

### After Storage Layer (without OS encryption)
- Message content: 100% private (blobs still encrypted)
- Metadata: 30% private (DB keys/indices visible in plaintext)
- **Overall:** 80% privacy (slight decrease)

**Recommendation:** Make OS-level full-disk encryption **mandatory** for production builds.

---

## Next Steps

1. ✅ Review this design document
2. ⏳ Implement Phase 1 (Basic Storage)
3. ⏳ Add storage tests
4. ⏳ Implement Phase 2 (CRDT Sync)
5. ⏳ Implement Phase 3 (Optimization)

**Ready to start implementation?**
