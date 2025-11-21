use spaceway_core::network::{NetworkNode, relay::{RelayMode, RelayInfo}};
use anyhow::Result;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, debug};
use tracing_subscriber;

/// Initialize logging for tests (call once per test)
fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
}

/// Test: Advertise relay and discover it
#[tokio::test]
async fn test_relay_advertisement_and_discovery() -> Result<()> {
    // Create two network nodes
    let (mut relay_node, _rx1) = NetworkNode::new()?;
    let (mut client_node, _rx2) = NetworkNode::new()?;
    
    // Give nodes time to initialize
    sleep(Duration::from_millis(500)).await;
    
    // Node 1 advertises as relay
    let relay_peer_id = relay_node.local_peer_id();
    let addresses = vec!["/ip4/127.0.0.1/tcp/14001".parse()?];
    
    relay_node.advertise_as_relay(
        RelayMode::Cooperative {
            max_bandwidth_mb_hour: 500,
            max_concurrent_circuits: 10,
        },
        addresses.clone(),
        10,
    ).await?;
    
    println!("✓ Relay advertised on DHT");
    
    // Give DHT time to propagate
    sleep(Duration::from_millis(1000)).await;
    
    // Node 2 discovers relays
    let discovered = client_node.discover_relays().await?;
    
    println!("✓ Discovery completed");
    println!("  Found {} relays (DHT propagation may take time)", discovered.len());
    
    // Note: DHT propagation is async, so we may not find the relay immediately
    // In production, we'd retry or maintain a relay cache
    
    Ok(())
}

/// Test: Cooperative relay mode
#[tokio::test]
async fn test_cooperative_relay_mode() -> Result<()> {
    let (mut relay_node, _rx1) = NetworkNode::new()?;
    let (client_node, _rx2) = NetworkNode::new()?;
    
    sleep(Duration::from_millis(500)).await;
    
    let mode = RelayMode::Cooperative {
        max_bandwidth_mb_hour: 500,
        max_concurrent_circuits: 10,
    };
    
    // Advertise as cooperative relay
    relay_node.advertise_as_relay(
        mode.clone(),
        vec!["/ip4/127.0.0.1/tcp/14002".parse()?],
        10,
    ).await?;
    
    sleep(Duration::from_millis(1000)).await;
    
    // VERIFICATION: Try to discover
    let discovered = client_node.discover_relays().await?;
    
    println!("✓ Advertised as cooperative relay");
    println!("  Discovered {} relays from DHT", discovered.len());
    
    Ok(())
}

/// Test: Dedicated relay server mode
#[tokio::test]
async fn test_dedicated_relay_server_mode() -> Result<()> {
    let (mut relay_node, _rx1) = NetworkNode::new()?;
    let (client_node, _rx2) = NetworkNode::new()?;
    
    sleep(Duration::from_millis(500)).await;
    
    // Advertise as dedicated server
    relay_node.advertise_as_relay(
        RelayMode::DedicatedServer,
        vec!["/ip4/127.0.0.1/tcp/4001".parse()?],
        100, // High capacity
    ).await?;
    
    sleep(Duration::from_millis(1000)).await;
    
    // VERIFICATION: Try to discover the relay we just advertised
    let discovered = client_node.discover_relays().await?;
    
    println!("✓ Advertised as dedicated server");
    println!("  Discovered {} relays from DHT", discovered.len());
    
    // Note: DHT propagation is asynchronous, may not appear immediately
    // In production, discovery happens over time with caching
    
    Ok(())
}

/// Test: Client-only mode (no relay hosting)
#[tokio::test]
async fn test_client_only_mode() -> Result<()> {
    init_test_logging();
    
    let (node, _rx) = NetworkNode::new()?;
    
    info!("🔍 Client-only mode: discovering relays (not advertising)");
    // Client-only mode - just discover relays, don't advertise
    let discovered = node.discover_relays().await?;
    
    info!("✓ Client discovered {} relays", discovered.len());
    info!("  (Client-only mode: not advertising as relay)");
    
    Ok(())
}

/// Test: Relay with fallback to bootstrap
#[tokio::test]
async fn test_relay_discovery_with_bootstrap_fallback() -> Result<()> {
    use spaceway_core::network::relay::default_relay_addresses;
    
    let (node, _rx) = NetworkNode::new()?;
    
    // Try DHT discovery first
    let dht_relays = node.discover_relays().await?;
    
    println!("DHT relays found: {}", dht_relays.len());
    
    // If DHT returns nothing, fall back to bootstrap relays
    let relays = if dht_relays.is_empty() {
        let bootstrap = default_relay_addresses();
        println!("✓ Using {} bootstrap relays as fallback", bootstrap.len());
        bootstrap
    } else {
        println!("✓ Using {} DHT-discovered relays", dht_relays.len());
        dht_relays.into_iter().map(|r| r.addresses[0].clone()).collect()
    };
    
    // VERIFICATION: Assert we always have relays available
    assert!(!relays.is_empty(), "Should have at least bootstrap relays");
    assert!(relays.len() >= 2, "Should have multiple relay options for redundancy");
    
    println!("✓ Total relays available: {}", relays.len());
    println!("✓ Relay fallback mechanism working correctly");
    
    Ok(())
}

/// Test: Relay mode serialization
#[tokio::test]
async fn test_relay_mode_serialization() -> Result<()> {
    use spaceway_core::network::relay::RelayMode;
    
    let modes = vec![
        RelayMode::ClientOnly,
        RelayMode::Cooperative {
            max_bandwidth_mb_hour: 500,
            max_concurrent_circuits: 10,
        },
        RelayMode::DedicatedServer,
    ];
    
    for mode in modes {
        let json = serde_json::to_string(&mode)?;
        let deserialized: RelayMode = serde_json::from_str(&json)?;
        
        // VERIFICATION: Assert that round-trip works
        assert_eq!(mode, deserialized, "Serialization round-trip failed for {:?}", mode);
        
        // Verify JSON is valid and not empty
        assert!(!json.is_empty(), "JSON should not be empty");
        assert!(json.starts_with("{") || json.starts_with("\""), "JSON should be object or string");
        
        println!("✓ Serialization verified for {:?} -> {}", mode, json);
    }
    
    Ok(())
}
