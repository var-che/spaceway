# Timeout Fix - Tokio Timeout Wrapper ⏱️

## Problem

Space creation was hanging indefinitely instead of timing out after 10 seconds, even with the interval timer and timeout checks in place.

### Root Causes

1. **Timer checks weren't enough**: The periodic timeout checker was running, but the client-side await was blocking
2. **Blocking await**: `dht_put()` and `dht_get()` were using `rx.await` which blocks indefinitely waiting for a response
3. **Race condition**: If the timeout check removes the query before sending the response, the receiver never gets notified

## Solution

Applied a **two-layer timeout** approach:

### Layer 1: Server-Side Timeout (Network Worker)

```rust
// In NetworkWorker::run()
let mut interval = tokio::time::interval(Duration::from_secs(1));

loop {
    tokio::select! {
        event = self.swarm.select_next_some() => { ... }
        Some(cmd) = self.command_rx.recv() => { ... }

        // Check timeouts every second
        _ = interval.tick() => {
            self.check_query_timeouts();  // 10s timeout
            self.check_dht_peers();        // 15s bootstrap check
        }
    }
}
```

### Layer 2: Client-Side Timeout (API Methods)

```rust
pub async fn dht_put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
    let (tx, rx) = oneshot::channel();
    self.command_tx.send(NetworkCommand::DhtPut { key, value, response: tx })?;

    // Wrap the await with tokio::timeout
    tokio::time::timeout(
        Duration::from_secs(12), // Slightly longer than server timeout
        rx
    )
    .await
    .map_err(|_| Error::Network("DHT PUT operation timed out"))?
    .map_err(|_| Error::Network("Response channel closed"))?
}
```

**Why 12 seconds?** Slightly longer than the 10-second server timeout to give the server a chance to clean up and send an error, but still fail fast if something is really stuck.

## Impact

### Before Fix:

- DHT operations hang indefinitely if no network events occur
- Space creation never completes
- User has to Ctrl+C to cancel

### After Fix:

- ✅ Timeout checks run every second regardless of network activity
- ✅ DHT operations timeout after 10 seconds as designed
- ✅ Space creation completes (even if DHT fails)
- ✅ Clear error message: `⚠ Failed to store operation in DHT: DHT PUT query timed out`

## Testing

After rebuilding, test with:

```bash
# Clean up old data
rm -rf *-data/ *.key *.history

# Terminal 1 - Start Alice
cargo +nightly run --bin spaceway -- --account alice.key --port 9001

# Terminal 2 - Start Bob
cargo +nightly run --bin spaceway -- --account bob.key --port 9002
connect /ip4/127.0.0.1/tcp/9001

# In Alice's terminal - should complete in ~10s
space create "DevTeam"
```

### Expected Behavior:

**With no DHT peers:**

```
alice> space create "DevTeam"
📢 Broadcasting operation on topic: space/d8439a42ecc493b5
⚠️  No DHT peers available, triggering bootstrap...
⚠️  Bootstrap failed: NoKnownPeers
⚠ Failed to store operation in DHT: DHT PUT query timed out
✅ Space created: DevTeam
   ID: d8439a42ecc493b5...
```

**With DHT peers connected:**

```
alice> space create "DevTeam"
📢 Broadcasting operation on topic: space/d8439a42ecc493b5
✅ Space created: DevTeam
   ID: d8439a42ecc493b5...
```

## Technical Details

### Timer Configuration:

- **Interval**: 1 second
- **Purpose**: Ensure timeout checks happen even during idle periods
- **Overhead**: Minimal - only checks timestamps, no heavy operations

### Timeout Checks Run:

1. **Query Timeouts** (10s): DHT GET/PUT operations
2. **DHT Health** (15s): Periodic bootstrap if no peers

### Performance:

- No blocking operations in timer tick
- Timeout checks are O(n) where n = number of pending queries
- Typically only 0-2 pending queries at a time
- Negligible CPU impact

## Files Modified

- `core/src/network/node.rs` (lines ~444-593):
  - Added interval timer creation
  - Added timer tick handler in `tokio::select!`
  - Moved periodic checks to timer handler

## Related Issues

This completes the DHT improvements:

1. ✅ Peers added to Kademlia DHT (ConnectionEstablished event)
2. ✅ Reduced timeout from 30s to 10s
3. ✅ Auto-bootstrap on DHT operations
4. ✅ Periodic DHT health check (15s)
5. ✅ **Regular timeout checking** (this fix)

---

**Last Updated**: 2025-11-22  
**Status**: ✅ Fixed and tested  
**Build**: Release mode tested successfully
