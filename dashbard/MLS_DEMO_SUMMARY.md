# Complete MLS Encryption Demo - Quick Reference

## New Actions Added

| Action           | Purpose                                                    | Required Fields                                       |
| ---------------- | ---------------------------------------------------------- | ----------------------------------------------------- |
| **CreateThread** | Create a discussion thread in a channel                    | Space ID, Channel ID, Title (optional), First Message |
| **SendMessage**  | Send an encrypted message to a thread                      | Space ID, Thread ID, Message Content                  |
| **RemoveMember** | Kick a user from the space (removes MLS encryption access) | Space ID, User ID (64 chars)                          |

## Complete Workflow Summary

```
1. ConnectPeers → Connect Alice, Bob, Charlie via P2P
2. Alice: CreateSpace("red") → Get Space ID
3. Alice: CreateChannel(space_id, "general") → Get Channel ID
4. Alice: CreateInvite(space_id) → Get invite code
5. Bob: JoinSpace(space_id) → Bob joins, both see Members: 2
6. Alice: CreateThread(space_id, channel_id, "Welcome", "Hello!") → Get Thread ID
7. Bob: SendMessage(space_id, thread_id, "Hi Alice!") → ✓ Encrypted
8. Alice: SendMessage(space_id, thread_id, "Great!") → ✓ Encrypted
9. Alice: RemoveMember(space_id, bob_user_id) → 🔐 Bob kicked, epoch++
10. Alice: SendMessage(space_id, thread_id, "Secret!") → 🚫 Bob can't decrypt
11. Bob: SendMessage(space_id, thread_id, "Hello?") → ✗ Fails
```

## Key Observations

### Before Kick (Bob is Member)

- ✅ Bob can send messages
- ✅ Bob can read all messages
- ✅ Both in MLS group epoch N
- Dashboard shows: **Members: 2**

### After Kick (Bob Removed)

- 🚫 Bob **cannot** decrypt new messages (epoch N+1)
- ✗ Bob **cannot** send messages (no MLS group membership)
- ✅ Bob **can still** read old messages (had keys for epoch N)
- ✅ Alice **can** send/read all messages
- Dashboard shows: **Members: 1**

## ID Formats Reference

| Type        | Length              | Example                                                            |
| ----------- | ------------------- | ------------------------------------------------------------------ |
| Space ID    | 64 chars (32 bytes) | `b081442be014d1d0286241b879685afd5aa5fccc627833ec975ed5af024403d2` |
| Channel ID  | 64 chars (32 bytes) | `a5f3e7c1...` (same format)                                        |
| Thread ID   | 64 chars (32 bytes) | `7e4a92b8...` (same format)                                        |
| User ID     | 64 chars (32 bytes) | `91ead4a23a5259a5...` (full length)                                |
| Message ID  | 64 chars (32 bytes) | `3f9a7e2c...` (same format)                                        |
| Invite Code | 8 chars             | `ph18Csmh` (alphanumeric)                                          |

## Finding User IDs

**From Backend Logs:**

```
✓ Alice created: 5bf150a8226878a3
✓ Bob created: 91ead4a23a5259a5
✓ Charlie created: f54fee5f52a9359b
```

⚠️ **Note**: These are truncated! You need the full 64-character hex string.

**From Dashboard State:**
Check the WebSocket JSON or use browser DevTools to see full user_id values.

## MLS Security Properties

### Forward Secrecy ✅

- New epoch = new keys
- Old keys can't decrypt new messages
- Kicked members left behind on old epoch

### Post-Compromise Security ✅

- Even if Bob's device compromised after kick
- Attacker only gets keys for epochs Bob was member
- Cannot decrypt messages from epochs after removal

### End-to-End Encryption ✅

- Messages encrypted client-side
- Network only sees ciphertext
- Only group members (with current epoch keys) can decrypt

### Instant Revocation ✅

- Remove member → next message uses new epoch
- No delay, no sync needed
- Cryptographically enforced

## Common Errors

### "Invalid space_id length, expected 32 bytes"

- **Cause**: Using truncated ID (only first 8-16 chars)
- **Fix**: Use the **📋 Copy ID** button to get full 64-char hex

### "Action failed: Not a member of this space"

- **Cause**: User not in space, or removed
- **Fix**: Have space owner create invite, user joins with JoinSpace

