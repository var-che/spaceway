# 🚀 IMPORTANT: Proper Multi-Peer Startup Order

## ⚠️ The Issue You Hit

When running Alice alone and creating a space, it **appears to hang** at:

```
📢 Broadcasting operation on topic: space/...
```

**Why?** The app is trying to:

1. Broadcast to the gossipsub network (no peers = instant)
2. Store in DHT (waits for response from peers that don't exist)

This will eventually timeout, but it takes 30-60 seconds.

## ✅ The Solution: Start All Peers FIRST

### Correct Order (Prevents Hanging)

#### Step 1: Start All Three Peers Quickly

**Terminal 1 - Alice:**

```bash
./run-spaceway.sh --account ./alice.key --port 9001
```

**Terminal 2 - Bob (within 10 seconds):**

```bash
./run-spaceway.sh --account ./bob.key --port 9002
```

**Terminal 3 - Charlie (within 10 seconds):**

```bash
./run-spaceway.sh --account ./charlie.key --port 9003
```

Wait for all three to show: ✓ Generated 10 KeyPackages

#### Step 2: Let Peers Discover Each Other (~10 seconds)

In any terminal, check peer discovery:

```
network
```

You should see 2 other peers listed. If not:

**In Bob's terminal:**

```
connect /ip4/127.0.0.1/tcp/9001
```

**In Charlie's terminal:**

```
connect /ip4/127.0.0.1/tcp/9001
```

Wait a few seconds, then check again:

```
network
```

#### Step 3: Now Create the Space (Won't Hang!)

**In Alice's terminal:**

```
space create "DevTeam"
```

This should complete **instantly** because:

- ✅ Peers are connected
- ✅ DHT has active nodes
- ✅ GossipSub mesh is established

You'll see:

```
✓ Created space: DevTeam
Space ID: space/f72289bc941d2a5d
```

**Copy this Space ID!**

#### Step 4: Continue with Channels/Threads

**Still in Alice's terminal:**

```
channel create "general"
thread create "Welcome"
invite create
```

**Copy the invite code shown**

#### Step 5: Bob and Charlie Join

**In Bob's terminal:**

```
join space/f72289bc941d2a5d <invite_code>
space space/f72289bc941d2a5d
send "Hi from Bob!"
```

**In Charlie's terminal:**

```
join space/f72289bc941d2a5d <invite_code>
space space/f72289bc941d2a5d
send "Hi from Charlie!"
```

---

## 🎯 Key Takeaway

**Never create spaces/channels/threads when you're the only peer online.**

The P2P protocols (GossipSub + Kademlia DHT) expect a network of peers. Running solo causes timeouts.

**Always:**

1. ✅ Start all test peers first
2. ✅ Verify they're connected (`network`)
3. ✅ Then create spaces/channels
4. ✅ Everything will be instant!

---

## 🐛 If You're Already Stuck

**Current state:** Alice hung at "Broadcasting operation..."

**Solution 1 - Wait it out:**

- Just wait 30-60 seconds
- The DHT operation will timeout
- Command will eventually complete
- But it's annoying!

**Solution 2 - Restart properly:**

```bash
# Kill all instances
pkill -f spaceway

# Clean up (optional)
rm -rf alice-data bob-data charlie-data

# Follow the correct order above
```

---

## 🚀 Alternative: Use the Automated Test

If you just want to see it work without manual coordination:

```bash
nix develop --command cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

This automated test handles all the timing correctly and completes in ~60 seconds.

---

## 📊 What's Happening Under the Hood

When you create a space:

```rust
async fn broadcast_op(&self, op: &CrdtOp) -> Result<()> {
    // 1. Broadcast via GossipSub (instant if peers exist)
    self.broadcast_op_on_topic(op, &topic).await?;

    // 2. Store in DHT (WAITS for peer responses!)
    self.dht_put_operations(&op.space_id, vec![op.clone()]).await?;
    //   ^^^^ This is what hangs without peers

    Ok(())
}
```

With peers = instant  
Without peers = 30-60s timeout

---

**Always start all peers before creating content!** 🚀
