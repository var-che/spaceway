# Spaceway Multi-Peer Testing Setup - Summary

## ✅ What's Been Set Up

Your Spaceway development environment is fully configured for multi-peer P2P testing!

### Environment

- ✅ **Nix 2.32.4** - Package manager with flakes support
- ✅ **Rust Nightly 1.93.0** - Required for edition2024 features
- ✅ **Project builds successfully** - All storage modules implemented
- ✅ **Tests passing** - 8/8 storage unit tests pass

### Testing Tools Created

1. **`MULTI_PEER_TESTING.md`** - Complete testing guide
2. **`scripts/start-peer.sh`** - Easy peer launcher
3. **`scripts/multi-peer-guide.sh`** - Quick reference display

## 🚀 Quick Start Options

### Option A: Automated Beta Test (Fastest)

Simulates 3 users automatically:

```bash
cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

**Duration:** ~60 seconds  
**What it does:** Alice, Bob, and Charlie create spaces, join, and exchange encrypted messages

### Option B: Manual 3-Peer Testing (Best for Development)

Test real P2P communication between CLI instances:

#### Terminal 1 - Start Alice

```bash
./scripts/start-peer.sh --name alice --port 9001
```

Then in Alice's CLI:

```
space create "DevSpace"
channel create "general"
invite create              # Copy the invite code
network                    # See peer info
```

#### Terminal 2 - Start Bob

```bash
./scripts/start-peer.sh --name bob --port 9002
```

Then in Bob's CLI:

```
connect /ip4/127.0.0.1/tcp/9001
join <space_id> <invite_code>
space <space_id>
send "Hello from Bob!"
```

#### Terminal 3 - Start Charlie

```bash
./scripts/start-peer.sh --name charlie --port 9003
```

Then in Charlie's CLI:

```
connect /ip4/127.0.0.1/tcp/9001
join <space_id> <invite_code>
space <space_id>
send "Hello from Charlie!"
```

## 📋 Essential CLI Commands

| Command                  | Description                                        |
| ------------------------ | -------------------------------------------------- |
| `help`                   | Show all available commands                        |
| `whoami`                 | Display user ID and account name                   |
| `network`                | Show peer ID, listening addresses, connected peers |
| `context`                | Display current space/channel/thread               |
| `spaces`                 | List all spaces you've joined                      |
| `space create <name>`    | Create a new space                                 |
| `space <id>`             | Switch to a space                                  |
| `channels`               | List channels in current space                     |
| `channel create <name>`  | Create a new channel                               |
| `threads`                | List threads in current channel                    |
| `thread create <title>`  | Create a new thread                                |
| `messages`               | Show messages in current thread                    |
| `send <text>`            | Send a message                                     |
| `invite create`          | Generate an invite code for current space          |
| `join <space_id> <code>` | Join a space with invite code                      |
| `connect <multiaddr>`    | Connect to a peer manually                         |
| `refresh`                | Refresh network status                             |
| `quit` / `exit`          | Exit the application                               |

## 🔍 What to Test

### 1. Peer Discovery

- Start Alice → Check `network` command
- Start Bob → Check `network` to see Alice
- Verify peers auto-discover via mDNS

### 2. Space Sharing

- Alice creates space and invite
- Bob joins using invite code
- Charlie joins same space
- All see same space in `spaces` list

### 3. Message Propagation

- Alice sends message in thread
- Bob sees it appear instantly
- Charlie also sees it
- All messages are E2E encrypted (MLS)

### 4. CRDT Synchronization

- Test offline/online sync:
  - Kill Bob's process
  - Alice sends messages
  - Restart Bob
  - Bob should sync all missed messages

### 5. Concurrent Operations

- All three send messages simultaneously
- CRDT ensures consistent ordering
- No conflicts or duplicates

## 🏗️ Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                  Spaceway/Descord                   │
│                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐         │
│  │  Alice   │  │   Bob    │  │ Charlie  │         │
│  │  :9001   │  │  :9002   │  │  :9003   │         │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘         │
│        │             │              │               │
│        └─────────────┴──────────────┘               │
│               libp2p Network                        │
│                                                     │
│  Features:                                          │
│  • P2P gossipsub messaging                         │
│  • MLS end-to-end encryption                       │
│  • CRDT state synchronization                      │
│  • RocksDB persistent storage                      │
│  • mDNS peer discovery                             │
│  • Relay server failover                           │
└─────────────────────────────────────────────────────┘
```

## 🛠️ Using Nix (Optional)

Your project has a Nix development environment configured:

```bash
# Enter Nix development shell
nix develop

# Now all dependencies are available
cargo build
cargo test
```

The Nix shell provides:

- Rust toolchain (pinned version)
- RocksDB system library
- OpenSSL
- All build dependencies
- Helpful cargo tools

## 🧹 Cleanup After Testing

```bash
# Remove test accounts
rm -f alice.key alice.history bob.key bob.history charlie.key charlie.history

# Remove test data directories
rm -rf alice-data/ bob-data/ charlie-data/ test-account-data/

# Or keep them for next test session
```

## 📚 Documentation

- **`MULTI_PEER_TESTING.md`** - Detailed testing guide
- **`README.md`** - Project overview and quick start
- **`docs/BETA_QUICK_START.md`** - Beta testing guide
- **`backend/RELAY_ARCHITECTURE.md`** - Relay server design

## 🐛 Common Issues

### "No known peers" warning

- Normal for first peer
- Other peers will discover via mDNS
- Use `connect` command to manually connect

### Build errors

- Always use `cargo +nightly` (not regular cargo)
- Project requires Rust nightly for edition2024

### Messages not syncing

- Verify all peers in same space: `context`
- Check network status: `network`
- Try manual connect: `connect /ip4/127.0.0.1/tcp/9001`

## 🎯 Next Steps

1. **Start with automated test** to verify everything works
2. **Try manual 3-peer test** to see P2P in action
3. **Test across different machines** on same network
4. **Explore advanced features** (file uploads, member management)
5. **Deploy relay server** for cross-network testing

---

**Your Spaceway application is ready for multi-peer testing!** 🚀

Choose your testing method and start exploring P2P functionality.
