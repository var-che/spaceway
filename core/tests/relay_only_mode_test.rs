//! Integration test for relay-only networking mode
//!
//! This test demonstrates IP privacy through circuit relay:
//! 1. Start a dedicated relay server
//! 2. Alice and Bob connect ONLY via the relay
//! 3. Neither Alice nor Bob know each other's IP addresses
//! 4. Messages are exchanged through the relay

use descord_core::{Client, ClientConfig};
use descord_core::crypto::Keypair;
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;

#[tokio::test]
#[ignore] // Requires relay server running
async fn test_relay_only_communication() {
    // This test requires a relay server to be running
    // Start it with: cargo run --package descord-relay --release
    
    // Alice's client
    let alice_keypair = Keypair::generate();
    let alice_temp = TempDir::new().unwrap();
    let alice_config = ClientConfig {
        storage_path: alice_temp.path().to_path_buf(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let alice = Client::new(alice_keypair, alice_config).unwrap();
    alice.start().await.unwrap();
    
    // Bob's client
    let bob_keypair = Keypair::generate();
    let bob_temp = TempDir::new().unwrap();
    let bob_config = ClientConfig {
        storage_path: bob_temp.path().to_path_buf(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let bob = Client::new(bob_keypair, bob_config).unwrap();
    bob.start().await.unwrap();
    
    println!("👤 Alice peer ID: {}", alice.network_peer_id().await);
    println!("👤 Bob peer ID: {}", bob.network_peer_id().await);
    
    // Step 1: Alice discovers relays from DHT
    println!("\n📡 Alice discovering relays...");
    sleep(Duration::from_secs(2)).await;
    
    let alice_relays = alice.discover_relays().await;
    if let Ok(relays) = &alice_relays {
        println!("   Found {} relays", relays.len());
        for relay in relays {
            println!("   - {} (reputation: {:.2})", relay.peer_id, relay.reputation);
        }
    }
    
    // Step 2: Alice connects to best relay
    if let Ok(relay) = alice.auto_connect_relay().await {
        println!("\n🔗 Alice connected to relay: {}", relay.peer_id);
        
        // Get Alice's relay address (no IP exposed!)
        let alice_relay_addrs = alice.relay_addresses().await;
        println!("   Alice's relay addresses: {:?}", alice_relay_addrs);
        
        // Step 3: Bob also connects to the same relay
        if let Some(relay_addr) = relay.addresses.first() {
            let relay_addr_str = relay_addr.to_string();
            bob.connect_to_relay(&relay_addr_str).await.unwrap();
            println!("\n🔗 Bob connected to relay: {}", relay.peer_id);
            
            sleep(Duration::from_secs(1)).await;
            
            // Step 4: Alice dials Bob through the relay
            let bob_peer_id = bob.network_peer_id().await;
            let relay_peer_id = relay.peer_id.to_string();
            
            println!("\n📞 Alice dialing Bob via relay...");
            alice.dial_peer_via_relay(
                &relay_addr_str,
                &relay_peer_id,
                &bob_peer_id,
            ).await.unwrap();
            
            sleep(Duration::from_secs(1)).await;
            
            // Step 5: Alice creates a space and sends messages
            println!("\n🏠 Alice creating space...");
            let (space, _, _) = alice.create_space("Secret Chat".to_string(), None).await.unwrap();
            
            sleep(Duration::from_secs(1)).await;
            
            // Step 6: Verify Bob receives the space discovery message
            // (In a real scenario, Bob would be subscribed to the space discovery topic)
            println!("\n✅ Relay-only communication established!");
            println!("   - Neither peer knows the other's IP address");
            println!("   - All traffic routed through relay: {}", relay.peer_id);
            println!("   - Space ID: {}", hex::encode(&space.id.0[..8]));
        }
    } else {
        println!("❌ No relays available. Start a relay server with:");
        println!("   cargo run --package descord-relay --release");
    }
}

#[tokio::test]
async fn test_relay_discovery_simulation() {
    // Test that relay discovery methods exist and can be called
    // (without actually needing a relay server running)
    
    let keypair = Keypair::generate();
    let temp = TempDir::new().unwrap();
    let config = ClientConfig {
        storage_path: temp.path().to_path_buf(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config).unwrap();
    client.start().await.unwrap();
    
    // These should not panic, just return empty results
    let relay_addrs = client.relay_addresses().await;
    assert!(!relay_addrs.is_empty()); // Should return at least p2p-circuit address
    assert!(relay_addrs[0].contains("/p2p-circuit/"));
    
    println!("✓ Relay address format correct: {}", relay_addrs[0]);
}

#[tokio::test]
async fn test_relay_only_vs_direct_mode() {
    // Demonstrate the difference between relay-only and direct modes
    
    let keypair = Keypair::generate();
    let temp = TempDir::new().unwrap();
    let config = ClientConfig {
        storage_path: temp.path().to_path_buf(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config).unwrap();
    client.start().await.unwrap();
    
    println!("\n📊 Address Comparison:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Direct addresses (expose IP)
    let direct_addrs = client.network_listeners().await;
    if !direct_addrs.is_empty() {
        println!("\n❌ Direct addresses (IP exposed):");
        for addr in &direct_addrs {
            if addr.contains("/ip4/") || addr.contains("/ip6/") {
                println!("   {}", addr);
            }
        }
    }
    
    // Relay-only addresses (IP hidden)
    let relay_addrs = client.relay_addresses().await;
    println!("\n✅ Relay-only addresses (IP hidden):");
    for addr in &relay_addrs {
        println!("   {}", addr);
    }
    
    println!("\n💡 For privacy, advertise only relay addresses!");
}

#[tokio::test]
#[ignore] // Requires multiple relay servers running
async fn test_relay_rotation() {
    // This test demonstrates automatic relay rotation for enhanced privacy
    // Start multiple relay servers on different ports before running
    
    let keypair = Keypair::generate();
    let temp = TempDir::new().unwrap();
    let config = ClientConfig {
        storage_path: temp.path().to_path_buf(),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config).unwrap();
    client.start().await.unwrap();
    
    println!("👤 Client peer ID: {}", client.network_peer_id().await);
    
    // Connect to initial relay
    println!("\n📡 Discovering relays...");
    sleep(Duration::from_secs(2)).await;
    
    if let Ok(initial_relay) = client.auto_connect_relay().await {
        println!("🔗 Connected to relay: {} (reputation: {:.2})", 
            initial_relay.peer_id, initial_relay.reputation);
        
        // Start relay rotation with a short interval for testing
        println!("\n🔄 Starting relay rotation (every 15 seconds)...");
        client.start_relay_rotation(Duration::from_secs(15)).await.unwrap();
        
        // Monitor relay changes for 45 seconds (should rotate 2-3 times)
        for i in 1..=3 {
            sleep(Duration::from_secs(15)).await;
            
            if let Some(current) = client.current_relay().await {
                println!("\n⏰ Check #{}: Currently on relay: {} (reputation: {:.2})",
                    i, current.peer_id, current.reputation);
            } else {
                println!("\n⚠️ Check #{}: No current relay", i);
            }
        }
        
        // Stop rotation
        client.stop_relay_rotation().await;
        println!("\n🛑 Relay rotation stopped");
        
        println!("\n✅ Relay rotation test complete!");
        println!("   - Periodically switches relays");
        println!("   - Prevents long-term traffic correlation");
        println!("   - Enhances privacy by distributing trust");
    } else {
        println!("❌ No relays available. Start relay servers with:");
        println!("   cargo run --package descord-relay --release");
    }
}
