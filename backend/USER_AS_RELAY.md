# User-as-Relay: Veilid-Style P2P Relay Network

## YES, Users Can Be Relays! 🎯

**No central server needed.** Users can volunteer to relay traffic for others.

## How It Works (Veilid Model)

### Architecture

```
┌─────────────────────────────────────────┐
│ Alice (Desktop, high uptime)            │
│ - Runs app normally                     │
│ - Enables "Relay Mode" in settings      │
│ - Becomes relay for others              │
│ - Gets priority when SHE needs relay    │
└─────────────────────────────────────────┘
         ↓ Relays traffic for ↓
         
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ Bob (Mobile) │    │ Carol (Home) │    │ Dave (Laptop)│
│ Uses Alice   │    │ Uses Alice   │    │ Uses Alice   │
│ as relay     │    │ as relay     │    │ as relay     │
└──────────────┘    └──────────────┘    └──────────────┘

Bob ──[Alice relays]──> Carol
      ↑ Neither knows the other's real IP
```

### User Perspective

**Alice (Desktop user with good internet):**
```
Settings > Network > Relay Mode
├─ [x] Enable Relay Mode
├─ Max bandwidth: 500 MB/hour
├─ Only when idle: [ ]
├─ Priority access: ✓ Enabled
│
└─ Status:
   ├─ Circuits relayed today: 42
   ├─ Bandwidth contributed: 2.3 GB
   ├─ Reputation: ★★★★☆ (87/100)
   └─ Earned credits: 150 (spend on your own relay usage)
```

**Bob (Mobile user, behind NAT):**
```
Settings > Network > Relay Mode
├─ [ ] Enable Relay Mode
├─ Use volunteer relays: [x]
│
└─ Status:
   ├─ Connected via: alice-relay (★★★★☆)
   ├─ Your IP hidden: ✓
   └─ Credits used: 12
```

### Discovery (No Central Directory Needed)

**Step 1: Advertise Availability**
```rust
// Alice enables relay mode
pub async fn enable_relay_mode(&mut self, config: RelayConfig) -> Result<()> {
    // Start listening as relay server
    let relay_server = relay::server::new(self.peer_id);
    self.swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;
    
    // Advertise on DHT: "I'm available as relay"
    self.publish_relay_availability().await?;
    
    // Advertise on mDNS for local peers
    // (automatically handled by libp2p)
    
    Ok(())
}

async fn publish_relay_availability(&self) -> Result<()> {
    // Publish to DHT under "relays/available/{peer_id}"
    let key = format!("relays/available/{}", self.peer_id);
    let value = RelayAdvertisement {
        peer_id: self.peer_id,
        addresses: self.swarm.external_addresses().collect(),
        capacity: self.relay_config.max_circuits,
        reputation: self.reputation_score,
        price: 0, // Free for community network
        uptime_estimate: 0.95, // 95% uptime
        last_seen: current_timestamp(),
    };
    
    self.dht.put(key, serialize(&value)?).await?;
    Ok(())
}
```

**Step 2: Discover Available Relays**
```rust
// Bob needs a relay
pub async fn find_relay_peers(&self) -> Vec<RelayPeer> {
    let mut relays = Vec::new();
    
    // 1. Check local network (mDNS)
    //    Fastest, zero-hop discovery
    for peer in self.mdns.discovered_peers() {
        if peer.is_relay_capable {
            relays.push(peer);
        }
    }
    
    // 2. Query DHT for volunteer relays
    let dht_relays = self.dht.get_providers("relays/available").await;
    relays.extend(dht_relays);
    
    // 3. Fallback: Use bootstrap relays (optional)
    if relays.is_empty() {
        relays.extend(BOOTSTRAP_RELAYS);
    }
    
    // Sort by reputation and proximity
    relays.sort_by_key(|r| (r.reputation, r.latency_estimate));
    relays
}
```

**Step 3: Connect Through Relay**
```rust
// Bob connects to Carol via Alice's relay
pub async fn connect_to_peer_via_relay(
    &mut self,
    target_peer_id: PeerId,
) -> Result<()> {
    // Find best relay
    let relays = self.find_relay_peers().await;
    let relay = relays.first().ok_or("No relays available")?;
    
    // Dial through relay
    let relay_addr = relay_multiaddr(
        &relay.address,
        &relay.peer_id,
        &target_peer_id
    );
    
    self.swarm.dial(relay_addr)?;
    Ok(())
}
```

### Zero Central Infrastructure

**What you DON'T need:**
- ❌ No relay servers to deploy
- ❌ No DNS records
- ❌ No VPS hosting costs
- ❌ No relay coordination server
- ❌ No central directory

**What handles coordination:**
- ✅ **DHT**: Distributed relay directory (Kademlia)
- ✅ **mDNS**: Local network relay discovery
- ✅ **Gossipsub**: Relay reputation gossip
- ✅ **Bootstrap nodes**: Only for initial DHT connection

### Incentive Mechanism

**Priority Access (Simplest)**
```rust
pub struct RelayCredits {
    pub earned: u64,  // Bytes relayed for others
    pub spent: u64,   // Bytes relayed for me
}

impl RelayPolicy {
    fn should_accept_circuit(&self, requester: &PeerId) -> bool {
        let credits = self.get_credits(requester);
        
        // If you've helped others, you get helped
        if credits.earned > credits.spent {
            return true;
        }
        
        // Allow some baseline usage even if not helping
        if credits.spent < FREE_TIER_LIMIT {
            return true;
        }
        
        // Otherwise, need to contribute
        false
    }
}
```

