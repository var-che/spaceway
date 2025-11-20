# Descord Beta Testing Guide

**Version:** 0.1.0-beta  
**Date:** November 20, 2025

Welcome to Descord beta testing! This guide will help you set up and test the privacy-preserving P2P messaging platform.

---

## Prerequisites

- **Rust** 1.75+ installed ([rustup.rs](https://rustup.rs))
- **Git** for cloning the repository
- **Terminal** (PowerShell on Windows, bash/zsh on Linux/macOS)
- **Internet connection** for DHT and relay connectivity

---

## Quick Start (Automated Test)

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/descord.git
cd descord
```

### 2. Start Relay Server

Open a terminal and run:

```bash
cargo run --package descord-relay --release
```

You should see:
```
🚀 Descord Relay Server
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📋 Configuration:
   Max connections: 100
   Bandwidth limit: 10.00 MB/s
   ...
```

**Keep this terminal open** - the relay server must run during testing.

### 3. Run Automated Beta Test

Open a **new terminal** and run:

```bash
cargo test --package descord-core --test beta_test -- --ignored --nocapture
```

This will automatically:
- Initialize 3 users (Alice, Bob, Charlie)
- Connect them via relay (no direct IPs)
- Create spaces, channels, and threads
- Test messaging with E2EE
- Verify privacy guarantees
- Test relay rotation

**Expected duration:** ~60 seconds

---

## Manual Beta Testing (For Humans)

### Scenario 1: Two-Person Chat

#### Terminal 1 - Alice

```bash
# Start Alice's client
cd descord
cargo build --release

# Run interactive session (future CLI)
# For now, use Rust API directly
```

Create file `alice.rs`:
```rust
use descord_core::{Client, ClientConfig, crypto::Keypair};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Alice
    let keypair = Keypair::generate();
    let config = ClientConfig {
        storage_path: "./data/alice".into(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let alice = Client::new(keypair, config)?;
    alice.start().await?;
    
    println!("👤 Alice's PeerID: {}", alice.network_peer_id().await);
    
    // Connect to relay
    sleep(Duration::from_secs(2)).await;
    let relay = alice.auto_connect_relay().await?;
    println!("🔗 Connected to relay: {}", relay.peer_id);
    
    // Create space
    let (space, _, _) = alice.create_space(
        "Test Space".to_string(),
        None
    ).await?;
    println!("🏠 Created space: {}", hex::encode(&space.id.0[..8]));
    
    // Advertise on DHT
    alice.advertise_space_presence(space.id).await?;
    println!("📡 Advertised on DHT");
    
    // Create channel and thread
    let (channel, _) = alice.create_channel(space.id, "general".to_string(), None).await?;
    let (thread, _) = alice.create_thread(
        space.id,
        channel.id,
        Some("Hello".to_string()),
        "Hi Bob!".to_string()
    ).await?;
    
    println!("💬 Posted message");
    
    // Keep running
    println!("\n⏳ Waiting for Bob...");
    sleep(Duration::from_secs(300)).await;
    
    Ok(())
}
```

Run:
```bash
cargo run --bin alice
```

#### Terminal 2 - Bob

Create file `bob.rs`:
```rust
use descord_core::{Client, ClientConfig, crypto::Keypair};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize Bob
    let keypair = Keypair::generate();
    let config = ClientConfig {
        storage_path: "./data/bob".into(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let bob = Client::new(keypair, config)?;
    bob.start().await?;
    
    println!("👤 Bob's PeerID: {}", bob.network_peer_id().await);
    
    // Connect to relay
    sleep(Duration::from_secs(2)).await;
    let relay = bob.auto_connect_relay().await?;
    println!("🔗 Connected to relay: {}", relay.peer_id);
    
    // Ask Alice for space ID (out-of-band)
    println!("\n📋 Enter Alice's space ID:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let space_id_hex = input.trim();
    
    let space_id_bytes = hex::decode(space_id_hex)?;
    let mut space_id_array = [0u8; 32];
    space_id_array.copy_from_slice(&space_id_bytes);
    let space_id = descord_core::types::SpaceId(space_id_array);
    
    // Discover Alice via DHT
    sleep(Duration::from_secs(5)).await;
    let peers = bob.discover_space_peers(space_id).await?;
    println!("🔍 Discovered {} peer(s)", peers.len());
    
    // Connect to peers
    bob.connect_to_space_peers(space_id).await?;
    println!("🌐 Connected to peers via relay");
    
    println!("\n✅ Bob is now in the space!");
    println!("   (Messages would sync via GossipSub - TODO)");
    
    sleep(Duration::from_secs(300)).await;
    Ok(())
}
```

---

## Relay Server Deployment (Production)

### Option 1: Deploy on VPS (Recommended)

```bash
# On your VPS (Ubuntu/Debian)
sudo apt update
sudo apt install build-essential pkg-config libssl-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Clone and build
git clone https://github.com/your-org/descord.git
cd descord
cargo build --package descord-relay --release

# Create systemd service
sudo nano /etc/systemd/system/descord-relay.service
```

Add:
```ini
[Unit]
Description=Descord Relay Server
After=network.target

[Service]
Type=simple
User=descord
WorkingDirectory=/home/descord/descord
ExecStart=/home/descord/descord/target/release/descord-relay
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable descord-relay
sudo systemctl start descord-relay
sudo systemctl status descord-relay
```

### Option 2: Docker (Coming Soon)

```bash
docker run -p 9000:9000 -p 8080:8080 descord/relay:latest
```

---

## Testing Checklist

### ✅ Basic Functionality

- [ ] Relay server starts without errors
- [ ] Client connects to relay
- [ ] Client gets relay circuit address
- [ ] DHT advertisement succeeds
- [ ] DHT peer discovery finds peers
- [ ] Peer dial via relay succeeds
- [ ] Space creation works
- [ ] Channel creation works
- [ ] Thread creation works
- [ ] Message posting works

### ✅ Privacy Guarantees

- [ ] Client has no direct listen addresses
- [ ] All addresses are `/p2p-circuit` format
- [ ] Peers cannot see each other's IPs
- [ ] Relay rotation triggers every 5 minutes
- [ ] Messages are encrypted (check logs for ciphertext)
- [ ] DHT only shows relay addresses, not user IPs

### ✅ Performance

- [ ] Relay connection < 2 seconds
- [ ] DHT advertisement < 1 second
- [ ] DHT discovery < 5 seconds
- [ ] Peer dial via relay < 3 seconds
- [ ] Message posting < 500ms

### ✅ Reliability

- [ ] Client reconnects after relay disconnect
- [ ] Relay rotation doesn't drop connections
- [ ] Multiple clients can connect simultaneously
- [ ] No crashes under normal operation
- [ ] Storage persists across restarts

---

## Monitoring & Debugging

### Relay Server Monitoring

Access monitoring endpoint:
```bash
curl http://localhost:8080/stats
```

Returns:
```json
{
  "active_connections": 3,
  "total_bytes_sent": 1048576,
  "total_bytes_received": 524288,
  "uptime_seconds": 3600,
  "relay_reputation": 0.95
}
```

### Client Logging

Enable debug logs:
```bash
RUST_LOG=debug cargo test --test beta_test -- --ignored --nocapture
```

### Common Issues

**Issue:** "No relay available"
- **Solution:** Make sure relay server is running: `cargo run --package descord-relay --release`

**Issue:** "DHT discovery returns empty"
- **Solution:** Wait 30-60 seconds for DHT propagation, or check bootstrap peers

**Issue:** "Peer dial failed"
- **Solution:** Verify both peers are connected to same relay

**Issue:** "Connection timeout"
- **Solution:** Check firewall rules, ensure ports 9000 (libp2p) and 8080 (monitoring) are open

---

## Security Best Practices for Beta Testers

### ✅ DO:
- Use relay servers you trust
- Verify PeerIDs out-of-band (e.g., Signal, email)
- Keep relay rotation enabled (default)
- Report security issues privately to security@descord.org
- Test on non-production data

### ❌ DON'T:
- Share private keys
- Disable relay-only mode (exposes IP)
- Use on sensitive/production data (beta software!)
- Trust unknown relays for confidential communications
- Assume metadata privacy from relay operators

---

## Reporting Issues

### Bug Reports

Open an issue at: https://github.com/your-org/descord/issues

Include:
1. **Description** - What happened vs what you expected
2. **Steps to reproduce** - Exact commands/code
3. **Environment** - OS, Rust version (`rustc --version`)
4. **Logs** - Relevant log output (use `RUST_LOG=debug`)
5. **Test results** - Output from beta_test

### Security Vulnerabilities

**DO NOT** open public issues for security bugs.

Email: security@descord.org with:
- Description of vulnerability
- Proof of concept (if possible)
- Suggested fix (if any)

We aim to respond within 48 hours.

---

## Beta Test Phases

### Phase 1: Automated Testing (Current)
- Run automated beta test
- Verify all checks pass
- Duration: 1 week

### Phase 2: Manual Multi-User (Week 2)
- 3-5 beta testers
- Create spaces, chat, test features
- Report bugs and UX issues

### Phase 3: Relay Stress Test (Week 3)
- Deploy multiple relay servers
- Test with 10-20 concurrent users
- Monitor performance and reliability

### Phase 4: Privacy Audit (Week 4)
- Third-party security review
- Metadata leakage analysis
- Traffic correlation testing

---

## Success Criteria

Beta testing is successful if:

- ✅ **Functionality:** 95%+ of features work as expected
- ✅ **Stability:** < 1 crash per 100 hours of operation
- ✅ **Privacy:** No IP leakage to peers confirmed
- ✅ **Performance:** < 5s latency for message delivery
- ✅ **Security:** No critical vulnerabilities found

---

## Next Steps After Beta

1. **Production Release (v1.0)**
   - Fix all critical bugs
   - Complete GossipSub message propagation
   - Build mobile clients (iOS/Android)

2. **Public Relay Network**
   - Deploy 10+ relays globally
   - Implement relay reputation system
   - Add relay incentives (future)

3. **User Applications**
   - CLI application
   - Desktop GUI (Tauri/Electron)
   - Mobile apps
   - Web interface

---

## Contact & Support

- **GitHub:** https://github.com/your-org/descord
- **Discord (ironic):** discord.gg/descord-beta
- **Email:** beta@descord.org
- **Documentation:** https://docs.descord.org

---

## Thank You!

Thank you for beta testing Descord! Your feedback is crucial for building a privacy-preserving alternative to centralized platforms.

Together, we can make private communication accessible to everyone. 🔒✨

---

**License:** MIT OR Apache-2.0  
**Version:** 0.1.0-beta  
**Last Updated:** November 20, 2025
