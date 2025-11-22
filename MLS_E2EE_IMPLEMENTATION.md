# MLS End-to-End Encryption Implementation Guide

## Current Status

✅ **Working**: CRDT-based space membership (plaintext)
⚠️ **Needs Integration**: MLS encryption layer

## Architecture Overview

Spaceway uses a **two-layer membership system**:

1. **CRDT Layer** (Space Membership):

   - Controls who can see spaces, channels, threads
   - Works offline, eventual consistency
   - Currently fully functional ✅

2. **MLS Layer** (Message Encryption):
   - Controls who can decrypt message content
   - Requires online coordination for key rotation
   - Infrastructure exists but not exposed in CLI ⚠️

## What Needs to Happen

### Phase 1: KeyPackage Management (Bob publishes his pre-keys)

**Current Flow**:

```
Bob starts app → (nothing happens with MLS)
```

**Needed Flow**:

```
Bob starts app → Generates KeyPackages → Publishes to DHT → Periodically refreshes
```

**Implementation**:

#### 1.1 Add CLI command for KeyPackage publication

**File**: `cli/src/commands.rs`

Add to `handle_command()`:

```rust
"keypackage" => self.cmd_keypackage(&parts[1..]).await,
```

Add new method:

```rust
async fn cmd_keypackage(&mut self, args: &[&str]) -> Result<()> {
    if args.is_empty() {
        ui::print_error("Usage: keypackage publish");
        return Ok(());
    }

    if args[0] == "publish" {
        ui::print_info("Publishing KeyPackages to DHT...");

        let client = self.client.lock().await;
        client.publish_keypackages().await?;

        ui::print_success("Published 10 KeyPackages to DHT");
        println!();
        println!("  Others can now add you to MLS groups");
        println!();
    }

    Ok(())
}
```

Update help text to include:

```rust
println!("{}", "  MLS Encryption:".bright_yellow());
println!("    {} - Publish KeyPackages to DHT", "keypackage publish".bright_green());
println!("    {} <space> <user> - Add member to MLS group", "member add".bright_green());
```

#### 1.2 Add KeyPackage publication to Client

**File**: `core/src/client.rs`

The code already exists! Check lines 1179-1204:

```rust
pub async fn publish_keypackages(&self) -> Result<()>
```

This method:

- Generates 10 KeyPackages
- Publishes them to DHT under key `keypackage:<user_id_hex>`
- Includes expiration timestamps

### Phase 2: Adding Members to MLS Group (Alice adds Bob)

**Current Flow**:

```
Bob joins with invite → Added to CRDT Space ✅ → (Not added to MLS group ❌)
```

**Needed Flow**:

```
Bob joins with invite → Added to CRDT Space ✅ → Alice adds Bob to MLS group ✅ → Bob receives Welcome message ✅
```

**Implementation**:

#### 2.1 Add CLI command for MLS member addition

**File**: `cli/src/commands.rs`

Add to `handle_command()`:

```rust
"member" => self.cmd_member(&parts[1..]).await,
```

Add new method:

```rust
async fn cmd_member(&mut self, args: &[&str]) -> Result<()> {
    let space_id = self.current_space.context("No space selected. Use: space <id>")?;

    if args.is_empty() {
        ui::print_error("Usage: member add <user_id>");
        ui::print_info("This adds the user to the MLS encryption group");
        return Ok(());
    }

    if args[0] == "add" {
        if args.len() < 2 {
            ui::print_error("Usage: member add <user_id>");
            return Ok(());
        }

        // Parse user_id from hex string
        let user_id_str = args[1];
        let decoded = hex::decode(user_id_str)
            .context("Invalid user ID hex")?;
        if decoded.len() != 32 {
            ui::print_error(&format!("User ID must be 32 bytes (64 hex chars), got {} bytes", decoded.len()));
            return Ok(());
        }
        let mut user_id_bytes = [0u8; 32];
        user_id_bytes.copy_from_slice(&decoded);
        let user_id = spaceway_core::types::UserId(user_id_bytes);

        ui::print_info(&format!("Adding user {} to MLS encryption group...", hex::encode(&user_id.0[..8])));

        let client = self.client.lock().await;
        match client.add_member_with_mls(
            space_id,
            user_id,
            spaceway_core::types::Role::Member
        ).await {
            Ok(_) => {
                ui::print_success(&format!("User {} added to MLS group!", hex::encode(&user_id.0[..8])));
                println!();
                println!("  ✓ KeyPackage fetched from DHT");
                println!("  ✓ User added to encryption group");
                println!("  ✓ Welcome message sent");
                println!("  ✓ Epoch rotated (new keys)");
                println!();
            }
            Err(e) => {
                ui::print_error(&format!("Failed to add member to MLS: {}", e));
                println!();
                println!("  Make sure the user has published KeyPackages:");
                println!("  $ keypackage publish");
                println!();
            }
        }
    }

    Ok(())
}
```

