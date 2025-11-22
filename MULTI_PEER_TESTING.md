# Multi-Peer Testing Guide for Spaceway

This guide will help you test P2P communication between multiple instances of your Spaceway application.

## 🎯 What We'll Test

- ✅ Peer discovery and connection
- ✅ Space creation and sharing
- ✅ Encrypted messaging (E2EE with MLS)
- ✅ CRDT synchronization
- ✅ Real-time message propagation

## 🚀 Option 1: Quick Automated Test

Run the existing automated beta test that simulates 3 users:

```bash
# Start the test (it will tell you if relay is needed)
cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

This test simulates:

- 🧑 **Alice**: Creates space, channel, thread
- 👨 **Bob**: Joins and posts messages
- 👦 **Charlie**: Joins and posts messages

**Duration:** ~15-20 seconds (improved DHT timeout)
**Note:** Requires relay server on `127.0.0.1:9000` for full testing

---

## 🚀 Option 2: Manual Multi-Peer Testing (Recommended for Development)

Test with real CLI instances to see P2P in action!

### Step 1: Start First Peer (Alice)

Open **Terminal 1**:

```bash
cargo +nightly run --bin spaceway -- \
  --account ./alice.key \
  --port 9001
```

In Alice's CLI:

```
space create "DevTeam"
# Note the space ID shown, e.g., space/abc123...
channels
channel create "general"
thread create "Welcome"
send "Hello from Alice!"
network    # See Alice's peer ID
```

### Step 2: Start Second Peer (Bob)

Open **Terminal 2**:

```bash
cargo +nightly run --bin spaceway -- \
  --account ./bob.key \
  --port 9002
```

In Bob's CLI:

```
network    # See Bob's peer ID and Alice in peers list
whoami
```

**To connect to Alice directly** (if not auto-discovering):

```
connect /ip4/127.0.0.1/tcp/9001
```

**To join Alice's space** (Alice needs to create an invite):

Back in Alice's terminal:

```
invite create
# Copy the invite code shown
```

Then in Bob's terminal:

```
join <space_id> <invite_code>
spaces
space <space_id>
channels
send "Hi from Bob!"
```

### Step 3: Start Third Peer (Charlie)

Open **Terminal 3**:

```bash
cargo +nightly run --bin spaceway -- \
  --account ./charlie.key \
  --port 9003
```

In Charlie's CLI:

```
connect /ip4/127.0.0.1/tcp/9001
# Join same space as Bob (using invite from Alice)
join <space_id> <invite_code>
space <space_id>
send "Hi from Charlie!"
```

---

## 🔍 What to Observe

### Peer Discovery

- Run `network` in each terminal
- You should see other peers listed
- Check connection status

### Message Propagation

- Send a message from Alice
- Watch it appear in Bob's and Charlie's terminals in real-time
- Try sending from all three simultaneously

### Encrypted Synchronization

- All messages are end-to-end encrypted with MLS
- Each peer maintains local RocksDB storage
- CRDT ensures eventual consistency

### Space & Channel Management

```bash
# In any peer's CLI:
spaces           # List all spaces you're in
channels         # List channels in current space
threads          # List threads in current channel
messages         # Show thread messages
context          # See current space/channel/thread
```

---

## 🧹 Cleanup

After testing:

```bash
# Stop all running instances (Ctrl+C in each terminal)

# Clean up all test data (recommended for fresh start)
cd /home/vlada/Documents/projects/spaceway
rm -rf *-data/ *.key *.history

# Or clean up specific users only
rm -rf alice.key alice.history alice-data/
rm -rf bob.key bob.history bob-data/
rm -rf charlie.key charlie.history charlie-data/
```

**Why clean up?**: Database format may change between builds, causing corruption errors on restart.

---

## 🐛 Troubleshooting

### "No known peers" Warning

- This is normal when starting the first peer
- **NEW**: System now auto-triggers DHT bootstrap every 15 seconds if no peers
- Other peers should discover each other via mDNS or manual `connect`
- DHT operations will timeout in 10 seconds (improved from 30 seconds)

### Can't See Other Peers

```bash
# Manually connect using the peer's multiaddr:
connect /ip4/127.0.0.1/tcp/9001

# Or if on different machines:
connect /ip4/<alice-ip>/tcp/9001
```

### Messages Not Syncing

1. Check all peers are in the same space: `context`
2. Verify network status: `network`
3. Check for errors in terminal output
4. Try `refresh` command to trigger sync

### Build Errors

Always use nightly toolchain:

```bash
cargo +nightly build
cargo +nightly run
cargo +nightly test
```

### RocksDB Corruption or Version Error

If you see errors like:

```
Error: Storage operation failed: Failed to open database: Corruption:
Corrupt or unsupported format_version: 6 in alice-data/000017.sst
```

This happens when database files are incompatible with the current build. **Solution**:

```bash
# Clean up all database files and start fresh
rm -rf *-data/ *.key *.history

# Then restart your peer
cargo +nightly run --bin spaceway -- --account alice.key --port 9001
```

**Note**: This will delete all local data. In production, you'd want to migrate data properly.

---

## 📊 Advanced Testing

### Test Offline/Online Sync

1. Start Alice and Bob
2. Kill Bob's process (Ctrl+C)
3. Have Alice send messages
4. Restart Bob
5. Bob should sync all missed messages via CRDT

### Test Relay Failover

Your app has built-in relay rotation:

- The app will automatically try different relays
- Check logs for relay selection messages

### Test DHT Join

Instead of using invite codes, try DHT-based join:

```bash
join dht <space_id>
```

---

## 🎯 Next Steps

- Test on different machines (same network)
- Test across different networks (requires relay server)
- Load test with many messages
- Test file uploads
- Test member management (kick, permissions)

**Happy Testing!** 🚀
