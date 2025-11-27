# MLS Encryption Kick Scenario - Complete Tutorial

## Overview

This tutorial demonstrates the **MLS (Messaging Layer Security) encryption** feature of Spaceway. You'll see how:

1. Alice creates a space and channel
2. Bob joins and can read/write messages
3. **Alice kicks Bob** from the space
4. **Bob can no longer decrypt new messages** (MLS group membership removed)

## Prerequisites

- Dashboard backend running on `http://localhost:3030`
- Dashboard frontend running on `http://localhost:5173`
- Both Alice and Bob clients initialized

---

## Step-by-Step Scenario

### Step 0: Connect the Peers (Required!)

**Why**: Without P2P connections, DHT and GossipSub operations fail.

```
Tab: Dashboard
Client: Alice (doesn't matter)
Action: 🔗 Connect Peers (P2P Network)
```

**Expected Result:**

```
✓ Peer connections initiated. Alice, Bob, and Charlie are now connected!
```

**Backend logs should show:**

```
INFO dashboard_backend: 🔗 Connecting all peers together...
INFO dashboard_backend: Alice: 12D3KooW... at ["/ip4/127.0.0.1/tcp/XXXXX"]
INFO dashboard_backend: Bob: 12D3KooW... at ["/ip4/127.0.0.1/tcp/XXXXX"]
INFO dashboard_backend: ✓ Bob connected to Alice
INFO dashboard_backend: ✓ Charlie connected to Alice
```

---

### Step 1: Alice Creates a Space

```
Client: Alice
Action: CreateSpace
Space Name: "red"
```

**Expected Result:**

```
✓ Created space 'red' with ID: b081442b...
```

