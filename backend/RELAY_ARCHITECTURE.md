# Relay Server Architecture & Deployment Strategy

## Three Deployment Models

### Model 1: Centralized Relays (Easiest, Current Approach)
**What it is:** Run 3-5 relay servers on cloud VPS instances

```
User A ──┐
User B ──┼──> relay1.descord.io (VPS in US-East)
User C ──┘

User D ──┐
User E ──┼──> relay2.descord.io (VPS in EU-West)
User F ──┘
```

**Pros:**
- Simple to deploy and maintain
- Low latency (professionally hosted)
- Guaranteed uptime
- Easy load balancing

**Cons:**
- Costs money (~$5-10/month per relay)
- Single point of failure if relays go down
- Trust model: Users must trust relay operators not to log metadata
- Centralization defeats some privacy goals

**Implementation:**
```bash
# Deploy on DigitalOcean/AWS/Hetzner
cargo install libp2p-relay-server
libp2p-relay-server --port 4001 --metrics-port 9090

# Hardcode in default_relay_addresses():
"/dns4/relay1.descord.io/tcp/4001/p2p/12D3..."
"/dns4/relay2.descord.io/tcp/4001/p2p/12D3..."
```

**Cost:** $15-30/month for 3 relays

---

### Model 2: User-Operated Relays (Veilid-Style) ⭐ **RECOMMENDED**
**What it is:** Users volunteer to run relay nodes, get prioritized routing

```
Alice (Desktop, always-on) ──> Runs relay server
  ↓ Helps relay traffic for:
  ├─> Bob
  ├─> Carol
  └─> Dave

Bob (Mobile) ──> Doesn't run relay, uses Alice's
```

**How Veilid does it:**
1. Desktop clients can opt-in to be relay nodes
2. Relay nodes get priority when they need relay services
3. Creates a distributed relay network (no central server needed)
4. Reputation system: good relays get more traffic

**Pros:**
- **Zero hosting costs** 
- Truly decentralized (no single point of failure)
- Privacy-preserving (no single entity sees all metadata)
- Scales naturally with user growth
- Incentive alignment: help others, get helped

**Cons:**
- Variable performance (home internet vs datacenter)
- NAT traversal complexity (users behind routers)
- Reliability issues (users go offline)
- Need fallback to centralized relays

**Implementation:**
```rust
pub struct RelayMode {
    pub mode: RelayOperationMode,
    pub incentive: RelayIncentive,
}

pub enum RelayOperationMode {
    // Only consume relay services
    ClientOnly,
    
    // Run relay server AND consume services (Veilid-style)
    Cooperative {
        max_bandwidth: u64,      // MB/hour
        max_concurrent_circuits: usize,
        uptime_target: f32,      // 0.0 - 1.0
    },
    
    // Dedicated relay server (VPS-hosted)
    DedicatedServer {
        public: bool,            // Listed in public relay directory
    },
}

pub enum RelayIncentive {
    // Priority routing when you need relay
    PriorityAccess,
    
    // Future: Token/credit system
    Credits { earned: u64, spent: u64 },
    
    // Altruism (no reward)
    Volunteer,
}
```

**Bootstrap Process:**
1. App starts → Try to connect to known relay nodes
2. If no relays available → Prompt user: "Enable relay mode to help the network?"
3. User opts in → App listens on public port, advertises as relay
4. DHT publishes relay availability: `relays/available/{peer_id}`
5. Other users discover and use your relay

**Discovery:**
```rust
// Find available volunteer relays
pub async fn discover_relays(&self) -> Vec<RelayInfo> {
    // 1. Try DHT: get_providers("relays/available")
    // 2. Try mDNS: find local network relays
    // 3. Fallback: hardcoded public relays
    
    let mut relays = Vec::new();
    
    // Prefer relays with high reputation
    relays.sort_by(|a, b| b.reputation.cmp(&a.reputation));
    
    // Return top 5
    relays.truncate(5);
    relays
}

pub struct RelayInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub reputation: u32,         // 0-100 based on uptime, speed
    pub capacity: RelayCapacity, // Available bandwidth
    pub last_seen: u64,          // Unix timestamp
}
```

**Reputation System:**
```rust
pub struct RelayReputation {
    pub uptime_percentage: f32,        // % time online
    pub avg_latency_ms: u32,           // Response time
    pub successful_circuits: u64,      // Completed relays
    pub failed_circuits: u64,          // Dropped connections
    pub bandwidth_contributed_mb: u64, // Total data relayed
}

// Score: (uptime * 40) + (reliability * 30) + (speed * 30)
fn calculate_score(rep: &RelayReputation) -> u32 {
    let reliability = rep.successful_circuits as f32 
        / (rep.successful_circuits + rep.failed_circuits) as f32;
    let speed_score = (1000.0 / rep.avg_latency_ms as f32).min(1.0);
    
    ((rep.uptime_percentage * 40.0) +
     (reliability * 30.0) +
     (speed_score * 30.0)) as u32
}
```

**UI Flow:**
```
┌────────────────────────────────────┐
│ Settings > Network > Relay Mode    │
├────────────────────────────────────┤
│                                    │
│ ○ Client Only (Use relays, don't   │
│   run one)                         │
│                                    │
│ ● Cooperative (RECOMMENDED)        │
│   Help others, get priority access │
│   └─ Max bandwidth: [500 MB/hour]  │
│   └─ When idle: ☑ Always           │
│                 ☐ Only when charging│
│                                    │
│ ○ Dedicated Server (Advanced)      │
│                                    │
│ Your contribution:                 │
│ ├─ 2.3 GB relayed this month      │
│ ├─ 43 circuits helped             │
│ └─ Reputation: ★★★★☆ (82/100)     │
│                                    │
│ [Learn More] [Save Settings]       │
└────────────────────────────────────┘
```

