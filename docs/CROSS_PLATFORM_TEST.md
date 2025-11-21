# Cross-Platform Testing: Windows ↔ Linux

Your setup:
- **Computer A** (this one): Windows - runs Alice
- **Computer B** (other one): Linux Mint - runs Bob

## Quick Start

### 1. On This Computer (Windows - Alice)

```powershell
.\start-alice.ps1
```

Then type `network` and note your IP and Peer ID.

---

### 2. On Linux Computer (Bob)

**First time setup:**
```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Build descord
cargo build --release --bin descord

# Make script executable
chmod +x start-bob.sh
```

**Then run:**
```bash
./start-bob.sh
```

**Connect to Alice:**
```
connect /ip4/<ALICE_IP>/tcp/9001/p2p/<ALICE_PEER_ID>
```

---

### 3. Create Space and Chat

**Alice (Windows):**
```
space TestSpace
invite
```

**Bob (Linux):**
```
join <space_id> <invite_code>
channel general
thread hello
send Hi from Linux!
```

**Alice (Windows):**
```
channel general
thread hello
messages
send Hi from Windows!
```

---

## Files You Need

### To Transfer to Linux Computer

Copy these files to your Linux Mint computer:
- The entire `descord/` folder, OR
- Just the source code (Linux will build its own binary)

**Easiest method:**
```bash
# On Linux computer
git clone <your-repo-url>
cd descord
cargo build --release --bin descord
chmod +x start-bob.sh
./start-bob.sh
```

---

## Detailed Guides

- **`QUICK_START.md`** - Quick reference card
- **`LINUX_SETUP.md`** - ⭐ **Start here for Linux setup**
- **`TWO_COMPUTER_SETUP.md`** - Full detailed guide

---

## Troubleshooting

### Can't connect from Linux to Windows?

1. **Check Windows firewall:**
   ```powershell
   New-NetFirewallRule -DisplayName "Descord" -Direction Inbound -Protocol TCP -LocalPort 9001 -Action Allow
   ```

2. **Verify Alice is listening:**
   On Windows, type: `network`

3. **Check network connectivity:**
   On Linux: `ping <ALICE_IP>`

4. **Use correct IP:**
   - Not `0.0.0.0`
   - Not `127.0.0.1`
   - Your actual network IP (e.g., `192.168.1.100`)

---

## What Gets Tested

✅ Cross-platform networking (Windows ↔ Linux)  
✅ libp2p QUIC transport  
✅ MLS end-to-end encryption  
✅ Real-time message sync via GossipSub  
✅ CRDT state replication  
✅ Space invite system  
✅ Forward secrecy (kick test)  

**This proves the entire decentralized stack works across operating systems!**

---

## Ready to Test?

### On Windows (This Computer):
```powershell
.\start-alice.ps1
```

### Transfer to Linux:
Copy the project folder to your Linux Mint computer, then follow `LINUX_SETUP.md`

---

🚀 Let's test cross-platform decentralized encrypted chat!