#### 2.2 Fix the deadlock in `add_member_with_mls()`

**File**: `core/src/client.rs`, lines ~1410-1440

The current code has the **same deadlock pattern** - it holds `space_manager.write()` while calling network operations.

**Current (BUGGY)**:

```rust
let mut manager = self.space_manager.write().await;
let (commit_msg, welcome_msg) = manager.add_member_with_mls(...)?;
let op = manager.add_member(...)?;
drop(provider);
drop(manager);

// Still holding manager lock here? Need to verify
self.store.put_op(&op)?;
self.broadcast_op(&op).await?;  // ❌ DEADLOCK if manager still locked
```

**Should be** (scope-based pattern):

```rust
let (op, commit_msg, welcome_msg) = {
    let provider = self.mls_provider.read().await;
    let mut manager = self.space_manager.write().await;

    let (commit_msg, welcome_msg) = manager.add_member_with_mls(...)?;
    let op = manager.add_member(...)?;

    (op, commit_msg, welcome_msg)
}; // ✅ Lock dropped here

// Safe now - no locks held
self.store.put_op(&op)?;
// Publish MLS messages...
self.broadcast_op(&op).await?;
```

### Phase 3: Subscribe to Welcome Messages (Bob receives)

**Current Status**: ✅ Already implemented!

The code in `client.rs` lines 240-280 already handles this:

```rust
// Subscribe to personal Welcome messages
let user_topic = format!("user/{}/welcome", self.user_id);
network.subscribe(&user_topic)?;

// In event loop:
if topic.contains("/welcome") {
    println!("  🎉 Received MLS Welcome message");
    // Process Welcome message to join MLS group
    match crate::mls::MlsGroup::from_welcome(...) {
        Ok((group, _)) => {
            // Store the MLS group
            manager.store_mls_group(&space_id, group)?;
        }
    }
}
```

**This works automatically!** Bob just needs to be running his client.

### Phase 4: Message Encryption/Decryption

**Current Flow**:

```
Alice sends message → Plaintext over GossipSub → Bob receives plaintext
```

**Needed Flow**:

```
Alice sends message → Encrypted with MLS → Sent over GossipSub → Bob decrypts with MLS
```

**Implementation**:

#### 4.1 Check current message sending

**File**: `core/src/client.rs`, search for `post_message()`

Look at line ~1604. The message is currently sent as plaintext in the CRDT operation.

**For encryption**, we need to:

1. Serialize the message content
2. Encrypt it using `mls_group.encrypt_application_message()`
3. Include encrypted blob in the operation
4. On receive, decrypt using `mls_group.decrypt_application_message()`

**File**: `core/src/client.rs`, `post_message()` method:

```rust
pub async fn post_message(
    &self,
    space_id: SpaceId,
    thread_id: ThreadId,
    content: String,
) -> Result<(Message, CrdtOp)> {
    // Check if we have an MLS group for this space
    let encrypted_content = {
        let manager = self.space_manager.read().await;
        if let Some(mls_group) = manager.get_mls_group(&space_id) {
            // Encrypt with MLS
            let provider = self.mls_provider.read().await;
            let encrypted = mls_group.encrypt_application_message(
                content.as_bytes(),
                &provider
            )?;
            Some(encrypted)
        } else {
            None
        }
    };

    let op = {
        let mut manager = self.thread_manager.write().await;
        if let Some(encrypted) = encrypted_content {
            // Create message with encrypted content
            manager.post_encrypted_message(thread_id, encrypted, ...)?
        } else {
            // Fallback to plaintext
            manager.post_message(thread_id, content, ...)?
        }
    };

    self.store.put_op(&op)?;
    self.broadcast_op(&op).await?;

    Ok((msg, op))
}
```

### Phase 5: Key Rotation on Member Removal

**Current Flow**:

```
Alice kicks Bob → Bob removed from CRDT Space ✅ → (Still in MLS group ❌)
```

**Needed Flow**:

```
Alice kicks Bob → Bob removed from CRDT Space ✅ → Bob removed from MLS group ✅ → New epoch (keys rotated) ✅
```

**Implementation**:

Similar to `add_member_with_mls()`, implement `remove_member_with_mls()`:

```rust
pub async fn remove_member_with_mls(
    &self,
    space_id: SpaceId,
    user_id: UserId,
) -> Result<CrdtOp> {
    let (op, commit_msg) = {
        let provider = self.mls_provider.read().await;
        let mut manager = self.space_manager.write().await;

        // Remove from MLS group (triggers epoch++, key rotation)
        let commit_msg = manager.remove_member_from_mls(
            &space_id,
            user_id,
            &provider,
        )?;

        // Remove from CRDT space
        let op = manager.remove_member(space_id, user_id, self.user_id, &self.keypair)?;

        (op, commit_msg)
    }; // ✅ Lock dropped

    // Publish Commit to remaining members
    let space_topic = format!("space/{}", hex::encode(&space_id.0[..8]));
    let commit_bytes = commit_msg.to_bytes()?;
    {
        let mut network = self.network.write().await;
        network.publish(&space_topic, commit_bytes).await?;
    }

    self.store.put_op(&op)?;
    self.broadcast_op(&op).await?;

    Ok(op)
}
```

