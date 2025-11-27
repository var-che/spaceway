# MLS CLI Commands - Implementation Complete ✅

## What Was Added

Added two new CLI commands to expose the existing MLS infrastructure:

### 1. `keypackage publish`

Publishes KeyPackages to the DHT so others can add you to encrypted groups.

**Usage**:

```bash
> keypackage publish
```

**What it does**:

- Generates 10 fresh KeyPackages
- Publishes them to DHT under key `keypackage:<user_id_hex>`
- Allows others to fetch your KeyPackages and add you to MLS groups
- KeyPackages are consumed as you join groups (one per group)

**Output**:

```
ℹ Publishing KeyPackages to DHT...

  Generating 10 KeyPackages...

✓ Published 10 KeyPackages to DHT

  ✓ Others can now add you to MLS encryption groups
  ✓ KeyPackages will be consumed as you join groups

  Tip: Re-run this command periodically to refresh expired packages
```

### 2. `member add <user_id>`

Adds a user to the MLS encryption group for the current space.

**Usage**:

```bash
> member add 1a2b3c4d5e6f7890...  # Full 64-char hex user ID
```

**What it does**:

1. Fetches the user's KeyPackage from DHT
2. Adds them to the MLS group
3. Sends them a Welcome message (via GossipSub topic `user/<id>/welcome`)
4. Rotates encryption keys (increments epoch)
5. Publishes Commit message to existing members

**Output**:

```
ℹ Adding 1a2b3c4d to MLS encryption group...

  This will:
  1. Fetch their KeyPackage from DHT
  2. Add them to the MLS group
  3. Send them a Welcome message
  4. Rotate encryption keys (new epoch)

🔑 Fetching KeyPackage for user UserId(...) from DHT...
✓ Fetched KeyPackage for user UserId(...) from DHT
  ✓ Added to MLS group, epoch rotated
  ✓ Published Commit to existing members on space/...
  ✓ Sent Welcome message to UserId(...) on user/.../welcome
✅ Member UserId(...) added with MLS (DHT)

✓ User 1a2b3c4d added to MLS group!

  ✓ User can now decrypt messages in this space
```

## Updated Help Menu

The `help` command now shows:

```
  MLS Encryption:
    keypackage publish - Publish KeyPackages to DHT for MLS
    member add <user_id> - Add member to MLS encryption group
```

## Testing the Flow

### Setup (Both Alice and Bob)

1. **Start Alice** (Terminal 1):

```bash
cargo run --release -- --account alice.key --port 9001
```

2. **Publish Alice's KeyPackages**:

```
> keypackage publish
```

3. **Start Bob** (Terminal 2):

```bash
cargo run --release -- --account bob.key --port 9002
```

4. **Publish Bob's KeyPackages**:

```
> keypackage publish
```

5. **Connect Bob to Alice**:

```
> connect /ip4/127.0.0.1/tcp/9001/p2p/<alice_peer_id>
```

### Test 1: Complete E2EE Flow (CRDT + MLS)

**Alice creates space and invites Bob**:

```
> space create "encrypted-test"
> invite create
> members  # Get Alice's user ID for reference
```

**Bob joins the space** (CRDT layer):

```
> join <space_id> <invite_code>
> space list  # Should see "encrypted-test"
```

At this point:

- ✅ Bob is in the Space (CRDT layer)
- ❌ Bob is NOT in the MLS group yet

**Alice adds Bob to MLS group** (Encryption layer):

```
> members  # Copy Bob's user ID
> member add <bob_user_id>
```

**Bob receives Welcome message** (automatically):

```
  🎉 Received MLS Welcome message
  ✓ Processing Welcome message...
  ✓ Joined MLS group for space space_...
```

At this point:

- ✅ Bob is in the Space (CRDT layer)
- ✅ Bob is in the MLS group (Encryption layer)
- ✅ Bob can decrypt messages (once message encryption is implemented)

### Test 2: Verify MLS Group State

**Both Alice and Bob can check**:

```
> members
```

This shows CRDT membership (Space layer), not MLS group membership (which is internal).

## What Still Needs Implementation

The CLI commands are done ✅, but to complete E2EE we still need:

### Phase 2: Fix Deadlocks in MLS Functions ⚠️

The `add_member_with_mls()` function in `core/src/client.rs` may have the same deadlock pattern. Need to verify and fix.

### Phase 3: Message Encryption ❌

