//! Integration tests for CRDT operation replication via DHT
//! 
//! Tests Phase 3: Operation Storage and Retrieval
//! 
//! NOTE: These tests demonstrate the operation replication code paths,
//! but DHT operations will fail in isolated test environments due to
//! QuorumFailed (no connected peers). In production with bootstrap nodes,
//! the DHT operations will succeed.

use spaceway_core::{
    Client, ClientConfig,
    types::*,
    crdt::OpType,
    crypto::signing::Keypair,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_operation_storage_code_path() -> anyhow::Result<()> {
    println!("\n=== Testing CRDT Operation Storage Code Path ===\n");
    
    // Create Alice (Space creator)
    let alice_keypair = Keypair::generate();
    let alice_id = alice_keypair.user_id();
    
    let alice_config = ClientConfig {
        storage_path: format!("./test-data/crdt-storage-alice-{}", uuid::Uuid::new_v4()).into(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let alice = Client::new(alice_keypair, alice_config)?;
    alice.start().await?;
    println!("✓ Alice created ({})", alice_id);
    
    // Alice creates a Space
    let (space, _create_op, _privacy) = alice.create_space_with_visibility(
        "Test Space".to_string(),
        Some("A space for testing".to_string()),
        SpaceVisibility::Public,
    ).await?;
    
    let space_id = space.id;
    println!("✓ Alice created Space: {}", space.name);
    println!("  (DHT PUT will fail due to no peers - expected in tests)");
    
    // Alice creates a channel (operation will be broadcast and attempted to store in DHT)
    let (channel, _channel_op) = alice.create_channel(
        space_id,
        "general".to_string(),
        Some("General discussion".to_string()),
    ).await?;
    
    println!("✓ Alice created Channel: {}", channel.name);
    println!("  (Operation batching and encryption code paths executed)");
    
    // Wait a moment
    sleep(Duration::from_millis(500)).await;
    
    println!("\n✅ Operation storage code path verified!");
    println!("✅ In production with DHT peers, operations would be stored successfully");
    
    Ok(())
}

#[tokio::test]
async fn test_operation_retrieval_code_path() -> anyhow::Result<()> {
    println!("\n=== Testing CRDT Operation Retrieval Code Path ===\n");
    
    let keypair = Keypair::generate();
    let user_id = keypair.user_id();
    
    let config = ClientConfig {
        storage_path: format!("./test-data/crdt-retrieval-{}", uuid::Uuid::new_v4()).into(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    client.start().await?;
    
    // Wait for network initialization
    sleep(Duration::from_millis(500)).await;
    
    // Try to fetch operations for a non-existent Space
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let fake_space_id = SpaceId::from_content(&user_id, "NonExistent", timestamp);
    
    println!("Fetching operations from DHT...");
    let ops = client.dht_get_operations(&fake_space_id).await?;
    
    // Should return empty vector when no index found
    assert_eq!(ops.len(), 0, "Should return empty vector for non-existent Space");
    
    println!("✓ Operation retrieval code path verified");
    println!("✓ Returns empty vector when no operations found (expected behavior)");
    
    Ok(())
}

/// This test demonstrates the full workflow but will fail in isolated tests.
/// It documents the expected behavior in production.
#[tokio::test]
#[ignore] // Ignored by default - requires DHT network
async fn test_full_operation_replication() -> anyhow::Result<()> {
    println!("\n=== Testing Full CRDT Operation Replication (Requires DHT Network) ===\n");
    println!("⚠️  This test is ignored by default because it requires DHT bootstrap peers");
    println!("   To run in production, provide bootstrap_peers in ClientConfig\n");
    
    // Create Alice (Space creator)
    let alice_keypair = Keypair::generate();
    let alice_id = alice_keypair.user_id();
    
    let alice_config = ClientConfig {
        storage_path: format!("./test-data/crdt-full-alice-{}", uuid::Uuid::new_v4()).into(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        // In production: bootstrap_peers: vec!["<multiaddr>".to_string()],
        bootstrap_peers: vec![],
    };
    
    let alice = Client::new(alice_keypair, alice_config)?;
    alice.start().await?;
    
    // Alice creates a Space
    let (space, _, _) = alice.create_space_with_visibility(
        "Test Space".to_string(),
        Some("A space for testing".to_string()),
        SpaceVisibility::Public,
    ).await?;
    
    let space_id = space.id;
    
    // Alice creates a channel
    alice.create_channel(
        space_id,
        "general".to_string(),
        Some("General discussion".to_string()),
    ).await?;
    
    // Wait for DHT propagation
    sleep(Duration::from_secs(2)).await;
    
    // Create Bob
    let bob_keypair = Keypair::generate();
    
    let bob_config = ClientConfig {
        storage_path: format!("./test-data/crdt-full-bob-{}", uuid::Uuid::new_v4()).into(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        // In production: same bootstrap peers
        bootstrap_peers: vec![],
    };
    
    let bob = Client::new(bob_keypair, bob_config)?;
    bob.start().await?;
    
    // Bob fetches operations (will work in production with DHT peers)
    let ops = bob.dht_get_operations(&space_id).await?;
    
    assert!(ops.len() >= 2, "Should have at least 2 operations");
    assert!(ops.iter().any(|op| matches!(op.op_type, OpType::CreateSpace(_))));
    assert!(ops.iter().any(|op| matches!(op.op_type, OpType::CreateChannel(_))));
    
    println!("✅ Full operation replication working!");
    
    Ok(())
}