## Implementation Checklist

### Phase 1: KeyPackage Management ✅

- [x] `publish_keypackages()` already exists in Client
- [ ] Add `keypackage publish` CLI command
- [ ] Update help text

### Phase 2: Adding to MLS Group

- [x] `add_member_with_mls()` already exists in Client
- [ ] Fix deadlock in `add_member_with_mls()`
- [ ] Add `member add <user_id>` CLI command
- [ ] Update help text

### Phase 3: Welcome Messages ✅

- [x] Welcome message subscription already exists
- [x] Welcome message processing already exists
- No changes needed!

### Phase 4: Message Encryption

- [ ] Modify `post_message()` to encrypt with MLS
- [ ] Modify message reception to decrypt with MLS
- [ ] Add fallback to plaintext if no MLS group

### Phase 5: Key Rotation on Removal

- [ ] Implement `remove_member_with_mls()`
- [ ] Fix deadlock in `remove_member()` (same pattern)
- [ ] Update `cmd_kick()` to use MLS removal

### Phase 6: Channel-Level Encryption

- [ ] Create per-channel MLS groups
- [ ] Handle channel-specific key rotation
- [ ] Bob removed from channel → can't decrypt channel messages
- [ ] Bob still in space → can decrypt space-level messages

## Testing Plan

### Test 1: Basic E2EE Flow

```bash
# Terminal 1 (Alice)
$ cargo run --release -- --account alice.key --port 9001
> keypackage publish
> space create "encrypted-test"
> invite create

# Terminal 2 (Bob)
$ cargo run --release -- --account bob.key --port 9002
> keypackage publish
> connect /ip4/127.0.0.1/tcp/9001/p2p/...
> join <space_id> <invite_code>

# Terminal 1 (Alice)
> member add <bob_user_id>  # Add Bob to MLS group
> channel create "general"
> thread create "Hello"
> send "This is encrypted!"

# Terminal 2 (Bob)
> space <space_id>
> channel general
> thread Hello
> messages  # Should see decrypted message ✅
```

### Test 2: Key Rotation

```bash
# Terminal 1 (Alice)
> kick <bob_user_id>  # Remove Bob from MLS group

> send "Bob can't see this"  # New epoch, new keys

# Terminal 2 (Bob)
> messages  # Should NOT see new message ✅
            # Should see: "⚠️ Failed to decrypt (not in group)"
```

## Security Properties

Once fully implemented:

✅ **Forward Secrecy**: Bob can't decrypt messages after being kicked (new epoch, new keys)
✅ **Post-Compromise Security**: If Bob's keys are compromised, he can't decrypt future messages after key rotation
✅ **Group Authentication**: Only group members can send valid encrypted messages
⚠️ **Metadata Privacy**: Message metadata (who sent to whom, when) is NOT encrypted (limitation of GossipSub)

## Notes

1. **Two-Layer Design is Intentional**:

   - CRDT layer = offline-capable access control
   - MLS layer = online encryption with key rotation
   - Allows invites to work offline, encryption to sync online

2. **DHT Quorum Challenges**:

   - With 2 peers, DHT quorum might fail (needs 20 peers)
   - Solution: Use `add_member_with_key_package_bundle()` for direct P2P KeyPackage exchange
   - CLI could support: `member add-p2p <user_id> <keypackage_file>`

3. **Channel-Level vs Space-Level**:

   - Current implementation: One MLS group per Space
   - Future: One MLS group per Channel for finer-grained access
   - Allows: Bob kicked from #admin but still in #general

4. **Performance**:
   - MLS encryption is fast (symmetric crypto)
   - Key rotation is slow (asymmetric crypto)
   - Limit group size to ~100 members for good performance

## Next Steps

1. **Start with CLI commands** (easiest):
   - Add `keypackage publish` command ✅
   - Add `member add <user_id>` command
2. **Fix deadlocks** (critical):
   - Fix `add_member_with_mls()` deadlock
   - Fix `remove_member()` deadlock
3. **Test basic flow**:
   - Bob publishes KeyPackages
   - Alice adds Bob to MLS group
   - Bob receives Welcome message
   - Verify MLS group is stored
4. **Implement encryption** (big one):
   - Encrypt message content in `post_message()`
   - Decrypt message content on receive
   - Test end-to-end
5. **Test key rotation**:
   - Alice kicks Bob
   - Verify Bob can't decrypt new messages
   - Verify Alice can still decrypt
