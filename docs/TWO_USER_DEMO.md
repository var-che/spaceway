# Two-User Demo - Alice and Bob

This guide shows how to run a working demo with Alice and Bob actually connecting and exchanging messages.

## Step 1: Start Alice (Listening on Port 9001)

**Terminal 1:**
```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-alice.key --port 9001
```

Alice will start and listen on port 9001 for incoming connections.

## Step 2: Get Alice's Peer Address

**In Alice's terminal:**
```
demo-alice> network
```

You'll see output like:
```
Network Status:
  Peer ID: 12D3KooWRUJT4o5j...
  Listening on: 1
    /ip4/0.0.0.0/tcp/9001

📋 Share this multiaddr for others to connect:
  /ip4/0.0.0.0/tcp/9001/p2p/12D3KooWRUJT4o5j...
```

**Copy the full multiaddr** (the line starting with `/ip4/0.0.0.0/tcp/9001/p2p/...`)

**Important**: Change `/ip4/0.0.0.0/` to `/ip4/127.0.0.1/` for local connections.

Example: If you see:
```
/ip4/0.0.0.0/tcp/9001/p2p/12D3KooWRUJT4o5jAbCdEfGhIjKlMnOpQrStUvWxYz123
```

Change it to:
```
/ip4/127.0.0.1/tcp/9001/p2p/12D3KooWRUJT4o5jAbCdEfGhIjKlMnOpQrStUvWxYz123
```

## Step 3: Alice Creates a Space

**In Alice's terminal:**
```
demo-alice> space create Tech Community
demo-alice> channel create general
demo-alice> thread create Introductions
demo-alice> send Welcome to the Tech Community!
demo-alice> send This is a decentralized forum running on p2p.
demo-alice> invite create
```

**Copy the full Space ID and invite code** from the output. Example:
```
$ join 587a73833b9e077a668d94d87442d14729e9806ab83bee3f59bbad23dbe91e25 7SjSezGR
```

## Step 4: Start Bob and Connect to Alice

**Terminal 2:**
```powershell
cd C:\Users\pc\Documents\projects\descord
.\target\release\descord.exe --account demo-bob.key --bootstrap "/ip4/127.0.0.1/tcp/9001/p2p/12D3KooW..."
```

Replace the multiaddr with Alice's full peer address from Step 2.

**Alternatively**, start Bob without bootstrap and connect manually:

```powershell
.\target\release\descord.exe --account demo-bob.key
```

Then in Bob's terminal:
```
demo-bob> connect /ip4/127.0.0.1/tcp/9001/p2p/12D3KooW...
```

## Step 5: Bob Joins Alice's Space

**In Bob's terminal:**
```
demo-bob> join 587a73833b9e077a668d94d87442d14729e9806ab83bee3f59bbad23dbe91e25 7SjSezGR
```

Use the Space ID and invite code from Alice's `invite create` output.

If you see "No space selected", continue:

```
demo-bob> spaces
demo-bob> space 587a    (use first few chars of Space ID)
demo-bob> channels
demo-bob> channel <ID>  (use first few chars)
demo-bob> threads
demo-bob> thread <ID>   (use first few chars)
demo-bob> messages
```

You should now see Alice's welcome messages! 🎉

## Step 6: Bob Sends a Reply

**In Bob's terminal:**
```
demo-bob> send Hi Alice! Thanks for the invite.
demo-bob> send This decentralized system is really cool!
```

## Step 7: Alice Sees Bob's Messages

**In Alice's terminal:**
```
demo-alice> refresh
demo-alice> messages
```

You should now see Bob's messages! 🎉

## Step 8: Continue the Conversation

Both users can now freely exchange messages:

**Alice:**
```
demo-alice> send Great to have you here Bob!
```

**Bob:**
```
demo-bob> messages
demo-bob> send Looking forward to exploring this more.
```

## What You've Demonstrated

✅ **Direct P2P Connection** - Bob connected directly to Alice  
✅ **Decentralized Messaging** - No central server involved  
✅ **Space Creation & Joining** - Hierarchical forum structure  
✅ **Invite System** - Secure invite codes for access control  
✅ **Real-time Sync** - Messages exchanged between peers  
✅ **Multi-user Operation** - Two CLI instances running simultaneously  

## Troubleshooting

### "DHT PUT/GET failed"
This is **normal** when only 2 peers are connected. DHT needs more peers for replication. The system still works through direct peer-to-peer gossip.

### "No space selected"
Run `spaces`, then `space <ID>` to switch to the space Bob joined.

### "Failed to create lock file"
Make sure you're using different account names (`demo-alice.key` vs `demo-bob.key`).

### Bob can't connect
- Verify you changed `/ip4/0.0.0.0/` to `/ip4/127.0.0.1/` in the multiaddr
- Check Alice is still running with `--port 9001`
- Make sure you copied the full peer ID correctly

### Messages don't sync
- Run `refresh` to trigger network sync
- Verify both users are in the same thread (use `context` to check)
- Confirm they're connected (use `network` to check peer status)

## Quick Reference

### Alice's Commands
```powershell
# Start Alice listening on port 9001
.\target\release\descord.exe --account demo-alice.key --port 9001

# Inside Alice's CLI
network                     # Get peer address to share
space create Tech Community
channel create general
thread create Introductions
send Hello everyone!
invite create              # Get Space ID + invite code
messages                   # View messages
refresh                    # Sync new messages from Bob
```

### Bob's Commands
```powershell
# Start Bob and connect to Alice
.\target\release\descord.exe --account demo-bob.key --bootstrap "<Alice's multiaddr>"

# Inside Bob's CLI
join <SPACE_ID> <INVITE_CODE>
spaces
space <ID>
channels
channel <ID>
threads
thread <ID>
messages                   # See Alice's messages
send Hi Alice!
```

---

**Enjoy real decentralized messaging! 🚀**
