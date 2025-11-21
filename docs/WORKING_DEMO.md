# Working Two-User Demo - Complete Guide

This guide explains the **correct workflow** for Alice and Bob to connect and chat.

## Understanding the Architecture

Descord uses **GossipSub** for peer-to-peer messaging. This means:
- ✅ Peers must be **connected** to exchange messages
- ✅ Bob must **subscribe to Alice's Space topic** to receive operations
- ✅ Once connected, messages sync automatically via GossipSub
- ⚠️  DHT warnings are normal with only 2 peers (needs 3+ for quorum)

## The Correct Workflow

### Step 1: Start Alice (Listening)

**Terminal 1:**
```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-alice.key --port 9001
```

Alice will start listening on port 9001.

### Step 2: Alice Gets Her Peer Address

**In Alice's terminal:**
```
demo-alice> network
```

Output:
```
Network Status:
  Peer ID: 12D3KooWRUJT4o5j...  
  Listening on: 1
    /ip4/0.0.0.0/tcp/9001

📋 Share this multiaddr for others to connect:
  /ip4/0.0.0.0/tcp/9001/p2p/12D3KooWRUJT4o5j...
```

**Copy the multiaddr** and change `/ip4/0.0.0.0/` to `/ip4/127.0.0.1/`.

Example result:
```
/ip4/127.0.0.1/tcp/9001/p2p/12D3KooWRUJT4o5jAbCdEfGhIjKlMnOpQrStUvWxYz123
```

### Step 3: Alice Creates Her Space

**In Alice's terminal:**
```
demo-alice> space create TechCommunity
demo-alice> channel create general  
demo-alice> thread create Welcome
demo-alice> send Hello everyone!
demo-alice> send This is a decentralized forum.
demo-alice> messages
```

You should see Alice's 2 messages.

### Step 4: Alice Creates an Invite

```
demo-alice> invite create
```

Output:
```
✓ Created invite code: FZKYAxaH

  Share this code with others to invite them:
  $ join 7178a065f470eed88e73adad70a539262fb9452bace16f2bb1b90c229dc5c0ea FZKYAxaH
```

**Copy both the Space ID (long hex) and invite code (short)**

### Step 5: Start Bob

**Terminal 2:**
```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-bob.key
```

Bob starts (not listening, just as a client).

### Step 6: Bob Connects to Alice

**In Bob's terminal:**
```
demo-bob> connect /ip4/127.0.0.1/tcp/9001/p2p/12D3KooW...
```

Use Alice's full multiaddr from Step 2.

Output:
```
ℹ Connecting to peer: /ip4/127.0.0.1/tcp/9001/p2p/12D3KooW...
✓ Connected to peer!
```

### Step 7: Bob Joins Alice's Space

**In Bob's terminal:**
```
demo-bob> join 7178a065f470eed88e73adad70a539262fb9452bace16f2bb1b90c229dc5c0ea FZKYAxaH
```

Use the full Space ID and invite code from Step 4.

**What happens:**
1. Bob subscribes to the Space topic
2. Bob receives operations from Alice via GossipSub
3. Bob's local state rebuilds (Space, Channel, Thread, Messages)
4. Bob uses the invite code to join

If you see errors, wait a few seconds and try again - GossipSub needs time to sync.

### Step 8: Bob Navigates to the Thread

```
demo-bob> spaces
demo-bob> space 7178    (use first few chars)
demo-bob> channels
demo-bob> channel <ID>  (use first few chars)
demo-bob> threads
demo-bob> thread <ID>   (use first few chars)
demo-bob> messages
```

**You should now see Alice's welcome messages!** 🎉

### Step 9: Bob Sends a Reply

```
demo-bob> send Hi Alice! Thanks for creating this space.
demo-bob> send This decentralized system is amazing!
```

### Step 10: Alice Sees Bob's Messages

**Back in Alice's terminal:**
```
demo-alice> refresh
demo-alice> messages
```

**You should now see Bob's messages!** 🎉

## Troubleshooting

### "Space not found. Connect to the Space creator first"

**Problem:** Bob isn't connected to Alice yet.

**Solution:**
1. Run `network` in Alice's terminal to get her multiaddr
2. Run `connect <multiaddr>` in Bob's terminal
3. Wait 2-3 seconds for connection to establish
4. Try `join` again

### "Invalid invite code" / "Invite is no longer valid"

**Problem:** Bob doesn't have the Space operations yet.

**Solution:**
1. Make sure Bob is connected (`connect` command)
2. Wait 5-10 seconds after connecting
3. The invite and Space metadata will sync via GossipSub
4. Try `join` again

### "DHT PUT/GET failed" warnings

**This is NORMAL** - DHT needs 3+ peers for quorum. With only Alice and Bob:
- ✅ GossipSub still works (direct peer-to-peer)
- ✅ Messages sync fine between connected peers
- ⚠️  DHT operations fail (expected behavior)

The system degrades gracefully - everything works via GossipSub!

### Bob sees "closest_peers: []"

**Problem:** DHT has no peers to query.

**Not a problem:** This is expected with 2 users. GossipSub handles all syncing.

### Messages don't appear

**Solutions:**
1. Run `refresh` to trigger sync
2. Verify you're in the same thread (`context`)
3. Check connection (`network`)
4. Wait a few seconds - GossipSub has latency

## Summary

**Correct order:**
1. Alice starts with `--port 9001`
2. Alice runs `network` → get multiaddr
3. Alice creates Space/Channel/Thread
4. Alice runs `invite create` → get Space ID + code
5. Bob starts (no port needed)
6. Bob runs `connect <Alice's multiaddr>`
7. **Wait 2-3 seconds for GossipSub to sync**
8. Bob runs `join <SPACE_ID> <CODE>`
9. Bob navigates to thread and sees messages
10. Both users can chat!

## Why This Works

1. **Alice listens** - She's the bootstrap peer
2. **Bob connects** - Establishes peer-to-peer link
3. **GossipSub syncs** - Operations flow from Alice to Bob
4. **Bob joins** - Uses synced invite to become a member
5. **Bidirectional chat** - Both can send/receive via GossipSub

No central server needed! 🚀

---

**Key Commands:**
- `network` - Show peer info
- `connect <multiaddr>` - Connect to peer
- `join <space_id> <code>` - Join with invite
- `messages` - View messages
- `send <text>` - Send message
- `refresh` - Force sync
- `context` - Check location