**📋 Copy the full Space ID** (click the Copy ID button next to Alice's space in the Dashboard)

---

### Step 2: Alice Creates a Channel

```
Client: Alice
Action: CreateChannel
Space ID: [paste full 64-char Space ID]
Channel Name: "general"
```

**Expected Result:**

```
✓ Created channel 'general' with ID: a5f3...
```

**📋 Copy the full Channel ID** from the result message

---

### Step 3: Alice Creates an Invite

```
Client: Alice
Action: CreateInvite
Space ID: [paste Space ID]
```

**Expected Result:**

```
✓ Created invite! Code: ph18Csmh (Space: b081442b...)
```

---

### Step 4: Bob Joins the Space

```
Client: Bob
Action: JoinSpace
Space ID: [paste Space ID]
```

**Expected Result:**

```
✓ Joined space 'red'
```

**Dashboard should now show:**

- Alice's spaces: 1 space ("red") with Members: 2
- Bob's spaces: 1 space ("red") with Members: 2

---

### Step 5: Alice Creates a Thread in the Channel

```
Client: Alice
Action: CreateThread
Space ID: [paste Space ID]
Channel ID: [paste Channel ID]
Thread Title: "Welcome"
First Message: "Hello everyone! This is the first message."
```

**Expected Result:**

```
✓ Created thread 'Welcome' with ID: 7e4a... (first message: 'Hello everyone! This is the first...')
```

**📋 Copy the Thread ID** from the result

---

### Step 6: Bob Sends a Message (Encrypted with MLS)

```
Client: Bob
Action: SendMessage
Space ID: [paste Space ID]
Thread ID: [paste Thread ID]
Message: "Hi Alice! I can read and write messages because I'm in the MLS group!"
```

**Expected Result:**

```
✓ Sent message: 'Hi Alice! I can read and write messages...' (ID: 3f9a...)
```

**What happened:**

- Bob's message was **encrypted using MLS** (end-to-end encryption)
- Both Alice and Bob can decrypt this message because they're both in the Space's MLS group
- The message traveled over the P2P network encrypted

---

### Step 7: Alice Sends Another Message

```
Client: Alice
Action: SendMessage
Space ID: [paste Space ID]
Thread ID: [paste Thread ID]
Message: "Great to have you here Bob!"
```

**Expected Result:**

```
✓ Sent message: 'Great to have you here Bob!' (ID: 8c2d...)
```

---

### Step 8: 🚫 Alice Kicks Bob from the Space

**This is the critical step!**

First, get Bob's User ID from the Dashboard. You'll see it displayed as "User ID: 91ead4a2..." but you need the **full 64-character hex string**.

**How to get Bob's full User ID:**

1. Look at the backend terminal logs when Bob was created
2. Find the line: `✓ Bob created: 91ead4a23a5259a5...` (this is shortened)
3. The full User ID is 32 bytes = 64 hex characters
4. Or check the dashboard state JSON for Bob's user_id

```
Client: Alice
Action: 🚫 Remove Member (Kick)
Space ID: [paste Space ID - 64 chars]
User ID: [paste Bob's full User ID - 64 chars]
```

**Expected Result:**

```
✓ Removed member 91ead4a2... from space. They can no longer decrypt new messages!
```

**What happened:**

- Alice initiated an **MLS group commit** that removes Bob
- Bob's MLS credentials are **removed from the group**
- The MLS group **epoch increments** (forward secrecy)
- Bob **cannot decrypt messages encrypted with the new epoch**

**Backend logs should show:**

```
🔐 MLS: Removing member from group (Commit operation)
✓ Member removed, new epoch: 3
```

---

### Step 9: Alice Sends a Message After Kicking Bob

```
Client: Alice
Action: SendMessage
Space ID: [paste Space ID]
Thread ID: [paste Thread ID]
Message: "This message is encrypted with the new MLS epoch. Bob cannot read this!"
```

**Expected Result:**

```
✓ Sent message: 'This message is encrypted with the new...' (ID: 1a7b...)
```

**🔐 MLS Security Property:**

- This message is encrypted with **epoch 3** (after Bob was removed)
- Bob only has keys for **epochs 1-2** (before removal)
- **Bob CANNOT decrypt this message** even if he intercepts the ciphertext!

---

### Step 10: 🚫 Bob Tries to Send a Message (Should Fail)

```
Client: Bob
Action: SendMessage
Space ID: [paste Space ID]
Thread ID: [paste Thread ID]
Message: "Can I still send messages?"
```

**Expected Result (should fail):**

```
✗ Action failed: Not a member of this space / MLS group membership missing
```

**What happened:**

- Bob's client recognizes he's no longer in the MLS group
- He cannot create MLS-encrypted messages for this space
- His send attempt is rejected

---

## Verification: Dashboard State

**Alice's View:**

```
Spaces: 1
  - Space "red" (ID: b081442b...)
    Channels: 1 ("general")
    Members: 1  ← Only Alice now!
```

**Bob's View:**

```
Spaces: 0 or 1*
  ← Bob may still see the space locally, but:
     - Cannot decrypt new messages
     - Cannot send messages
     - MLS group membership: REMOVED
```

---

## MLS Encryption Properties Demonstrated

### ✅ Forward Secrecy

- Old messages (epochs 1-2): Bob can still decrypt messages sent BEFORE he was kicked
- New messages (epoch 3+): Bob CANNOT decrypt messages sent AFTER he was kicked
- Each epoch has different encryption keys

### ✅ Post-Compromise Security

- If Bob's device is compromised AFTER being kicked, attacker gets:
  - ✓ Keys for epochs 1-2 (old messages Bob already saw)
  - ✗ Keys for epochs 3+ (new messages - Bob never had these keys)

### ✅ End-to-End Encryption

- All messages encrypted client-side before transmission
- Server (or P2P network) never sees plaintext
- Only current MLS group members can decrypt

### ✅ Group Membership Enforcement

- Removing a member = removing their decryption capability
- Instant effect: next message uses new epoch keys
- No backdoors: kicked member truly cannot read new content

---

## Backend Implementation Details

### MLS Operations

1. **Space Creation**: Creates MLS group with Alice as founder
2. **Join Space**: Adds Bob to MLS group (Welcome message + group state)
3. **Send Message**: Encrypts with current epoch's group key
4. **Remove Member**: MLS Commit removes Bob, increments epoch, generates new keys

### Key Methods Called

```rust
// Space creation
client.create_space(name, None)
  → Creates MLS group

// Bob joins
client.join_space_from_dht(space_id)
  → Processes MLS Welcome message

// Send message
client.post_message(space_id, thread_id, content)
  → Encrypts with MLS

// Kick Bob
client.remove_member(space_id, bob_user_id)
  → MLS Commit, epoch++, new keys
```

---

## Troubleshooting

### "DHT GET failed: NotFound"

- **Solution**: Run Step 0 (Connect Peers) first!
- DHT operations require P2P connectivity

### "Space not found"

- Make sure you copied the **full 64-character Space ID**
- Use the 📋 Copy ID button, don't truncate

### "Invalid user_id length"

- User ID is **16 hex characters** (8 bytes)
- Example: `91ead4a23a5259a5`
- Find it in the Dashboard under each client

### Bob can still send messages after kick

- Check backend logs for MLS commit success
- Verify Dashboard shows "Members: 1" for Alice
- Try restarting Bob's client (in production, would sync removal via gossip)

### "Invalid user_id length"

- User ID is **64 hex characters** (32 bytes)
- Same length as Space ID
- Check backend terminal logs for the full User ID when clients are created
- Example: `91ead4a23a5259a5...` (need all 64 chars)

---

## Real-World Implications

This scenario demonstrates how Spaceway would handle:

- **Employee offboarding**: Removing employee from company chat (can't read new discussions)
- **Moderation**: Kicking abusive users from community channels
- **Confidential discussions**: Ensuring removed members can't access future sensitive info
- **Access revocation**: Immediate effect when someone leaves a group

The MLS protocol ensures these properties hold even if:

- The kicked user has copies of old messages
- The kicked user monitors network traffic
- The kicked user compromises relay servers

**Cryptographic guarantee**: Without the group's private keys for the current epoch, decryption is computationally infeasible.

---

## Next Steps

Try these variations:

1. **Bob rejoins**: Alice creates new invite → Bob joins again (gets current epoch keys)
2. **Multiple channels**: Create multiple channels, kick Bob from one but not others
3. **Charlie as moderator**: Give Charlie permission to kick Bob
4. **Message history**: Verify Bob can still read old messages in local storage

## Related Documentation

- `CONNECT_PEERS_FEATURE.md` - P2P network setup
- `TUTORIAL_FEATURE.md` - Basic workflow
- MLS Specification: https://messaginglayersecurity.rocks/