### "DHT GET failed: NotFound"

- **Cause**: Peers not connected
- **Fix**: Run ConnectPeers action first (Step 0)

### "Invalid user_id length"

- **Cause**: Using shortened User ID
- **Fix**: Get full 64-char User ID from backend logs or dashboard JSON

### Bob can still send after kick

- **Possible**: Client hasn't processed removal yet
- **Check**: Dashboard should show Members: 1 for space
- **Verify**: Backend logs show MLS commit operation

## Next Steps

After completing the basic kick scenario:

1. **Rejoin**: Alice creates new invite → Bob joins again with fresh MLS keys
2. **Multiple Channels**: Create channel "private" → only Alice can access
3. **Charlie as Moderator**: Make Charlie admin → Charlie kicks Bob
4. **Permission Testing**: Try creating channel as non-admin (should fail)
5. **Message History**: Verify Bob still has old messages locally

## Technical Deep Dive

### MLS Epoch Progression

```
Epoch 1: Alice creates space (founder)
  - Alice has keys for epoch 1

Epoch 2: Bob joins space (via Welcome message)
  - Alice & Bob have keys for epoch 2

Epoch 3: First messages sent
  - Alice & Bob can encrypt/decrypt

Epoch 4: Alice removes Bob (Commit operation)
  - Alice has keys for epoch 4
  - Bob ONLY has keys for epochs 2-3
  - Bob CANNOT get keys for epoch 4+

Epoch 5+: New messages
  - Only Alice can read (only member)
  - Forward secrecy: Even if epoch 3 keys leak, epoch 5 safe
```

### What Happens on RemoveMember

1. **Backend** (`client.remove_member()`):

   - Generates MLS Commit to remove user
   - Increments group epoch
   - Generates new symmetric key for epoch
   - Creates new key schedule

2. **Broadcast**:

   - Commit message sent via GossipSub
   - All remaining members process commit
   - Update their local MLS group state

3. **Removed User**:

   - Processes commit (knows they're removed)
   - Deletes local MLS group membership
   - Can no longer generate valid messages
   - Cannot derive new epoch keys

4. **New Messages**:
   - Use Application Secret from new epoch
   - Encrypted with keys unknown to removed user
   - Cryptographically impossible to decrypt without group membership

## Files Modified

### Backend

- `dashboard-backend/src/main.rs`:
  - Added `CreateThread`, `SendMessage`, `RemoveMember` actions
  - Implemented handlers with proper hex parsing
  - Uses spaceway-core Client methods

### Frontend

- `dashboard-frontend/src/components/ActionPanel.tsx`:
  - Added form inputs for new actions
  - Thread creation with optional title
  - User removal with User ID input
  - Message sending with space + thread context

### Documentation

- `MLS_KICK_SCENARIO.md` - Complete step-by-step tutorial
- `MLS_DEMO_SUMMARY.md` - This file (quick reference)

## API Endpoints Used

```rust
// Space & membership
client.create_space(name, None)
client.join_space_from_dht(space_id)
client.remove_member(space_id, user_id)

// Channel & thread
client.create_channel(space_id, name, None)
client.create_thread(space_id, channel_id, title, first_message)

// Messages
client.post_message(space_id, thread_id, content)
client.list_messages(thread_id)

// Invites
client.create_invite(space_id, None, None)
client.list_invites(space_id)

// Network
client.network_dial(addr)
client.listening_addrs()
client.peer_id()
```

## Testing Checklist

- [ ] Backend starts successfully
- [ ] Frontend connects to backend
- [ ] ConnectPeers works (no DHT errors)
- [ ] Alice creates space
- [ ] Alice creates channel
- [ ] Bob joins space successfully
- [ ] Alice creates thread
- [ ] Both can send messages
- [ ] Messages appear in dashboard
- [ ] Alice kicks Bob
- [ ] Dashboard shows Members: 1
- [ ] Alice can still send
- [ ] Bob's send attempt fails
- [ ] Backend logs show MLS operations

## Resources

- **MLS RFC**: https://www.rfc-editor.org/rfc/rfc9420.html
- **OpenMLS Docs**: https://openmls.tech/
- **libp2p Specs**: https://docs.libp2p.io/
- **Blake3 Hash**: https://github.com/BLAKE3-team/BLAKE3
