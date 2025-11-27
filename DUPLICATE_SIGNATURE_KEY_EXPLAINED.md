# Understanding "DuplicateSignatureKey" Error

## The Error

```
✗ Failed to add member to MLS: Cryptographic operation failed: Failed to add member to MLS group: CreateCommitError(ProposalValidationError(DuplicateSignatureKey))
```

## What It Means

**The user is already in the MLS encryption group!**

Each user can only be added to an MLS group once. This is a cryptographic requirement - you can't have duplicate identity keys in the same group.

## Common Scenarios

### Scenario 1: Trying to Add the Space Creator

When Alice creates a space, **she is automatically added to the MLS group**:

```bash
alice> space create "my-space"
# ✅ Alice automatically added to MLS group during creation
```

If Alice then tries to add herself:

```bash
alice> member add <alice_user_id>
# ❌ Error: DuplicateSignatureKey (Alice is already in the group!)
```

**Solution**: Don't add the creator - they're already in the MLS group!

### Scenario 2: Adding the Same User Twice

```bash
alice> member add <bob_user_id>
# ✅ Success! Bob added to MLS group

alice> member add <bob_user_id>
# ❌ Error: DuplicateSignatureKey (Bob is already in the group!)
```

**Solution**: Check who's already in the group before adding.

### Scenario 3: Bob Hasn't Joined the Space (CRDT Layer)

This is the **correct workflow**:

```bash
# Step 1: Bob joins the Space (CRDT layer)
bob> join <space_id> <invite_code>
# ✅ Bob is now in the Space (can see it in `space list`)

# Step 2: Alice adds Bob to MLS group (encryption layer)
alice> member add <bob_user_id>
# ✅ Success! Bob is now in the MLS group
```

If you skip Step 1 and try to add Bob to MLS without him joining the space first, it might work cryptographically but Bob won't receive updates properly.

## Updated Error Messages

The CLI now provides clearer error messages:

### If User Already in MLS Group:

```
✗ User is already in the MLS encryption group!

  This user has already been added to the MLS group for this space.
  Each user can only be added once.

  Note: Space creators are automatically added to their MLS group.
```

### If User Hasn't Published KeyPackages:

```
✗ User hasn't published KeyPackages yet

  Tell the user to run:
  > keypackage publish
```

### If DHT Quorum Not Reached:

```
✗ DHT quorum not reached

  Need more peers in the network to fetch KeyPackages from DHT.
  For 2-peer setup, consider direct KeyPackage exchange.
```

## How to Check Who's in the MLS Group

Currently, there's no direct CLI command to list MLS group members (it's stored internally). However, you can infer it from the workflow:

1. **Space Creator**: Automatically in MLS group
2. **Members Added via `member add`**: In MLS group
3. **Members Joined via Invite Only**: NOT in MLS group (until explicitly added)

## Recommended Workflow

### For Alice (Space Admin):

```bash
# 1. Create space (you're auto-added to MLS group)
> space create "secret-project"

# 2. Publish KeyPackages (so others can add you to their spaces)
> keypackage publish

# 3. Create invite for Bob
> invite create

# 4. Wait for Bob to join...

# 5. Check members
> members

# 6. Add Bob to MLS group (copy his full user ID from members list)
> member add <bob_full_user_id_64_chars>
```

### For Bob (Joining User):

```bash
# 1. Publish KeyPackages FIRST (before joining)
> keypackage publish

# 2. Connect to Alice
> connect /ip4/127.0.0.1/tcp/9001/p2p/...

# 3. Join the space with invite code
> join <space_id> <invite_code>

# 4. Wait for Alice to add you to MLS group
# You'll see: "🎉 Received MLS Welcome message"
```

## Why Two Layers?

Spaceway uses a **two-layer membership system**:

### Layer 1: CRDT Space Membership

- **Purpose**: Access control (who can see the space, channels, etc.)
- **Added via**: Invite codes, `join` command
- **Works**: Offline, eventually consistent
- **Check with**: `members` command

### Layer 2: MLS Encryption Group

- **Purpose**: Message encryption (who can decrypt messages)
- **Added via**: `member add <user_id>` command
- **Works**: Online only, requires coordination
- **Check with**: No direct command (internal state)

**You need BOTH** to fully participate:

- ✅ CRDT membership: Can see spaces, channels, threads
- ✅ MLS membership: Can decrypt message content

## Testing the Full Flow

**Terminal 1 (Alice)**:

```bash
$ cargo run --release -- --account alice.key --port 9001
> keypackage publish
> space create "test"
> invite create
> whoami  # Check your user ID
```

**Terminal 2 (Bob)**:

```bash
$ cargo run --release -- --account bob.key --port 9002
> keypackage publish
> connect /ip4/127.0.0.1/tcp/9001/p2p/12D3KooW...
> join <space_id> <invite_code>
> whoami  # Copy your full User ID (64 hex chars)
```

**Terminal 1 (Alice)**:

```bash
> members  # Verify Bob is in the Space
> member add <bob_full_user_id>  # Add Bob to MLS group
# ✅ Should succeed (Bob not in MLS group yet)
```

**Terminal 2 (Bob)**:

```bash
# Should see:
  🎉 Received MLS Welcome message
  ✓ Joined MLS group for space space_...
```

**Terminal 1 (Alice)** - Try adding Bob again:

```bash
> member add <bob_full_user_id>
# ❌ Should fail with: "User is already in the MLS encryption group!"
```

This is expected behavior! ✅

## Build Status

✅ **Build successful**: 9.37s
✅ **Improved error messages**: Now clearly explains DuplicateSignatureKey
✅ **Ready to test**: Full MLS workflow

## Summary

The `DuplicateSignatureKey` error is **not a bug** - it's MLS working correctly! It means:

1. ✅ The user is already in the MLS group
2. ✅ You can't add them twice (by design)
3. ✅ Space creators are auto-added

**Solution**: Only add users who have joined the space (via invite) but haven't been added to the MLS group yet.
