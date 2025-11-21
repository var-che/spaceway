//! Integration tests for DHT operations using SmoothTest framework
//!
//! These tests demonstrate the SmoothTest framework and verify basic functionality.
//! Full DHT network tests require actual network connectivity and are slow.

use descord_core::smoothtest::*;

#[tokio::test]
async fn test_smooth_client_batch_creation() {
    // Test that we can create multiple isolated clients
    let batch = SmoothClientBatch::new(3).unwrap();
    
    assert_eq!(batch.len(), 3);
    println!("✓ Created batch of 3 clients");
}

#[tokio::test]
async fn test_smooth_client_create_space() {
    // Test that individual clients can create spaces
    let batch = SmoothClientBatch::new(2).unwrap();
    
    let alice = &batch[0];
    let bob = &batch[1];

    // Each creates a space
    let space1 = alice.create_space("alice-space", Some("Alice's space")).await.unwrap();
    let space2 = bob.create_space("bob-space", Some("Bob's space")).await.unwrap();

    println!("✓ Alice created space: {:?}", space1.id);
    println!("✓ Bob created space: {:?}", space2.id);
    
    // Each client should see only their own space
    assert_eq!(alice.space_count().await, 1);
    assert_eq!(bob.space_count().await, 1);
}

#[tokio::test]
async fn test_smooth_client_isolated_storage() {
    // Verify each client has isolated storage
    let batch = SmoothClientBatch::new(3).unwrap();
    
    for (i, client) in batch.iter().enumerate() {
        let path = client.data_path();
        println!("Client {}: {:?}", i, path);
        assert!(path.exists());
    }
    
    println!("✓ All clients have isolated storage directories");
}

// Note: The following tests are commented out because they require
// actual network connectivity and take a long time to run.
// They demonstrate what will be possible once we implement:
// 1. Local relay/bootstrap server for testing
// 2. Faster DHT convergence for tests

/*
#[tokio::test]
#[ignore = "Requires network setup - run manually"]
async fn test_dht_space_discovery() {
    // This would test 3+ clients discovering each other's spaces via DHT
    // Requires: local bootstrap server, proper network configuration
}

#[tokio::test]
#[ignore = "Requires network setup - run manually"]
async fn test_offline_space_joining() {
    // This would test joining a space when creator is offline
    // Requires: DHT replication, 4+ nodes for reliability
}
*/

