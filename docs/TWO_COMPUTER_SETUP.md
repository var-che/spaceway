# Two-Computer Setup Guide

This guide shows you how to connect two computers and test the full decentralized chat system with MLS encryption and DHT storage.

## Overview

You'll set up:
- **Computer A** (this computer): Alice - Creates Space, listens for connections
- **Computer B** (your other computer): Bob - Connects to Alice, joins Space

Both computers will communicate peer-to-peer with end-to-end encryption.

---

## Prerequisites

1. Both computers on same network (or have direct network connectivity)
2. Firewall allows TCP connections on chosen port
3. Descord built on both machines: `cargo build --release --bin descord`

---

## Step 1: Setup Computer A (Alice)

### 1.1 Find your local IP address

On Computer A, run:
```powershell
# Windows
ipconfig | Select-String "IPv4"

# Linux/Mac
ip addr show | grep "inet "
```

Note your local IP (e.g., `192.168.1.100`)

### 1.2 Start Alice with listening port

```powershell
cd C:\Users\pc\Documents\projects\descord

# Start Alice listening on port 9001
cargo run --release --bin descord -- --account alice.key --port 9001
```

### 1.3 Get Alice's multiaddr

Once Alice starts, type:
```
network
```

You'll see output like:
```
Network Status:
  Peer ID: 12D3KooWRBhwfeeFooBar...
  Listening on: 1
    /ip4/0.0.0.0/tcp/9001

📋 Share this multiaddr for others to connect:
  /ip4/0.0.0.0/tcp/9001/p2p/12D3KooWRBhwfeeFooBar...
```

**IMPORTANT**: Replace `0.0.0.0` with your actual IP address from Step 1.1

Your connection string for Bob will be:
```
/ip4/192.168.1.100/tcp/9001/p2p/12D3KooWRBhwfeeFooBar...
```

---

## Step 2: Setup Computer B (Bob)

### 2.1 Copy the project to Computer B

If not already there:
```powershell
# Clone or copy the project
git clone <repo-url>
cd descord

# Build
cargo build --release --bin descord
```

### 2.2 Start Bob and connect to Alice

On Computer B:
```powershell
# Start Bob (no port = client-only mode)
cargo run --release --bin descord -- --account bob.key

# Once started, connect to Alice using her multiaddr from Step 1.3
connect /ip4/192.168.1.100/tcp/9001/p2p/12D3KooWRBhwfeeFooBar...
```

You should see:
```
✓ Successfully connected to peer
```

---

## Step 3: Create and Join a Space

### 3.1 Alice creates a Space

On Computer A (Alice):
```
# Create a new Space
space MyTestSpace

# Create an invite code
invite
```

Alice will see:
```
✓ Created invite: ABC12345
  Share this code with others to join
  Valid for: 7 days
```

### 3.2 Bob joins the Space

On Computer B (Bob), use the space_id and invite code from Alice:
```
# Join using space_id and invite code
join <space_id> ABC12345
```

Bob should see:
```
✓ Successfully joined Space: MyTestSpace
```

---

## Step 4: Send Encrypted Messages

### 4.1 Create a Channel and Thread

On Alice:
```
# Create a channel
channel general

# Create a thread
thread Hello World

# Send a message
send Hello from Alice! This is encrypted with MLS!
```

### 4.2 Bob reads and replies

On Bob:
```
# Switch to the space
space MyTestSpace

# Switch to the channel
channel general

# Switch to the thread
thread Hello World

# Read messages
messages

# Send a reply
send Hi Alice! I can read your encrypted message!
```

---

## Step 5: Verify Encryption

### 5.1 Check MLS encryption is working

Messages should be encrypted automatically. To verify:

1. Both users should see messages appear instantly
2. Messages are end-to-end encrypted via MLS
3. Network traffic shows encrypted payloads (not plaintext)

### 5.2 Test member removal

On Alice:
```
# Check members
members

# Remove Bob (to test encryption)
kick <bob_user_id>
```

After removal:
- Alice can still send messages
- Bob **cannot decrypt** new messages (lacks new epoch keys)
- This proves forward secrecy is working!

---

## Step 6: Test DHT Offline Joining (3+ Computers)

For full DHT testing, you need 3+ peers for Kademlia quorum.

### 6.1 Setup Computer C (Carol)

