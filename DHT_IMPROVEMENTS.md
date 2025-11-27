# DHT Performance & Peer Discovery Improvements

## 🎯 Problem Summary

The Spaceway P2P network was experiencing 30-60 second hangs when creating spaces or performing DHT operations. This was caused by:

1. **Peers not added to Kademlia DHT**: When peers connected via `connect` command or relay circuits, they were only added to GossipSub (pub/sub layer) but NOT to the Kademlia DHT routing table
2. **Long DHT timeout**: DHT queries waited 30 seconds before timing out when no peers were available
3. **No automatic bootstrap**: The system didn't automatically try to bootstrap the DHT when it detected no peers

## ✅ Improvements Made

### 1. Automatic Peer Addition to Kademlia DHT

**File**: `core/src/network/node.rs` (line ~627)

```rust
SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
    // Add peer to GossipSub
    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);

    // ✨ NEW: Add peer to Kademlia DHT
    self.swarm.behaviour_mut().kademlia.add_address(
        &peer_id,
        endpoint.get_remote_address().clone()
    );

    let _ = self.event_tx.send(NetworkEvent::PeerConnected(peer_id));
}
```

**Impact**: Now when peers connect (via direct connection, mDNS, or relay), they're automatically added to BOTH GossipSub and Kademlia, enabling DHT operations to work immediately.

---

### 2. Reduced DHT Query Timeout

**File**: `core/src/network/node.rs` (line ~618)

**Before**: 30 seconds

```rust
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
```

**After**: 10 seconds

```rust
const QUERY_TIMEOUT: Duration = Duration::from_secs(10);
```

**Impact**: Failed DHT operations (when no peers available) now timeout 3x faster, reducing perceived latency.

---

### 3. Proactive Bootstrap on DHT Operations

**File**: `core/src/network/node.rs` (line ~535)

```rust
NetworkCommand::DhtPut { key, value, response } => {
    // ✨ NEW: Check if we have DHT peers
    let peer_count: usize = self.swarm.behaviour_mut().kademlia
        .kbuckets()
        .map(|bucket| bucket.iter().count())
        .sum();

    if peer_count == 0 {
        eprintln!("⚠️  No DHT peers available, triggering bootstrap...");
        if let Err(e) = self.swarm.behaviour_mut().kademlia.bootstrap() {
            eprintln!("⚠️  Bootstrap failed: {:?}", e);
        }
    }

    // Proceed with DHT PUT...
}
```

**Impact**: Before attempting DHT operations, the system checks if DHT peers exist and tries to bootstrap if needed. This can help discover peers that were missed during initial connection.

---

### 4. Periodic DHT Health Check

**File**: `core/src/network/node.rs` (line ~588)

```rust
fn check_dht_peers(&mut self) {
    const BOOTSTRAP_CHECK_INTERVAL: Duration = Duration::from_secs(15);

    // Only check every 15 seconds
    if now.duration_since(self.last_bootstrap_check) < BOOTSTRAP_CHECK_INTERVAL {
        return;
    }

    self.last_bootstrap_check = now;

    // Count peers in routing table
    let peer_count: usize = self.swarm.behaviour_mut().kademlia
        .kbuckets()
        .map(|bucket| bucket.iter().count())
        .sum();

    if peer_count == 0 {
        eprintln!("⚠️  No DHT peers in routing table, triggering bootstrap...");
        if let Err(e) = self.swarm.behaviour_mut().kademlia.bootstrap() {
            eprintln!("   Bootstrap failed: {:?} (normal if no bootstrap peers)", e);
        }
    }
}
```

**Impact**: Every 15 seconds, the system automatically checks if it has DHT peers and triggers bootstrap if the routing table is empty. This helps maintain DHT connectivity even if peers disconnect.

---

## 📊 Performance Comparison

### Before Improvements:

- **Space creation**: 30-60 second hang → timeout
- **DHT operations**: Failed silently or took 30 seconds to timeout
- **Peer discovery**: Manual `connect` commands didn't enable DHT
- **Relay connections**: Peers couldn't use DHT features

### After Improvements:

- **Space creation**: ~10 seconds maximum (with better error feedback)
- **DHT operations**: Fail fast (10 seconds) with auto-retry via bootstrap
- **Peer discovery**: Automatic DHT enablement on any connection
- **Relay connections**: Full DHT functionality via relay circuits

---

## 🧪 Testing

### Manual Test (3 Peers on localhost)

```bash
# Terminal 1 - Alice
cargo +nightly run --bin spaceway -- --account alice.key --port 9001

# Terminal 2 - Bob
cargo +nightly run --bin spaceway -- --account bob.key --port 9002
connect /ip4/127.0.0.1/tcp/9001

# Terminal 3 - Charlie
cargo +nightly run --bin spaceway -- --account charlie.key --port 9003
connect /ip4/127.0.0.1/tcp/9001

# In Alice's terminal:
space create "Test Space"
# Should complete in <10 seconds instead of hanging
```

### Automated Test

```bash
cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

**Expected behavior**:

- Space creation completes within 10-15 seconds (with warning if no relay)
- DHT operations timeout faster with clear error messages
- Automatic bootstrap attempts visible in logs

---

## 🔍 Monitoring DHT Health

You can now monitor DHT connectivity in the logs:

- `✅ Connection established with peer: <peer-id>` - Peer added to both GossipSub and Kademlia
- `⚠️  No DHT peers available, triggering bootstrap...` - Auto-recovery attempt before DHT operation
- `⚠️  No DHT peers in routing table, triggering bootstrap...` - Periodic health check (every 15s)
- `⚠ Failed to store operation in DHT: ...` - Graceful degradation (GossipSub still works)

---

## 📝 Notes

### Graceful Degradation

The system is designed to work even when DHT fails:

- **GossipSub** handles real-time messaging between connected peers
- **DHT** provides offline sync and peer discovery
- If DHT fails, the app continues working with reduced functionality

### Bootstrap Peers

For best results, configure bootstrap peers when creating the network:

```rust
NetworkNode::new_with_config(
    vec!["<relay-multiaddr>".to_string()],  // Bootstrap peers
    vec![]                                    // Listen addresses
)
```

### Relay Server Requirement

The automated beta test expects a relay server on `127.0.0.1:9000`:

```bash
cargo run --package descord-relay --release
```

Without a relay, peers can still use direct connections but privacy features are limited.

---

## 🎯 Future Improvements

1. **Adaptive Timeout**: Scale timeout based on number of DHT peers
2. **Bootstrap Peer Pool**: Maintain a list of known-good bootstrap peers
3. **DHT Metrics**: Expose routing table size, query success rate
4. **Smart Retry**: Exponential backoff for bootstrap attempts
5. **Peer Quality Score**: Prioritize reliable peers in routing table

---

## 🐛 Troubleshooting

### "No DHT peers in routing table" keeps appearing

- This is normal if you haven't configured bootstrap peers
- Peers need to manually connect to each other first
- Use `connect <multiaddr>` to establish initial connections

### DHT operations still timing out

- Check that peers are actually connected: `network` command
- Verify at least 2-3 peers are connected
- Check firewall/NAT isn't blocking connections
- Ensure RocksDB libraries are available (see ROCKSDB_FIX.md)

### Bootstrap keeps failing

- Normal if no bootstrap peers configured
- Add bootstrap peers via `--bootstrap` CLI flag
- Or connect to peers manually with `connect` command

---

**Last Updated**: 2025-11-22
**Author**: GitHub Copilot
**Tested on**: Linux x86_64, Rust nightly 1.93.0