---

### Model 3: Hybrid (Best of Both Worlds) ⭐⭐ **ULTIMATE**
**What it is:** Combine centralized fallback + distributed user relays

```
Layer 1: User-Operated Relays (Primary)
  ├─> Alice's desktop relay
  ├─> Bob's home server relay
  └─> Carol's always-on PC relay

Layer 2: Fallback Public Relays (When Layer 1 unavailable)
  ├─> relay1.descord.io (VPS)
  └─> relay2.descord.io (VPS)

Layer 3: Tor Integration (Maximum privacy, slowest)
  └─> .onion addresses (optional)
```

**Algorithm:**
```rust
async fn select_relay(&self, privacy_level: PrivacyLevel) -> Multiaddr {
    match privacy_level {
        PrivacyLevel::Low => {
            // Direct P2P, no relay
            return direct_connection();
        }
        
        PrivacyLevel::High => {
            // Try user relays first, fallback to public
            if let Some(relay) = self.discover_user_relay().await {
                return relay;
            }
            self.get_public_relay().await
        }
        
        PrivacyLevel::Maximum => {
            // Tor only, no fallback
            self.get_tor_relay().await
        }
    }
}

async fn discover_user_relay(&self) -> Option<Multiaddr> {
    let relays = self.discover_relays().await;
    
    // Filter by:
    // 1. Reputation > 70
    // 2. Available capacity
    // 3. Geographic proximity (lower latency)
    
    relays.into_iter()
        .filter(|r| r.reputation > 70 && r.capacity.available())
        .min_by_key(|r| r.avg_latency_ms())
        .map(|r| r.addresses[0].clone())
}
```

**Cost:** $5-10/month for 2 fallback relays + free user-operated network

---

## Recommended Implementation Plan

### Phase 1: Centralized Bootstrap (Week 1)
- Deploy 2 VPS relays (relay1.descord.io, relay2.descord.io)
- Hardcode in `default_relay_addresses()`
- Get basic relay working
- **Goal:** Prove IP privacy works

### Phase 2: User-Operated Relays (Week 2-3)
- Add "Relay Mode" setting to CLI/GUI
- Implement relay discovery via DHT
- Add reputation tracking
- **Goal:** First user-operated relay running

### Phase 3: Reputation & Incentives (Week 4)
- Track relay performance metrics
- Implement priority access for relay operators
- Add relay directory to DHT
- **Goal:** Self-sustaining relay network

### Phase 4: Tor Integration (Optional, Week 5+)
- Add libp2p-tor-transport
- Support .onion addresses
- Maximum privacy mode
- **Goal:** Censorship resistance

---

## Comparison to Other Projects

### Veilid
- **Relay Model:** Distributed user-operated relays only
- **Incentive:** Priority routing for relay operators
- **No fallback:** If no relays available, connection fails
- **Descord Difference:** We add fallback public relays

### Tor
- **Relay Model:** Volunteer-run relay network (thousands of nodes)
- **Incentive:** Altruism, no automatic priority
- **Very slow:** 3-hop routing adds 200-500ms latency
- **Descord Difference:** 1-hop relay is faster, can upgrade to Tor for max privacy

### Discord/Slack
- **Relay Model:** All traffic through their servers (100% centralized)
- **Privacy:** Zero (they see everything)
- **Performance:** Fast (datacenters)
- **Descord Difference:** Decentralized, privacy-preserving

---

## Technical Deep Dive: How Relay Works

### Direct Connection (Current, No Privacy)
```
Alice (192.168.1.5) ──────────────> Bob (203.0.113.42)
       ^                                    ^
       └──── Both IPs visible ─────────────┘
```

### Relay Connection (Private)
```
Alice (192.168.1.5) ──encrypted──> Relay (1.2.3.4) ──encrypted──> Bob (203.0.113.42)
       ^                              |                               ^
       └── Alice only sees ──────────┘                               |
           Relay IP (1.2.3.4)                                        |
                                                                     |
           Bob only sees ─────────────────────────────────────────────┘
           Relay IP (1.2.3.4)

Neither peer knows the other's real IP!
```

### What Relay Sees (Metadata Exposure)
```
Relay knows:
- Alice's IP: 192.168.1.5
- Bob's IP: 203.0.113.42
- They are communicating
- How much data transferred
- When they connected

Relay CANNOT see:
- Message content (encrypted by MLS)
- What space they're in (encrypted metadata)
- Who else is in the space
```

### Multi-Hop for Maximum Privacy (Future)
```
Alice ─> Relay1 ─> Relay2 ─> Relay3 ─> Bob

Relay1 knows: Alice's IP, Relay2's IP (not Bob)
Relay2 knows: Relay1's IP, Relay3's IP (not Alice or Bob)
Relay3 knows: Relay2's IP, Bob's IP (not Alice)

No single relay knows both endpoints!
```

---

## Immediate Next Steps

**Quick Win (2 hours):**
1. Deploy 1 relay server on DigitalOcean ($6/month)
2. Update `default_relay_addresses()` with relay IP
3. Test `dial_via_relay()` actually hides IPs
4. Document in `GETTING_STARTED.md`

**Veilid-Style (2 weeks):**
1. Add "Run as relay" checkbox in settings
2. Implement relay discovery via DHT
3. Track relay reputation
4. Priority access for relay operators
5. Launch with bootstrap relays + user network

**Which do you prefer?**
