# 🌐 Network Architecture Explained - Simple Guide

## ❓ Your Questions Answered

### **Q: Do we need DHT for 2 people vs 3 people?**

**A: No, the number of peers doesn't determine if you use DHT or not.**

Your app uses **THREE network protocols simultaneously**:

```
┌─────────────────────────────────────────────────────────┐
│              YOUR SPACEWAY APPLICATION                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1️⃣  Direct P2P Connection (always)                     │
│      • Alice ←→ Bob (if on same network/no NAT)        │
│      • Uses mDNS for local peer discovery              │
│      • Works great for 2, 3, or 100 peers              │
│                                                         │
│  2️⃣  GossipSub (always - for messaging)                 │
│      • Pub/sub messaging system                        │
│      • Works with ANY number of peers (2, 3, 100+)     │
│      • Broadcasts messages to all subscribed peers     │
│                                                         │
│  3️⃣  Kademlia DHT (always - for data storage)           │
│      • Distributed hash table                          │
│      • Stores: space metadata, invites, operations     │
│      • Works with ANY number of peers                  │
│      • More efficient with more peers (3+ ideal)       │
│                                                         │
│  4️⃣  Circuit Relay (optional - for NAT traversal)       │
│      • Only needed when peers can't connect directly   │
│      • Used when behind firewalls/NATs                 │
│      • NOT required for localhost testing!             │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

## 🎯 **How It Actually Works**

### **Scenario 1: Testing Locally (Same Machine)**

```
Alice:9001 ←→ Bob:9002 ←→ Charlie:9003
     └──────────────┬──────────────┘
              ALL DIRECT
           (no relay needed!)
```

**What happens:**

- ✅ **Direct P2P**: All 3 connect directly via localhost
- ✅ **GossipSub**: Shares messages between all peers
- ✅ **DHT**: Stores space/invite data (works better with 3+ peers)
- ❌ **Relay**: NOT NEEDED (they can all reach each other)

**This is what you're testing right now!**

---

### **Scenario 2: Same Network (e.g., Home WiFi)**

```
Alice (192.168.1.100) ←→ Bob (192.168.1.101) ←→ Charlie (192.168.1.102)
          └────────────────┬────────────────┘
                     ALL DIRECT
                  (no relay needed!)
```

**What happens:**

- ✅ **mDNS discovery**: Auto-discover peers on local network
- ✅ **Direct P2P**: Connect directly (same subnet)
- ✅ **GossipSub + DHT**: Work perfectly
- ❌ **Relay**: NOT NEEDED

---

### **Scenario 3: Different Networks (Internet)**

```
Alice (Home NAT)     Bob (Coffee Shop NAT)     Charlie (Office NAT)
       ↓                       ↓                        ↓
       └───────────────→ RELAY SERVER ←─────────────────┘
                        (Required!)
```

**What happens:**

- ❌ **Direct P2P**: FAILS (NAT blocks incoming connections)
- ✅ **Relay**: Routes traffic between peers
- ✅ **GossipSub + DHT**: Work through relay
- 🔐 **Privacy**: Relay sees metadata but not message content (encrypted)

**This requires a relay server!**

---

## 🚀 **For Your Testing - Localhost (No Relay Needed)**

Since you're testing on **the same machine** (localhost), you **DON'T need a relay server**!

### **Method 1: Direct Local Testing (Easiest)**

```bash
# Terminal 1 - Alice
./run-spaceway.sh --account ./alice.key --port 9001

# Terminal 2 - Bob
./run-spaceway.sh --account ./bob.key --port 9002

# Terminal 3 - Charlie
./run-spaceway.sh --account ./charlie.key --port 9003
```

Then:

1. **Wait 10 seconds** for all to start
2. In Bob: `connect /ip4/127.0.0.1/tcp/9001`
3. In Charlie: `connect /ip4/127.0.0.1/tcp/9001`
4. Check connections: `network`
5. **Now** Alice can create space (won't hang!)

---

## 📋 **The DHT "Hang" Issue Explained**

### **Why Alice hangs when creating space alone:**

```rust
async fn broadcast_op(&self, op: &CrdtOp) -> Result<()> {
    // 1. GossipSub broadcast (instant)
    self.broadcast_op_on_topic(op, &topic).await?;

    // 2. DHT storage (WAITS for peers!)  ← THIS IS THE PROBLEM
    self.dht_put_operations(&op.space_id, vec![op.clone()]).await?;
    //    ↑ Waits 30-60s for DHT response when no peers exist
}
```

**With 1 peer (Alice alone):**

- GossipSub: ✅ Instant (no one to send to, returns immediately)
- DHT Put: ❌ **Hangs 30-60s** (waiting for peers that don't exist)

**With 3 peers (Alice + Bob + Charlie connected):**

- GossipSub: ✅ Instant (broadcasts to Bob & Charlie)
- DHT Put: ✅ **Instant** (Bob & Charlie store the data)

---

## ✅ **Simple Solution: Start All Peers First**

```bash
# 1. Start all 3 peers quickly
# 2. Wait 10 seconds
# 3. Connect them manually
# 4. THEN create spaces

# This ensures DHT has peers to talk to!
```

---

## 🎯 **Do You Need a Relay Server?**

### **For Local Testing (localhost): NO**

- All peers can connect directly via 127.0.0.1
- GossipSub + DHT work fine
- No relay needed!

### **For Same Network Testing (WiFi): NO**

- Peers discover each other via mDNS
- Direct connections work
- No relay needed!

### **For Internet Testing (different networks): YES**

- NAT prevents direct connections
- Relay server required
- See next section for setup

---

## 🛠️ **If You Want to Test With Relay Server**

### **Option 1: Use Existing Beta Test (Has Relay)**

```bash
# This test expects relay at localhost:9000
# You'd need to start it first:
cargo +nightly run --package descord-relay --release &
sleep 2
cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

### **Option 2: Use libp2p-relay-server (Simpler)**

```bash
# Install official libp2p relay
cargo install libp2p-relay-server

# Run it
libp2p-relay-server --port 9000
```

But **you don't need this for localhost testing!**

---

## 📊 **Summary**

| Scenario                  | Direct P2P | GossipSub     | DHT        | Relay Needed? |
| ------------------------- | ---------- | ------------- | ---------- | ------------- |
| **1 peer alone**          | N/A        | ✅ (no peers) | ❌ (hangs) | No            |
| **2-3 peers (localhost)** | ✅         | ✅            | ✅         | **NO** ✅     |
| **2-3 peers (same WiFi)** | ✅         | ✅            | ✅         | **NO** ✅     |
| **2-3 peers (internet)**  | ❌         | ✅            | ✅         | **YES** ❌    |

---

## 🚀 **Recommendation for You**

**For now, test WITHOUT relay:**

1. Use `./start-3-peers-guide.sh` to see the exact commands
2. Start all 3 peers on different ports
3. Connect them manually
4. Test messaging

**Later, if you want to test across networks:**

- Set up a relay server
- Or use the automated beta test

**Your current setup is perfect for local testing - no relay needed!** ✅