**Reputation System**
```rust
pub struct RelayReputation {
    pub successful_circuits: u64,
    pub failed_circuits: u64,
    pub uptime_percentage: f32,
    pub avg_latency_ms: u32,
    pub bandwidth_contributed_mb: u64,
}

// Good actors get priority in relay selection
fn calculate_reputation_score(rep: &RelayReputation) -> u32 {
    let reliability = rep.successful_circuits as f32 
        / (rep.successful_circuits + rep.failed_circuits).max(1) as f32;
    
    let uptime_score = rep.uptime_percentage * 40.0;
    let reliability_score = reliability * 40.0;
    let speed_score = (100.0 / rep.avg_latency_ms as f32).min(20.0);
    
    (uptime_score + reliability_score + speed_score) as u32
}
```

## Implementation Phases

### Phase 1: Bootstrap Relay Only (1 week)
Deploy 2 VPS relays to prove concept works.

```rust
// Hardcoded bootstrap relays
const BOOTSTRAP_RELAYS: &[&str] = &[
    "/ip4/1.2.3.4/tcp/4001/p2p/12D3Koo...",
    "/ip4/5.6.7.8/tcp/4001/p2p/12D3Koo...",
];
```

### Phase 2: User-Operated Discovery (2 weeks)
Enable users to volunteer as relays.

```rust
// Add to settings
pub struct NetworkConfig {
    pub relay_mode: RelayMode,
    pub max_relay_bandwidth_mb_hour: u64,
    pub relay_only_when_idle: bool,
}

pub enum RelayMode {
    ClientOnly,          // Use relays, don't run one
    Cooperative,         // Run relay + use relays
    DedicatedServer,     // VPS/always-on relay
}
```

### Phase 3: DHT-Based Discovery (3 weeks)
Remove hardcoded relays, use DHT entirely.

```rust
// No hardcoded relays needed!
pub async fn discover_relays(&self) -> Vec<RelayPeer> {
    // Query DHT for active relays
    self.dht.get_providers("relays/available").await
}
```

### Phase 4: Incentives & Reputation (4 weeks)
Track contributions, prioritize helpful peers.

```rust
// Track relay contributions
pub struct PeerStats {
    pub bytes_relayed_for_me: u64,
    pub bytes_relayed_by_me: u64,
    pub reputation: u32,
}
```

## Bootstrap Strategy (Cold Start Problem)

**Problem:** First user has no relays available.

**Solution: Progressive Fallback**

```rust
async fn connect_with_fallback(&mut self, target: PeerId) -> Result<()> {
    // 1. Try user-operated relays (DHT/mDNS)
    if let Ok(relay) = self.find_user_relay().await {
        return self.dial_via_relay(target, relay).await;
    }
    
    // 2. Try bootstrap relays (VPS fallback)
    if let Ok(relay) = self.find_bootstrap_relay().await {
        return self.dial_via_relay(target, relay).await;
    }
    
    // 3. Direct connection (if both peers reachable)
    if !self.requires_relay() {
        return self.dial_direct(target).await;
    }
    
    // 4. Fail with helpful message
    Err("No relays available. Enable relay mode to help the network?")
}
```

**Initial Launch:**
- Deploy 2-3 VPS bootstrap relays
- Gradual migration as users enable relay mode
- Eventually, shut down VPS relays when network is self-sustaining

## Comparison: Centralized vs Decentralized

| Aspect | Central VPS Relays | User-Operated Relays |
|--------|-------------------|---------------------|
| **Cost** | $15-30/month | $0 (free) |
| **Privacy** | Relay sees all metadata | Distributed, no single view |
| **Reliability** | High (99.9% uptime) | Variable (depends on volunteers) |
| **Scalability** | Limited by VPS capacity | Scales with user growth |
| **Setup** | 30 minutes | 2-3 weeks development |
| **Censorship Resistance** | Low (can be blocked) | High (no central point) |
| **Performance** | Fast (datacenter) | Variable (home internet) |

## Recommended Hybrid Approach

**For Descord:**

1. **Launch:** Use 2 VPS bootstrap relays ($10/month)
2. **Month 1:** Add "Relay Mode" setting, encourage volunteers
3. **Month 2:** DHT-based relay discovery goes live
4. **Month 3:** 50+ user relays available, reduce VPS relays to 1
5. **Month 6:** 500+ user relays, shut down VPS relays entirely

**This gives:**
- ✅ Immediate functionality (bootstrap relays)
- ✅ Path to decentralization (user relays)
- ✅ Graceful migration (hybrid mode)
- ✅ Low long-term cost (community-operated)

## Code Changes Needed

### 1. Enable Relay Server in Swarm
```rust
// In network/node.rs
let behaviour = DescordBehaviour {
    gossipsub,
    mdns,
    relay_client,  // Already added ✅
    relay_server,  // NEW: Act as relay for others
};
```

### 2. DHT Relay Advertisement
```rust
// In network/relay.rs
pub async fn advertise_as_relay(&self) -> Result<()> {
    // Publish to DHT
    // Handle relay requests
}
```

### 3. Relay Discovery
```rust
pub async fn discover_relays(&self) -> Vec<RelayPeer> {
    // Query DHT + mDNS
}
```

### 4. Settings Integration
```rust
// In client.rs
pub async fn set_relay_mode(&mut self, mode: RelayMode) -> Result<()> {
    // Update config
    // Start/stop relay server
}
```

## Bottom Line

**You asked: "Can users be relays instead of central servers?"**

**Answer: YES!** 

- ✅ Technically possible with libp2p (Veilid does this)
- ✅ No central server required (DHT handles discovery)
- ✅ Bootstrap relays optional (for initial launch)
- ✅ Scales naturally with user growth
- ✅ Zero long-term hosting costs

**Next step:** Implement Phase 1 (bootstrap relays) to prove it works, then add Phase 2 (user relay opt-in) for decentralization.

**Want me to implement the user-operated relay discovery code?**
