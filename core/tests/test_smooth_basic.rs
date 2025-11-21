//! Basic SmoothTest framework tests with debug output and timeouts

use descord_core::smoothtest::*;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn test_01_smooth_client_creation() {
    println!("TEST 1: Creating single SmoothClient...");
    
    let client = SmoothClient::new().unwrap();
    println!("✓ Client created successfully");
    
    let count = client.space_count().await;
    println!("✓ Space count: {}", count);
    
    assert_eq!(count, 0);
    println!("✓ TEST 1 PASSED");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_02_smooth_batch_creation() {
    println!("\nTEST 2: Creating SmoothClientBatch...");
    
    let batch = SmoothClientBatch::new(3).unwrap();
    println!("✓ Batch created with {} clients", batch.len());
    
    assert_eq!(batch.len(), 3);
    println!("✓ TEST 2 PASSED");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_03_create_space_with_timeout() {
    println!("\nTEST 3: Creating space with timeout...");
    
    let client = SmoothClient::new().unwrap();
    println!("✓ Client created");
    
    // Use timeout to prevent hanging
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        client.create_space("test-space", Some("Test"))
    ).await;
    
    match result {
        Ok(Ok(space)) => {
            println!("✓ Space created: {:?}", space.id);
            println!("✓ TEST 3 PASSED");
        }
        Ok(Err(e)) => {
            println!("✗ Space creation failed: {}", e);
            panic!("Space creation error: {}", e);
        }
        Err(_) => {
            println!("✗ TIMEOUT: Space creation took longer than 5 seconds");
            panic!("Test timed out - likely hanging on network operation");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_04_multiple_clients_create_spaces() {
    println!("\nTEST 4: Multiple clients creating spaces...");
    
    let batch = SmoothClientBatch::new(2).unwrap();
    println!("✓ Batch created");
    
    let alice = &batch[0];
    let bob = &batch[1];
    
    // Create Alice's space with timeout
    println!("Creating Alice's space...");
    let alice_result = tokio::time::timeout(
        Duration::from_secs(5),
        alice.create_space("alice-space", Some("Alice"))
    ).await;
    
    match alice_result {
        Ok(Ok(space)) => println!("✓ Alice's space created: {:?}", space.id),
        Ok(Err(e)) => panic!("Alice space creation failed: {}", e),
        Err(_) => panic!("Alice space creation timed out"),
    }
    
    // Create Bob's space with timeout
    println!("Creating Bob's space...");
    let bob_result = tokio::time::timeout(
        Duration::from_secs(5),
        bob.create_space("bob-space", Some("Bob"))
    ).await;
    
    match bob_result {
        Ok(Ok(space)) => println!("✓ Bob's space created: {:?}", space.id),
        Ok(Err(e)) => panic!("Bob space creation failed: {}", e),
        Err(_) => panic!("Bob space creation timed out"),
    }
    
    // Check space counts
    println!("Checking space counts...");
    let alice_count = alice.space_count().await;
    let bob_count = bob.space_count().await;
    
    println!("Alice has {} spaces", alice_count);
    println!("Bob has {} spaces", bob_count);
    
    assert_eq!(alice_count, 1);
    assert_eq!(bob_count, 1);
    
    println!("✓ TEST 4 PASSED");
}
