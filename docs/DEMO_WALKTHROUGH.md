# Descord Live Demo - Step by Step

This guide walks you through a live demo with two users (Alice and Bob) interacting in real-time.

## Prerequisites

Build the release binary:
```powershell
cargo build --release --bin descord
```

## Setup

You'll need **TWO separate terminal windows** - one for Alice, one for Bob.

### Terminal 1 - Alice's Window

```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-alice.key
```

### Terminal 2 - Bob's Window

```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-bob.key
```

**Important**: Each user automatically gets their own data directory:
- Alice: `demo-alice-data/`
- Bob: `demo-bob-data/`

## Demo Script

### Part 1: Alice Creates a Community

**In Alice's terminal:**

```
alice> space create Tech Community
```
✓ Alice creates a new Space. **Copy the Space ID** (first 8 chars shown).

```
alice> channel create general
```
✓ Alice creates a channel named "general".

```
alice> thread create Introductions
```
✓ Alice creates a thread for people to introduce themselves.

```
alice> send Welcome to Tech Community! This is a decentralized forum.
```
✓ Alice sends the first message.

```
alice> send Feel free to introduce yourself and share what you're working on!
```
✓ Alice sends a second message.

```
alice> messages
```
✓ Alice views all messages in the thread.

```
alice> invite create
```
✓ Alice creates an invite code. **Copy the invite code** (e.g., `E0ZBSNwi`).

### Part 2: Bob Joins the Community

**In Bob's terminal:**

```
bob> join <SPACE_ID> <INVITE_CODE>
```
Replace `<SPACE_ID>` and `<INVITE_CODE>` with the values Alice created.

Example:
```
bob> join 75ff1615c470e031 E0ZBSNwi
```

✓ Bob joins Alice's Space using the invite code.

**Important**: If you see errors about "No space selected", continue with these steps:

```
bob> spaces
```
✓ Bob lists all spaces (should see "Tech Community").

```
bob> space <SPACE_ID_PREFIX>
```
Use the first 4-8 characters of the Space ID. Example:
```
bob> space 75ff
```

✓ Bob switches to the Tech Community space.

```
bob> channels
```
✓ Bob lists channels (should see "general").

```
bob> channel <CHANNEL_ID_PREFIX>
```
Use the first 4-8 characters shown. Example:
```
bob> channel a1b2
```

✓ Bob switches to the general channel.

```
bob> threads
```
✓ Bob lists threads (should see "Introductions").

```
bob> thread <THREAD_ID_PREFIX>
```
Use the first 4-8 characters. Example:
```
bob> thread c3d4
```

✓ Bob switches to the Introductions thread.

```
bob> messages
```
✓ **Bob should now see Alice's welcome messages!** 🎉

### Part 3: Bob Responds

**In Bob's terminal:**

```
bob> send Hi Alice! Thanks for creating this community.
```
✓ Bob sends his first message.

```
bob> send I'm excited to be here and learn about decentralized tech!
```
✓ Bob sends a second message.

### Part 4: Alice Sees Bob's Messages

**Back in Alice's terminal:**

```
alice> refresh
```
✓ Alice refreshes the network to sync new messages.

```
alice> messages
```
✓ **Alice should now see Bob's messages!** 🎉

### Part 5: Continue the Conversation

Both Alice and Bob can now freely send messages:

```
alice> send Great to have you here, Bob! What are you interested in?
```

```
bob> send I'm learning about CRDTs and p2p networking. This is amazing!
```

```
alice> messages
```

```
bob> messages
```

## What You've Demonstrated

✅ **Decentralized Space Creation** - Alice created a Space without any central server  
✅ **Invite System** - Bob joined using an invite code  
✅ **Hierarchical Structure** - Space → Channel → Thread organization  
✅ **Real-time Messaging** - Alice and Bob exchanging messages  
✅ **Multi-user Operation** - Two separate CLI instances running simultaneously  
✅ **Automatic Data Isolation** - Each user has their own storage  
✅ **Offline Capabilities** - Messages stored in DHT for offline retrieval  

## Advanced Features to Try

### File Upload

**Alice uploads a file:**
```
alice> upload README.md
```
✓ File is encrypted and uploaded to the DHT.

**Bob retrieves it:**
```
bob> refresh
bob> messages
```
✓ Bob sees the file attachment (once blob retrieval is implemented).

### DHT Joining (Without Invite)

**Simulate offline join** (works even if Alice is offline):
```
bob> join dht <SPACE_ID>
```
✓ Bob can join public spaces directly from DHT.

### Multiple Spaces

**Alice creates another space:**
```
alice> space create Gaming Chat
alice> spaces
```

**Switch between spaces:**
```
alice> space 75ff    # Back to Tech Community
alice> space a1b2    # To Gaming Chat
```

## Tips for Smooth Demo

1. **Use separate terminal windows** - Run Alice and Bob side-by-side so you can see both at once
2. **Copy IDs carefully** - Use Ctrl+C to copy Space IDs and invite codes
3. **Use short prefixes** - You only need 4-8 characters of any ID
4. **Run `context`** - Check where you are if you get lost
5. **Run `refresh`** - Sync new messages before viewing
6. **Check `messages` frequently** - See the conversation evolve

## Common Issues

### "Failed to create lock file"
- Make sure Alice and Bob have **different account names** (e.g., `demo-alice.key` vs `demo-bob.key`)
- Each account gets its own data directory automatically

### "Command not recognized"
- You're typing commands in the Windows command prompt instead of inside the Descord CLI
- Wait for the `alice>` or `bob>` prompt before typing commands

### "No space selected"
- Run `spaces` to list available spaces
- Run `space <id>` to select one

### Messages don't appear
- Run `refresh` to sync the network
- Make sure you're in the same thread (run `context` to check)

---

**Enjoy your decentralized chat! 🚀**
