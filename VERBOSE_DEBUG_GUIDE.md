# Verbose Debugging - Broadcast Operation Flow

## 🔍 What We're Tracking

I've added **extensive debug output** at every step of the broadcast operation to pinpoint exactly where the hang occurs.

## Debug Flow Markers

### 1️⃣ Broadcast Start

```
📢 [BROADCAST START] Broadcasting operation on topic: space/...
📢 [BROADCAST] Operation type: ..., space_id: ...
```

### 2️⃣ GossipSub Step

```
📢 [BROADCAST] Step 1: Calling broadcast_op_on_topic (GossipSub)...
📢 [BROADCAST] Step 1: ✓ GossipSub broadcast completed
```

**If you see Step 1 start but never complete**: GossipSub is hanging (unlikely since it's worked before)

### 3️⃣ DHT Storage Initiation

```
📢 [BROADCAST] Step 2: Calling dht_put_operations (DHT storage)...
🔷 [DHT_PUT_OPS] START: Storing 1 operations for space ...
```

**If you never see DHT_PUT_OPS START**: The call itself is blocking before entering the function

### 4️⃣ Network Lock Acquisition

```
🔷 [DHT_PUT_OPS] Step 1: Acquiring network lock...
🔷 [DHT_PUT_OPS] Step 1: ✓ Network lock acquired
```

**If Step 1 never completes**: Another thread holds the network lock indefinitely

### 5️⃣ DHT GET for Index

```
🔷 [DHT_PUT_OPS] Step 2: Fetching DHT index for key ...
🔷 [dht_get] START: key=...
🔷 [dht_get] Sending DhtGet command to network thread...
🔷 [dht_get] Command sent, awaiting response with 12s timeout...
```

**Expected outcomes within 12 seconds**:

- ✓ Success: `🔷 [dht_get] END: ✓ Success (X values)`
- ✗ Timeout: `🔷 [dht_get] END: ✗ TIMEOUT after 12 seconds`
- ✗ Error: `🔷 [dht_get] END: ✗ Network error: ...`

### 6️⃣ DHT PUT for Batch

```
🔷 [DHT_PUT_OPS] Step 6: Storing batch in DHT (key: ..., size: X bytes)...
🔶 [dht_put] START: key=..., value_size=X bytes
🔶 [dht_put] Sending DhtPut command to network thread...
🔶 [dht_put] Command sent, awaiting response with 12s timeout...
```

**Expected outcomes within 12 seconds**:

- ✓ Success: `🔶 [dht_put] END: ✓ Success`
- ✗ Timeout: `🔶 [dht_put] END: ✗ TIMEOUT after 12 seconds`
- ✗ Error: `🔶 [dht_put] END: ✗ Network error: ...`

### 7️⃣ DHT PUT for Index

```
🔷 [DHT_PUT_OPS] Step 8: Storing updated index in DHT (size: X bytes)...
🔶 [dht_put] START: key=..., value_size=X bytes
```

**Same timeout expectations as Step 6**

### 8️⃣ Completion

```
🔷 [DHT_PUT_OPS] END: ✓ Successfully stored 1 operations in DHT (batch 1)
📢 [BROADCAST] Step 2: ✓ DHT storage completed
📢 [BROADCAST END] Broadcast operation completed
```

## Network Thread Debug Output

You'll also see output from the network worker thread:

### When DHT Command is Received

```
🔍 DHT PUT: X peers in routing table
🔍 DHT PUT query started: QueryId(...)
```

### When Query Completes

```
✓ DHT PUT: Record stored successfully in XXms, query_id: QueryId(...)
```

### If Query Times Out (10s)

```
⏱️  DHT PUT query timed out after 10s: QueryId(...)
```

## Test Now

**Run in two terminals:**

**Terminal 1 (Alice):**

```bash
cd /home/vlada/Documents/projects/spaceway
./target/release/spaceway --port 9001 --name alice
# Wait for connection, then:
create space test-space
```

**Terminal 2 (Bob):**

```bash
cd /home/vlada/Documents/projects/spaceway
./target/release/spaceway --port 9002 --name bob --peer /ip4/127.0.0.1/tcp/9001
```

## What You'll Learn

The debug output will show you **exactly** which step hangs:

1. **Hangs before "Step 1"**: Issue in broadcast_op itself (unlikely)
2. **Hangs at Step 1**: GossipSub issue (very unlikely)
3. **Hangs before DHT_PUT_OPS START**: Call to dht_put_operations blocking (unlikely)
4. **Hangs at "Acquiring network lock"**: Lock contention issue
5. **Hangs at "Sending DhtGet command"**: Command channel dead
6. **Hangs at "awaiting response"**: This is where we expect the hang
7. **Gets TIMEOUT**: The 12s timeout is working, network thread not responding
8. **Hangs forever**: Timeout not working, blocking somewhere unexpected

## Expected Behavior

With the current setup, you should see one of:

- **Best case**: All steps complete successfully (unlikely on first try)
- **Timeout case**: See `TIMEOUT after 12 seconds` at dht_get or dht_put
- **Infinite hang case**: Last message is "awaiting response" and nothing happens

The output will tell us definitively where to look next!
