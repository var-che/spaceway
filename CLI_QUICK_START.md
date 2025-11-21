# Descord CLI Quick Start Guide

## Installation

Build the CLI:
```bash
cargo build --release --bin descord
```

The binary will be at `target/release/descord` (or `target/release/descord.exe` on Windows).

## Basic Usage

### Starting the CLI

```bash
# Create/load account for Alice
./descord --account alice.key

# Create/load account for Bob (in a separate terminal)
./descord --account bob.key
```

The account file will be created automatically if it doesn't exist.
**Each account gets its own data directory** (e.g., `alice-data/`, `bob-data/`), so multiple users can run simultaneously.

## Quick Workflow Example

### Alice creates a Space and invites Bob:

```bash
# Alice starts the CLI
./descord --account alice.key

# Create a Space
alice> space create My Community

# Create a channel
alice> channel create general

# Create a thread
alice> thread create Welcome

# Send a message
alice> send Hello everyone!

# Create an invite code
alice> invite create

# You'll see: "Created invite code: abc12345"
# Share the Space ID and invite code with Bob
```

### Bob joins Alice's Space:

```bash
# Bob starts the CLI
./descord --account bob.key

# Join using invite code (replace with actual IDs)
bob> join a1b2c3d4e5f6g7h8 abc12345

# Or join from DHT if Space is public
bob> join dht a1b2c3d4e5f6g7h8

# Switch to the Space
bob> spaces
bob> space a1b2c3d4

# View channels
bob> channels
bob> channel general

# View threads
bob> threads
bob> thread a1b2c3d4

# Read messages
bob> messages

# Send a message
bob> send Hi Alice!
```

## Complete Command Reference

### General Commands
```
help            - Show all available commands
whoami          - Show your user info
context         - Show current space/channel/thread
refresh         - Refresh network status
quit / exit     - Exit the application
```

### Space Commands
```
spaces                  - List all your spaces
space create <name>     - Create a new space
space <id>              - Switch to a space (use ID prefix)
```

### Channel Commands  
```
channels                - List channels in current space
channel create <name>   - Create a new channel
channel <id>            - Switch to a channel
```

### Thread Commands
```
threads                 - List threads in current channel
thread create <title>   - Create a new thread
thread <id>             - Switch to a thread
```

### Message Commands
```
messages        - Show messages in current thread
send <text>     - Send a message
```

### Invite Commands
```
invite          - List active invites for current space
invite create   - Create a new invite code
join <space_id> <code>  - Join a space with invite code
join dht <space_id>     - Join a space from DHT (works offline)
```

### File Commands
```
upload <file>   - Upload a file to the current space
```

## Features

### ✅ Offline-First Operation
- Spaces, channels, and messages are stored in the DHT
- Join spaces even when the creator is offline
- Automatic sync when peers come online

### 🔒 End-to-End Encryption
- All messages encrypted with MLS
- Forward secrecy and post-compromise security
- Cryptographically enforced access control

### 🌐 Decentralized
- No central server required
- P2P networking via libp2p
- DHT-based discovery and storage

### 🚀 Performance
- Fast local storage with RocksDB
- Efficient CRDT-based sync
- Real-time GossipSub messaging

## Tips

1. **Use ID prefixes**: You don't need to type the full ID. Just type enough characters to uniquely identify (e.g., `space a1b2` instead of the full 64-character hex).

2. **Check context**: Use `context` to see where you currently are (which space/channel/thread).

3. **Message formatting**: Multi-word messages work fine: `send This is a complete sentence!`

4. **DHT joining**: The `join dht` command is perfect for public spaces when the creator might be offline.

5. **File uploads**: Uploaded files are automatically encrypted and replicated to the DHT for offline availability.

## Example Session

```
$ ./descord --account alice.key

============================================================
Descord - Privacy-Preserving Decentralized Forum
============================================================

Account: alice
User ID: a1b2c3d4e5f6g7h8
Relay: /ip4/127.0.0.1/tcp/9000

Type 'help' for available commands, 'quit' to exit

alice> space create Developer Community
✓ Created space: Developer Community (a1b2c3d4)

alice> channel create announcements
✓ Created channel: announcements (e5f6g7h8)

alice> thread create Welcome to Descord
✓ Created thread: Welcome to Descord (i9j0k1l2)

alice> send Welcome to our decentralized community!
✓ Message sent (m3n4)

alice> messages

Messages (1):
  │ a1b2 12:34:56 (m3n4)
  │ Welcome to our decentralized community!

alice> invite create
✓ Created invite code: XyZ789Ab

  Share this code with others to invite them:
  $ join a1b2c3d4 XyZ789Ab

alice> quit
Goodbye!
```

## Troubleshooting

### "No space selected"
Make sure you've selected a space first: `space <id>` or create one with `space create`

### "No channel selected"  
After selecting a space, select a channel: `channel <id>` or create one

### "Failed to create lock file" / "file is being used by another process"
This means two users are trying to use the same data directory. Each account automatically gets its own directory (e.g., `demo-alice-data/` for `demo-alice.key`). Make sure you're using different `--account` filenames for each user.

### "DHT PUT failed"
This is normal in isolated testing without other peers. In production with relay nodes, DHT operations will succeed.

### Commands not recognized
Type `help` to see all available commands and their syntax. **Important**: Commands like `join` must be run INSIDE the Descord CLI, not from your system command prompt.

## Production Deployment

For production use:
1. Deploy relay servers (see `RELAY_ARCHITECTURE.md`)
2. Configure bootstrap peers in the client
3. Set up proper logging (`RUST_LOG=info`)
4. Use systemd or similar for daemon mode

## Next Steps

- Check out `BETA_TESTING.md` for multi-user testing
- See `FEATURE_ROADMAP.md` for upcoming features
- Read `SECURITY_ANALYSIS.md` for privacy details

---

**Happy chatting! 🚀**