On a third computer:
```powershell
# Start Carol listening
cargo run --release --bin descord -- --account carol.key --port 9002

# Connect to Alice
connect /ip4/192.168.1.100/tcp/9001/p2p/<alice_peer_id>

# Connect to Bob  
connect /ip4/192.168.1.101/tcp/9003/p2p/<bob_peer_id>
```

### 6.2 Test offline Space joining

1. Alice creates Space and stores metadata in DHT
2. Alice goes offline (close her client)
3. Carol joins Space using DHT: `join <space_id> <invite_code>`
4. Carol retrieves Space data from Bob/DHT (Alice offline!)

This proves DHT persistent storage works.

---

## Common Commands Reference

### Network Commands
```
network              # Show your peer ID and listening addresses
connect <multiaddr>  # Connect to another peer
```

### Space Management
```
spaces               # List all spaces
space <name>         # Create or switch to space
join <space_id> <code> # Join space with invite code
invite               # Create invite code for current space
members              # List space members
kick <user_id>       # Remove member (admin only)
```

### Messaging
```
channels             # List channels
channel <name>       # Create or switch to channel
threads              # List threads
thread <title>       # Create or switch to thread
messages             # Show messages in current thread
send <text>          # Send message
```

### Utilities
```
whoami               # Show your user info
context              # Show current space/channel/thread
refresh              # Sync from network
help                 # Show all commands
quit                 # Exit
```

---

## Troubleshooting

### "Connection refused"

- Check firewall allows TCP on the port
- Verify IP address is correct (`ipconfig` or `ip addr`)
- Ensure Alice is listening (`network` shows port)

### "Cannot decrypt message"

- Normal after being kicked (proves encryption works!)
- Otherwise: ensure both users in same MLS epoch

### "DHT timeout"

- DHT needs 3+ peers for quorum
- For 2-peer testing, DHT may timeout (expected)
- Core messaging works without DHT

### Messages not appearing

- Try `refresh` command to sync
- Check `network` shows connection to peer
- Verify you're in the same space/channel/thread (`context`)

---

## Network Architecture

```
Computer A (Alice)           Computer B (Bob)
┌─────────────────┐         ┌─────────────────┐
│  descord-cli    │         │  descord-cli    │
│  Port: 9001     │◄───────►│  Port: random   │
│  Listening      │   P2P   │  Connect-only   │
└─────────────────┘         └─────────────────┘
        │                            │
        │     Encrypted MLS          │
        │     Application Data       │
        └────────────────────────────┘
                   │
            ┌──────┴──────┐
            │  GossipSub  │  Real-time propagation
            │  DHT Store  │  Offline availability
            └─────────────┘
```

### Security Features Active

1. **MLS Encryption**: All messages encrypted with group keys
2. **Forward Secrecy**: Kicked members can't decrypt new messages
3. **Peer-to-Peer**: Direct connection, no central server
4. **DHT Storage**: Space metadata replicated for offline access
5. **Ed25519 Signatures**: All operations cryptographically signed

---

## What You're Testing

When you run this two-computer setup, you're verifying:

✅ **Networking**: libp2p peer discovery and connection  
✅ **MLS Encryption**: End-to-end encrypted group messaging  
✅ **CRDT Sync**: Operation replication and conflict resolution  
✅ **GossipSub**: Real-time message propagation  
✅ **Access Control**: Role-based permissions and member removal  
✅ **Forward Secrecy**: Removed members can't decrypt future messages  

With 3+ computers:
✅ **DHT Storage**: Offline Space joining via Kademlia DHT  
✅ **Network Resilience**: Mesh network redundancy  

---

## Next Steps

After successful two-computer testing:

1. **Deploy relay nodes** - Public relays for NAT traversal
2. **Add third computer** - Test full DHT quorum
3. **Stress test** - Multiple spaces, channels, large messages
4. **Mobile clients** - iOS/Android apps
5. **Production deployment** - Public network launch

---

## Success Criteria

Your test is successful when:

- [x] Both computers connect via libp2p
- [x] Alice creates Space, Bob joins with invite
- [x] Messages appear instantly on both sides
- [x] Network shows encrypted payloads (not plaintext)
- [x] Bob kicked → can't decrypt Alice's new messages
- [ ] With 3rd computer: Carol joins Alice's Space while Alice offline (DHT)

---

**You're now running a fully decentralized, end-to-end encrypted chat system!** 🚀

For questions or issues, check the logs or file an issue on GitHub.
