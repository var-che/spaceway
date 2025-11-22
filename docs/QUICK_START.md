# 🚀 Spaceway - Quick Start Guide

## ⚠️ CRITICAL: Read This First!

**DO NOT create spaces/channels/threads when you're the only peer running!**

The app will hang for 30-60 seconds waiting for DHT peers.

**Correct order:**

1. Start all 3 peers first
2. Let them discover each other (10 seconds)
3. THEN create spaces

See `STARTUP_ORDER_IMPORTANT.md` for details.

---

## ✅ Everything is Ready!

Your Spaceway development environment is fully set up with:

- ✅ Nix with RocksDB and all dependencies
- ✅ Rust nightly toolchain
- ✅ Project builds successfully
- ✅ Storage module complete (3000+ lines)
- ✅ All tests passing

## 🎯 Start Testing NOW - 3 Simple Steps

### Step 1: Open Terminal 1 - Start Alice

```bash
./run-spaceway.sh --account ./alice.key --port 9001
```

Wait for it to start, then in Alice's CLI type:

```
space create "TestSpace"
channel create "general"
thread create "Hello"
invite create
```

**Copy the space ID and invite code shown!**

---

###Step 2: Open Terminal 2 - Start Bob

```bash
./run-spaceway.sh --account ./bob.key --port 9002
```

Then in Bob's CLI:

```
connect /ip4/127.0.0.1/tcp/9001
join <space_id> <invite_code>
space <space_id>
send "Hi from Bob!"
```

---

### Step 3: Open Terminal 3 - Start Charlie

```bash
./run-spaceway.sh --account ./charlie.key --port 9003
```

Then in Charlie's CLI:

```
connect /ip4/127.0.0.1/tcp/9001
join <space_id> <invite_code>
space <space_id>
send "Hi from Charlie!"
```

---

## 📋 Essential Commands

Once inside the CLI:

| Command                  | What It Does         |
| ------------------------ | -------------------- |
| `help`                   | Show all commands    |
| `whoami`                 | Your user info       |
| `network`                | See connected peers  |
| `spaces`                 | List your spaces     |
| `space create "Name"`    | Create a new space   |
| `space <id>`             | Switch to a space    |
| `channels`               | List channels        |
| `channel create "Name"`  | Create a channel     |
| `threads`                | List threads         |
| `thread create "Title"`  | Create a thread      |
| `messages`               | Show thread messages |
| `send "text"`            | Send a message       |
| `invite create`          | Generate invite code |
| `join <space_id> <code>` | Join a space         |
| `context`                | Current location     |
| `quit`                   | Exit                 |

---

## 🧪 Alternative: Automated Test

Want to see it work without manual steps?

```bash
nix develop --command cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

This runs an automated 3-peer test (Alice, Bob, Charlie) in ~60 seconds!

---

## 📊 What You'll See

### Peer Discovery

- Each peer discovers others via mDNS
- Check with `network` command
- See peer IDs and connection status

### Encrypted Messaging

- All messages are E2E encrypted (MLS)
- CRDT ensures consistency
- Real-time propagation

### Space Collaboration

- Alice creates space
- Bob and Charlie join via invite
- All see same messages instantly

---

## 🐛 Troubleshooting

### "No known peers" warning

- Normal for first peer (Alice)
- Bob and Charlie will discover Alice
- Use `connect /ip4/127.0.0.1/tcp/9001` to manually connect

### Can't see messages

1. Verify you're in the same space: `context`
2. Check connections: `network`
3. Try `refresh` command

### Build/run errors

- Always use `./run-spaceway.sh` (handles Nix automatically)
- Or manually: `nix develop --command cargo +nightly run...`

---

## 📚 More Documentation

- **`TESTING_SETUP_SUMMARY.md`** - Complete testing guide
- **`MULTI_PEER_TESTING.md`** - Detailed P2P testing
- **`ROCKSDB_FIX.md`** - How the library issue was fixed
- **`README.md`** - Project overview

---

## 🎉 Ready?

**Open your first terminal and run:**

```bash
./run-spaceway.sh --account ./alice.key --port 9001
```

Then follow the steps above!

**Your privacy-preserving P2P forum is ready to test!** 🚀
