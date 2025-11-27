# How to Add Bob to MLS Group

## What Was Fixed

### Issue 1: Missing Event Handlers

Bob's `space list` showed nothing because the event processing loop was missing handlers for:

- `CreateInvite` - Creating invite operations
- `RevokeInvite` - Revoking invites
- `UseInvite` - When someone uses an invite code
- `AddMember` - Adding members to spaces
- `RemoveMember` - Removing members from spaces

These handlers are now added, so Bob will properly receive and process all space operations.

### Issue 2: Understanding the MLS Flow

There are **two ways** Bob can join the MLS group:

## Option 1: Manual MLS Addition (NOT YET IMPLEMENTED)

This would require:

1. Bob publishes his KeyPackages to DHT
2. Alice fetches Bob's KeyPackage
3. Alice adds Bob to MLS group with `add_member_with_mls`
4. Alice sends Welcome message to Bob
5. Bob receives Welcome and joins MLS group

**Problem**: The CLI doesn't have commands for this yet. You'd need to add:

- `keypackage publish` command
- `member add-mls <space_id> <user_id>` command

## Option 2: Automatic via Invite (CURRENT APPROACH)

The invite system is designed for **plaintext collaboration** by default. Here's what currently happens:

### Current Flow:

1. ✅ Alice creates space (with MLS group, Alice is only member)
2. ✅ Alice creates invite
3. ✅ Bob joins with invite code
4. ✅ Bob is added to the Space as a member (via CRDT)
5. ⚠️ Bob is **NOT** added to MLS group automatically
6. ⚠️ Alice's messages are encrypted (Bob can't decrypt them)
7. ⚠️ Bob's messages are sent in plaintext

### Why This Design?

The current design separates two concerns:

- **Space membership** (CRDT-based, works offline)
- **MLS group membership** (requires online coordination)

This allows:

- Offline invite acceptance
- Gradual MLS adoption
- Spaces that mix encrypted/plaintext members

## Testing the Current State

Run this flow to test what's working:

```bash
# Terminal 1 - Alice
./target/release/spaceway -p 9001
space create test
invite create
# Copy the invite command shown

# Terminal 2 - Bob (in another terminal)
./target/release/spaceway -p 9002
connect /ip4/127.0.0.1/tcp/9001
# Paste the join command from Alice
space list  # Should now show "test" space!
```

## What Works Now

✅ Space creation
✅ Invite creation
✅ Invite join (Bob becomes a member)
✅ `space list` shows spaces Bob has joined
✅ Bob can see he's a member of the space
✅ CRDT operations sync properly

## What Doesn't Work Yet

❌ Bob cannot decrypt Alice's encrypted messages
❌ No CLI command to add Bob to MLS group
❌ Bob's KeyPackages aren't published to DHT automatically

## Next Steps to Enable Full MLS

### Option A: Add CLI Commands

Add these commands to `cli/src/main.rs`:

```rust
// Publish KeyPackages
"keypackage" | "kp" => {
    if args.len() < 2 {
        println!("Usage: keypackage <publish|list>");
        continue;
    }
    match args[1].as_str() {
        "publish" => {
            client.publish_key_packages_to_dht().await?;
            println!("✓ Published KeyPackages to DHT");
        }
        _ => println!("Unknown subcommand"),
    }
}

// Add member with MLS
"member" => {
    if args.len() < 2 {
        println!("Usage: member add-mls <space_id> <user_id>");
        continue;
    }
    match args[1].as_str() {
        "add-mls" => {
            let space_id = SpaceId::from_hex(&args[2])?;
            let user_id = UserId::from_hex(&args[3])?;
            client.add_member_with_mls(space_id, user_id, Role::Member).await?;
            println!("✓ Added member with MLS encryption");
        }
        _ => println!("Unknown subcommand"),
    }
}
```

### Option B: Auto-add to MLS on Invite

Modify `join_with_invite()` to:

1. Publish Bob's KeyPackages to DHT first
2. After joining, trigger Alice to add Bob to MLS
3. Requires Alice to monitor for new joins

## Summary

The **deadlock fixes are complete** and Bob can now **successfully join spaces via invite**. The `space list` command will work now.

For full end-to-end encryption, you'll need to either:

1. Manually add CLI commands for MLS (Option A)
2. Implement auto-MLS on invite (Option B)

The underlying MLS infrastructure is all there - it just needs CLI exposure or automation!
