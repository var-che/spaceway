# Debug Output Guide

We've added comprehensive debug output to track the DHT query lifecycle. Here's what to look for:

## Key Debug Messages

### When DHT PUT is initiated:

```
🔍 DHT PUT: X peers in routing table
🔍 DHT PUT query started: QueryId(...)
```

- **If peer count = 0**: Peers aren't being added to Kademlia despite ConnectionEstablished
- **If query never starts**: Issue is before put_record() call
- **If query starts but never completes**: Issue is with query processing

### When DHT PUT completes (SUCCESS):

```
✓ DHT PUT: Record stored successfully in XXms, query_id: QueryId(...)
```

### When DHT PUT fails:

```
✗ DHT PUT failed after XXms: [error], query_id: QueryId(...)
❌ DHT PUT failed immediately: [error]
```

### When timeout checker runs (every 10 seconds):

```
⏱️  DHT PUT query timed out after XXs: QueryId(...)
🕐 Timeout check: X GET, Y PUT queries timed out (tracking Z GET, W PUT total)
```

- **If you never see this**: Timer tick not running (event loop issue)
- **If timeout triggers after 10s**: Query stuck, not completing
- **If you see pending queries growing**: Queries starting but never completing

### When queries are untracked:

```
⚠️  DHT PUT completed but query not tracked: QueryId(...)
⚠️  DHT PUT failed but query not tracked: QueryId(...)
```

- This indicates the query completed but wasn't in pending_put_queries map

## Test Scenario

Run Alice and Bob in separate terminals:

**Terminal 1 (Alice):**

```bash
./target/release/spaceway-cli --port 9001 --name alice
# Wait for connection, then:
create space test-space
```

**Terminal 2 (Bob):**

```bash
./target/release/spaceway-cli --port 9002 --name bob --peer /ip4/127.0.0.1/tcp/9001
```

## Expected Flow

1. **Connection**: See "✅ Connection established"
2. **DHT Bootstrap**: See "DHT bootstrap complete"
3. **Space Creation**: See "📢 Broadcasting operation on topic: space/..."
4. **DHT PUT initiated**: Should see "🔍 DHT PUT: X peers in routing table"
5. **Query started**: Should see "🔍 DHT PUT query started: QueryId(...)"
6. **Within 10 seconds**: Should see EITHER:
   - Success: "✓ DHT PUT: Record stored successfully"
   - Failure: "✗ DHT PUT failed"
   - Timeout: "⏱️ DHT PUT query timed out"

## Diagnostic Scenarios

### Scenario 1: Peer count = 0

```
🔍 DHT PUT: 0 peers in routing table
```

**Problem**: ConnectionEstablished not adding peers to Kademlia
**Fix**: Check kademlia.add_address() is being called

### Scenario 2: Query never starts

```
🔍 DHT PUT: 2 peers in routing table
[no "query started" message]
```

**Problem**: put_record() failing immediately or bootstrap blocking
**Fix**: Check for "❌ DHT PUT failed immediately" message

### Scenario 3: Query starts but hangs

```
🔍 DHT PUT query started: QueryId(123)
[wait forever, no completion]
```

**Problem**: Query sent but no response from network
**Solutions**:

- Check if timeout fires after 10s
- Verify Kademlia events are being processed
- May need to check firewall or NAT

### Scenario 4: Timer never runs

```
[no timeout check messages after 10+ seconds]
```

**Problem**: Event loop not processing timer ticks
**Fix**: Check interval.tick() in select! loop

## What We're Testing

The `tokio::timeout(12s)` wrapper on dht_put() should prevent indefinite hanging, but we need to verify:

1. Are queries actually being created?
2. Are they completing (success/failure)?
3. Is the timer-based timeout (10s) firing?
4. Are there any blocking operations before the query is even created?

With this debug output, we'll know exactly where the hang is occurring.
