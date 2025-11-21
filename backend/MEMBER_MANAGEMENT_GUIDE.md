# Member Management Commands - Quick Guide

## Commands Added (v0.1.1)

### 1. List Members
```bash
members
```
Shows all members in the current Space with their roles.

**Output:**
```
Members in Space (3):
  UserId(a41ab587e5fdca34) [Admin]
  UserId(cbcc147bca30c2ae) [Member]
  UserId(f3d2891a5b7c4e12) [Moderator]
```

### 2. Kick/Remove Member
```bash
kick <user_id>
# OR
remove <user_id>
```
Removes a member from the current Space.

**Permissions:**
- Only Admins and Moderators can kick members
- Cannot kick yourself
- Target must be a current member

**Example:**
```bash
# First, see who's in the Space
members

# Then kick a user (copy their full UserId from the members list)
kick cbcc147bca30c2ae
```

**Success Output:**
```
ℹ Removing user UserId(cbcc147bca30c2ae) from Space...
✓ Successfully removed user UserId(cbcc147bca30c2ae)
```

**Error Cases:**
```
✗ Failed to remove member: Only admins and moderators can remove members
✗ Failed to remove member: Cannot remove yourself from Space
✗ Failed to remove member: User not a member of Space
```

## Updated Help Command

The `help` command now shows:

```
  Spaces:
    spaces - List all spaces
    space <name> - Create or switch to space
    join <space_id> <code> - Join space with invite
    invite - Create invite for current space
    members - List members in current space
    kick <user_id> - Remove member from current space
```

## Complete Workflow Example

### Scenario: Alice kicks Bob from PrivateClub

```bash
# Alice's terminal
$ descord

# Create and enter a Space
space PrivateClub
✓ Created Space: PrivateClub

# Check current members
members
Members in Space (1):
  UserId(a41ab587e5fdca34) [Admin]  # Alice

# ... Bob joins via invite ...

# Check members again
members
Members in Space (2):
  UserId(a41ab587e5fdca34) [Admin]  # Alice
  UserId(cbcc147bca30c2ae) [Member] # Bob

# Kick Bob
kick cbcc147bca30c2ae
✓ Successfully removed user UserId(cbcc147bca30c2ae)

# Verify Bob is gone
members
Members in Space (1):
  UserId(a41ab587e5fdca34) [Admin]  # Only Alice
```

### What Happens When Bob is Kicked

1. **Immediate Effect:**
   - Bob is removed from the Space's member list
   - RemoveMember operation broadcasts to all peers
   - Bob's client receives the operation

2. **Current Behavior (v0.1.1):**
   - ✅ Bob can't send new messages (permission denied)
   - ⏳ Bob can still decrypt messages (MLS key rotation not implemented)
   - ✅ Bob keeps local copy of old messages

3. **Future Behavior (v0.2.0 with MLS):**
   - ✅ Bob can't send new messages
   - ✅ Bob can't decrypt NEW messages (MLS keys rotated)
   - ✅ Bob keeps old messages (already decrypted)

## Testing the Feature

### Manual Test
1. Start two Descord instances (Alice and Bob)
2. Alice creates a Space
3. Alice adds Bob as a member (use `invite` command)
4. Bob joins the Space
5. Both users exchange messages
6. Alice runs `members` to see Bob
7. Alice runs `kick <bob_user_id>`
8. Verify Bob is removed

### Integration Test
```bash
cargo test --test test_kick_member
```

This runs the automated test scenario in `core/tests/test_kick_member.rs`.

## Implementation Details

### Client API
```rust
// List all members in a Space
pub async fn list_members(&self, space_id: &SpaceId) -> Vec<(UserId, Role)>

// Remove a member from a Space
pub async fn remove_member(&self, space_id: SpaceId, user_id: UserId) -> Result<CrdtOp>
```

### CRDT Operation
```rust
OpType::RemoveMember(OpPayload::RemoveMember {
    user_id: UserId,
    reason: Option<String>,  // Not exposed in CLI yet
})
```

### Event Handling
When a peer receives a RemoveMember operation:
1. Validates the operation signature
2. Checks author has permission (Admin/Moderator)
3. Removes the user from local Space member list
4. Updates CRDT validator state
5. (Future) Rotates MLS group keys

## Security Notes

⚠️ **Current Limitation:** MLS key rotation is not yet implemented. This means:
- Kicked members can't send messages (enforced)
- Kicked members CAN still decrypt new messages (not enforced)

This will be fixed in v0.2.0 when MLS group management is fully integrated.

## Troubleshooting

### "No space selected"
```bash
# Switch to a Space first
space PrivateClub
# Then try members/kick
members
```

### "Invalid user ID hex"
```bash
# User ID must be the full hex string from 'members' command
# Correct:
kick cbcc147bca30c2ae

# Incorrect (missing parts):
kick cbcc
```

### "Only admins and moderators can remove members"
You need Admin or Moderator role to kick members. Check your role with:
```bash
members  # Your role is shown next to your UserId
```

---

**Version:** 0.1.1
**Status:** ✅ Feature Complete (MLS integration pending)