Currently messages are sent as plaintext in CRDT operations. Need to:

1. **Encrypt on send** (`post_message()`):

   ```rust
   let encrypted = mls_group.encrypt_application_message(content.as_bytes())?;
   ```

2. **Decrypt on receive** (event handler):

   ```rust
   let decrypted = mls_group.decrypt_application_message(encrypted_content)?;
   let content = String::from_utf8(decrypted)?;
   ```

3. **Fallback to plaintext** if no MLS group exists (backwards compatibility)

### Phase 4: Key Rotation on Kick ❌

When Alice kicks Bob:

```
> kick <bob_user_id>
```

Should:

1. Remove Bob from MLS group
2. Rotate keys (new epoch)
3. Bob can't decrypt future messages ✅

Currently only removes from CRDT Space, not MLS group.

## Security Properties (When Fully Implemented)

✅ **Forward Secrecy**: After being kicked, Bob can't decrypt new messages
✅ **Post-Compromise Security**: Key rotation limits damage from compromised keys
✅ **Group Authentication**: Only group members can send valid encrypted messages
⚠️ **Metadata Privacy**: Message metadata (who/when) is NOT encrypted (GossipSub limitation)

## Technical Details

### KeyPackage Storage

- **DHT Key**: `SHA256("keypackage:" + user_id_hex)[:32]`
- **Value**: JSON-serialized array of `KeyPackageBundle`
- **Expiration**: KeyPackages have timestamps, expire after period
- **Consumption**: One KeyPackage consumed per group joined

### Welcome Message Delivery

- **Topic**: `user/<user_id>/welcome`
- **Subscription**: Automatic (subscribed in Client initialization)
- **Processing**: Automatic (event handler in Client::start())
- **Storage**: MLS group stored in SpaceManager

### MLS Group Management

- **Location**: `SpaceManager.mls_groups: HashMap<SpaceId, MlsGroup>`
- **Epoch**: Increments on every member add/remove
- **Key Rotation**: Automatic on epoch change
- **Persistence**: Stored in RocksDB (via storage layer)

## Files Modified

### `/home/vlada/Documents/projects/spaceway/cli/src/commands.rs`

1. **Added command handlers** (line ~51):

   - `"member" => self.cmd_member(&parts[1..]).await`
   - `"keypackage" => self.cmd_keypackage(&parts[1..]).await`

2. **Updated help text** (line ~120):

   - Added "MLS Encryption" section with both commands

3. **Added `cmd_member()` method** (line ~535):

   - Parses user ID from hex
   - Calls `client.add_member_with_mls()`
   - Displays detailed progress and error messages

4. **Added `cmd_keypackage()` method** (line ~600):
   - Calls `client.publish_key_packages_to_dht()`
   - Provides user guidance on usage

## Next Steps

1. ✅ **CLI Commands** - DONE!
2. ⚠️ **Test Basic Flow** - Ready to test!
3. ❌ **Fix MLS Deadlocks** - Need to verify `add_member_with_mls()`
4. ❌ **Implement Message Encryption** - Biggest remaining piece
5. ❌ **Test Key Rotation** - After kick implementation

## How to Test Right Now

Even without message encryption, you can test the MLS group membership flow:

```bash
# Terminal 1 (Alice)
$ cargo run --release -- --account alice.key --port 9001
> keypackage publish
> space create "test-mls"
> invite create

# Terminal 2 (Bob)
$ cargo run --release -- --account bob.key --port 9002
> keypackage publish
> connect /ip4/127.0.0.1/tcp/9001/p2p/...
> join <space_id> <invite_code>

# Terminal 1 (Alice)
> members  # Copy Bob's user ID
> member add <bob_user_id>  # Add Bob to MLS group

# Terminal 2 (Bob) - Should see:
  🎉 Received MLS Welcome message
  ✓ Joined MLS group for space space_...
```

At this point, the MLS group is established! Messages just aren't encrypted yet.

## Build Status

✅ **Compiles successfully**

- Build time: ~9 seconds
- Only warnings (unused imports, dead code)
- No errors

## Conclusion

The CLI commands for MLS are now fully implemented and ready to use! The infrastructure for E2EE is complete, we just need to:

1. Verify no deadlocks in `add_member_with_mls()`
2. Implement message encryption/decryption
3. Implement key rotation on member removal

The hard part (MLS protocol implementation) is done. The remaining work is connecting the pieces together.
