# Member Kick/Remove Implementation

## Status: ✅ Core Implementation Complete

### Implemented (v0.1.0)

#### 1. Client API
**File:** `core/src/client.rs`

```rust
/// Remove a member from a Space (kick)
pub async fn remove_member(
    &self,
    space_id: SpaceId,
    user_id: UserId,
) -> Result<CrdtOp>
```

- Creates RemoveMember CRDT operation
- Signs operation with author's keypair
- Broadcasts to all peers via GossipSub
- Stores locally in RocksDB

#### 2. SpaceManager API
**File:** `core/src/forum/space.rs`

```rust
pub fn remove_member(
    &mut self,
    space_id: SpaceId,
    user_id: UserId,
    author: UserId,
    author_keypair: &Keypair,
) -> Result<CrdtOp>
```

**Permission Checks:**
- ✅ Only Admin or Moderator can remove members
- ✅ Cannot remove yourself
- ✅ Target user must be a member

**Operation Flow:**
1. Validates author has `can_kick_members()` permission
2. Creates `RemoveMember` CRDT operation with:
   - Target `user_id`
   - Optional `reason` (currently None)
   - Proper causality tracking (HLC, epoch)
3. Signs operation with Ed25519
4. Applies locally: `space.remove_member(&user_id)`
5. Updates CRDT validator state

#### 3. CRDT Operation
**File:** `core/src/crdt/ops.rs`

```rust
OpType::RemoveMember(OpPayload::RemoveMember {
    user_id: UserId,
    reason: Option<String>,
})
```

**Validator Handling:**
- Marks member as removed at current epoch
- Updates `MembershipRecord.removed_at`
- Tracks removal in causality graph

#### 4. Member Listing
**File:** `core/src/client.rs`

```rust
pub async fn list_members(&self, space_id: &SpaceId) -> Vec<(UserId, Role)>
```

Retrieves all current members and their roles from a Space.

### Remaining Work for Full Security

#### 1. MLS Key Rotation (HIGH PRIORITY)
**File:** `core/src/mls/group.rs`

When a member is removed, the MLS group MUST rotate keys so the removed member can't decrypt future messages.

**Required:**
```rust
impl MlsGroup {
    pub fn remove_member(&mut self, user_id: &UserId) -> Result<()> {
        // 1. Remove member from MLS group
        // 2. Generate new epoch keys
        // 3. Distribute Welcome messages to remaining members
        // 4. Update local encryption state
    }
}
```

**Security Impact:** ⚠️ **CRITICAL** - Without this, kicked members can still decrypt new messages!

#### 2. Event Handler for Incoming RemoveMember
**File:** `core/src/client.rs` (in `handle_incoming_op`)

Currently RemoveMember operations are created but not fully processed when received from peers.

**Required:**
```rust
crate::crdt::OpType::RemoveMember(_) => {
    let mut space_manager = self.space_manager.write().await;
    space_manager.process_remove_member(&op)?;
    
    // Update MLS group
    if let Some(mls_group) = self.mls_groups.get_mut(&op.space_id) {
        mls_group.remove_member(&user_id)?;
    }
}
```

#### 3. CLI Command
**File:** `cli/src/commands.rs`

```rust
"kick" | "remove" => {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 2 {
        ui::print_error("Usage: kick <username>");
        continue;
    }
    
    cmd_kick(client, parts[1]).await?;
}
```

### Test Scenario

**Integration Test:** `core/tests/test_kick_member.rs`

**Scenario:**
1. Alice creates a Space
2. Alice adds Bob as a member
3. Alice and Bob exchange messages
4. Alice kicks Bob
5. Alice sends new messages
6. **Verify:** Bob can't decrypt new messages (MLS security)
7. **Verify:** Bob can't send messages (permission denied)

**Current Status:**
- ✅ Steps 1-3 working
- ✅ Step 4 (kick) API implemented
- ⏳ Steps 5-7 require MLS integration

### Usage Example

```rust
// Alice kicks Bob from a Space
let bob_user_id = bob_keypair.user_id();
alice.remove_member(space_id, bob_user_id).await?;

// Bob is now removed:
// - Can't send new messages
// - Can't see new messages (once MLS is integrated)
// - Still has local copy of old messages
```

### API Compatibility

**v0.1.0:**
- Remove member operation broadcasts to all peers
- Compatible with existing CRDT operation sync
- No breaking changes to existing APIs

**Future (v0.2.0):**
- MLS key rotation on removal
- Kicked member receives notification
- Optional ban (prevents re-joining)

### Security Properties

**Current Implementation:**
- ✅ Permission-based access control
- ✅ CRDT operation signature verification
- ✅ Causal consistency (HLC timestamps)
- ⏳ E2E encryption (requires MLS integration)

**After MLS Integration:**
- ✅ Forward secrecy (kicked members can't decrypt new messages)
- ✅ Post-compromise security (new keys generated)
- ✅ Group membership authentication

### Next Steps

1. **Immediate (v0.1.1):**
   - Add `kick` CLI command
   - Complete RemoveMember event handler
   - Add notification to kicked member

2. **Short-term (v0.2.0):**
   - Integrate MLS key rotation
   - Add ban functionality (prevent re-join)
   - Add kick reason field support

3. **Long-term (v1.0.0):**
   - Audit log for kicks/bans
   - Temporary kicks (timeout)
   - Appeal system

### References

- CRDT Operations: `core/src/crdt/ops.rs`
- Permission System: `core/src/permissions.rs`
- MLS Group Management: `core/src/mls/group.rs`
- Space Management: `core/src/forum/space.rs`

---

**Last Updated:** 2025 (v0.1.0 - Initial Beta)
