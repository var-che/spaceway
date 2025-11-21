# Linux Setup Guide for Bob (Computer B)

This guide is for setting up the **Linux Mint computer** to connect to your Windows computer running Alice.

## Step 1: Get the Project on Linux

### Option A: Git Clone (if project is in git repo)
```bash
git clone <your-repo-url>
cd descord
```

### Option B: Copy from Windows Computer

On Windows (Alice's computer):
```powershell
# Create a zip file
Compress-Archive -Path C:\Users\pc\Documents\projects\descord -DestinationPath descord.zip
```

Transfer `descord.zip` to Linux computer via USB, network share, or:
```bash
# On Linux, from Windows share
scp user@windows-pc:/path/to/descord.zip ~/
unzip descord.zip
cd descord
```

---

## Step 2: Install Dependencies (Linux Mint)

```bash
# Update package list
sudo apt update

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install build dependencies
sudo apt install -y build-essential pkg-config libssl-dev cmake

# Verify Rust installation
rustc --version
cargo --version
```

---

## Step 3: Build Descord

```bash
cd descord

# Build in release mode (optimized)
cargo build --release --bin descord

# This will take 5-10 minutes on first build
# Output: ./target/release/descord
```

---

## Step 4: Make Scripts Executable

```bash
# Make the startup script executable
chmod +x start-bob.sh
chmod +x start-alice.sh  # Optional, if you want to test as Alice too
```

---

## Step 5: Get Alice's Connection Info

On your **Windows computer** (Alice), you should already be running:
```powershell
.\start-alice.ps1
```

In Alice's terminal, type:
```
network
```

You'll see something like:
```
Network Status:
  Peer ID: 12D3KooWRBhwfeeFooBar...
  Listening on: 1
    /ip4/0.0.0.0/tcp/9001

📋 Share this multiaddr for others to connect:
  /ip4/0.0.0.0/tcp/9001/p2p/12D3KooWRBhwfeeFooBar...
```

**Important**: You need to:
1. Get Alice's **actual IP address** (not 0.0.0.0)
2. Get Alice's **Peer ID** (the 12D3KooW... string)

To find Alice's IP on Windows:
```powershell
ipconfig | Select-String "IPv4"
```

Example: `192.168.1.100`

---

## Step 6: Start Bob on Linux

On your **Linux Mint computer**:

```bash
# Run Bob
./start-bob.sh
```

You'll see:
```
============================================================
  Starting Bob (Connecting Node)
============================================================

🚀 Starting descord...

After startup:
  1. Get Alice's multiaddr from her terminal (network command)
  ...

bob>
```

---

## Step 7: Connect to Alice

In Bob's terminal, connect using Alice's multiaddr:

```bash
# Replace with Alice's actual IP and Peer ID
connect /ip4/192.168.1.100/tcp/9001/p2p/12D3KooWRBhwfeeFooBar...
```

You should see:
```
✓ Successfully connected to peer
```

Verify the connection:
```
network
```

---

## Step 8: Join Alice's Space

On **Alice's computer** (Windows), create a space and invite:
```
space TestSpace
invite
```

Alice will see:
```
✓ Created Space: TestSpace
  Space ID: 9d2bf8a78ca50c92...

✓ Created invite: ABC12345
  Share this code with others to join
  Valid for: 7 days
```

Copy the **Space ID** and **Invite Code**.

---

On **Bob's computer** (Linux), join the space:
```
join 9d2bf8a78ca50c92... ABC12345
```

You should see:
```
✓ Successfully joined Space: TestSpace
```

---

## Step 9: Send Encrypted Messages

### On Alice (Windows):
```
channel general
thread Hello
send Hi from Alice on Windows!
```

### On Bob (Linux):
```
channel general
thread Hello
messages
```

Bob should see Alice's encrypted message!

### Bob replies:
```
send Hi from Bob on Linux!
```

### Alice checks:
```
messages
```

Alice should see Bob's message!

---

## Troubleshooting

### "Connection refused" or "Failed to connect"

**Check firewall on Windows (Alice's computer):**
```powershell
# Windows PowerShell (as Administrator)
New-NetFirewallRule -DisplayName "Descord" -Direction Inbound -Protocol TCP -LocalPort 9001 -Action Allow
```

**Check both computers are on same network:**
```bash
# On Linux, ping Alice's IP
ping 192.168.1.100
```

**Verify Alice is listening:**
On Alice's terminal, type `network` and confirm it shows listening on port 9001.

---

### "Cannot decrypt message"

This is **normal** if:
- Alice kicked Bob from the space (proves forward secrecy!)
- Otherwise, both should be in the same MLS epoch - try rejoining

---

### Build fails on Linux

**Missing OpenSSL:**
```bash
sudo apt install libssl-dev pkg-config
```

**Missing CMake:**
```bash
sudo apt install cmake
```

**Rust version too old:**
```bash
rustup update stable
```

---

### Check versions

```bash
# Rust
rustc --version  # Should be 1.70+

# Cargo
cargo --version

# Build dependencies
pkg-config --version
cmake --version
```

---

## Quick Command Reference

```bash
# Network
network              # Show your peer ID and connections
connect <multiaddr>  # Connect to Alice
whoami               # Your user info

# Spaces & Channels
spaces               # List spaces
space <name>         # Switch to space
join <id> <code>     # Join space with invite
channel <name>       # Create/switch channel
thread <title>       # Create/switch thread

# Messaging
messages             # View messages in current thread
send <text>          # Send encrypted message
context              # Show where you are
refresh              # Sync from network

# Utilities
help                 # All commands
quit                 # Exit
```

---

## What You're Testing

✅ **Cross-platform P2P**: Windows ↔ Linux communication  
✅ **MLS Encryption**: End-to-end encrypted messages  
✅ **Real-time Sync**: GossipSub message propagation  
✅ **Network Stack**: libp2p, QUIC, circuit relay  
✅ **CRDT Sync**: Conflict-free state replication  

---

## File Locations on Linux

```
descord/
├── target/release/descord  # Binary executable
├── bob.key                 # Bob's identity (auto-created)
├── bob-data/              # Bob's local database
├── bob.history            # Command history
└── start-bob.sh           # Startup script
```

---

## Performance Tips

**First build is slow** (5-10 minutes):
- Subsequent builds are fast (incremental)
- Release mode is optimized for performance

**To rebuild:**
```bash
cargo build --release --bin descord
```

**To clean and rebuild:**
```bash
cargo clean
cargo build --release --bin descord
```

---

## Next: Test with 3 Computers

For full DHT testing (offline Space joining), you need 3+ peers:
- Alice (Windows)
- Bob (Linux)
- Carol (any OS)

With 3 peers:
1. All connect to each other
2. Alice creates Space (stored in DHT)
3. Alice goes offline
4. Carol joins Space from DHT (Alice offline!)

See `TWO_COMPUTER_SETUP.md` Step 6 for details.

---

**You're now running decentralized, encrypted chat across Windows and Linux!** 🚀🐧

Any issues? Check the main `TWO_COMPUTER_SETUP.md` or logs.
