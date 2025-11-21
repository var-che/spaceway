# Quick Start: Two-Computer Test

## Computer A (This One) - Alice

Run this:
```powershell
.\start-alice.ps1
```

Then:
1. Type `network` - copy your multiaddr
2. Type `space TestSpace` - create a space  
3. Type `invite` - get invite code
4. Share multiaddr + space_id + invite code with Bob

---

## Computer B (Other Computer) - Bob

### Windows:
```powershell
.\start-bob.ps1
```

### Linux:
```bash
./start-bob.sh
```

Then:
1. Type `connect <alice_multiaddr>` - connect to Alice
2. Type `join <space_id> <invite_code>` - join space
3. Type `channel general` - create/join channel
4. Type `thread test` - create/join thread  
5. Type `send Hello!` - send encrypted message

---

## Key Commands

### Setup
- `network` - Show your connection info
- `connect <multiaddr>` - Connect to peer
- `whoami` - Show your user ID

### Spaces
- `space <name>` - Create/switch space
- `invite` - Create invite code
- `join <space_id> <code>` - Join with invite
- `members` - List members
- `kick <user_id>` - Remove member

### Messaging  
- `channel <name>` - Create/switch channel
- `thread <title>` - Create/switch thread
- `send <text>` - Send message
- `messages` - Show messages

### Info
- `context` - Current location
- `help` - All commands
- `quit` - Exit

---

## What Gets Tested

✅ P2P connection between computers
✅ MLS end-to-end encryption
✅ Real-time message sync
✅ Space invite system
✅ Member management
✅ Forward secrecy (kick test)

---

## Test Sequence

1. **Alice**: `.\start-alice.ps1`
2. **Alice**: `network` (copy multiaddr)
3. **Alice**: `space TestSpace`
4. **Alice**: `invite` (copy code)
5. **Bob**: `.\start-bob.ps1`  
6. **Bob**: `connect <alice_multiaddr>`
7. **Bob**: `join <space_id> <code>`
8. **Both**: `channel general`
9. **Both**: `thread hello`
10. **Alice**: `send Hi from Alice!`
11. **Bob**: `messages` (should see Alice's message)
12. **Bob**: `send Hi from Bob!`
13. **Alice**: `messages` (should see Bob's message)

✅ Success: Both see encrypted messages instantly!

---

## Troubleshooting

**Bob can't connect:**
- Check Alice's IP: `ipconfig` 
- Check firewall allows port 9001
- Replace `0.0.0.0` in multiaddr with actual IP

**Messages not appearing:**
- Type `refresh` to sync
- Check `context` - must be in same space/channel/thread
- Type `messages` to view

**Build needed:**

Windows:
```powershell
cargo build --release --bin descord
```

Linux:
```bash
cargo build --release --bin descord
chmod +x start-bob.sh
```

---

**Setup Guides:**
- `TWO_COMPUTER_SETUP.md` - Detailed Windows/general guide
- `LINUX_SETUP.md` - **Linux Mint specific instructions**
