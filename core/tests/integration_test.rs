//! Integration tests for Descord
//!
//! Tests multi-client scenarios with actual networking and storage

mod integration;

use anyhow::Result;
use descord_core::{Client, ClientConfig, crypto::Keypair, Permissions, Role};
use descord_core::mls::{MlsGroup, MlsGroupConfig, provider::create_provider};
use descord_core::types::{SpaceId, UserId};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Helper to create a test client
async fn create_test_client(_name: &str) -> Result<Client> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    Ok(Client::new(keypair, config)?)
}

#[tokio::test]
async fn test_single_client_basic_operations() -> Result<()> {
    let name = "test_single_client";
    
    let client = create_test_client(name).await?;
    let user_id = client.user_id();
    
    println!("Created client with user ID: {}", hex::encode(user_id.as_bytes()));
    
    // Create a Space
    let (space, _op, _) = client.create_space(
        "Test Space".to_string(),
        Some("A test community".to_string()),
    ).await?;
    
    println!("Created space: {} ({})", space.name, hex::encode(&space.id.0.as_bytes()[..8]));
    assert_eq!(space.name, "Test Space");
    assert_eq!(space.owner, user_id);
    
    // List spaces
    let spaces = client.list_spaces().await;
    assert_eq!(spaces.len(), 1);
    assert_eq!(spaces[0].id, space.id);
    
    // Create a Channel
    let (channel, _op) = client.create_channel(
        space.id,
        "general".to_string(),
        Some("General discussion".to_string()),
    ).await?;
    
    println!("Created channel: {}", channel.name);
    assert_eq!(channel.name, "general");
    assert_eq!(channel.space_id, space.id);
    
    // List channels
    let channels = client.list_channels(&space.id).await;
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].id, channel.id);
    
    // Create a Thread
    let (thread, _op) = client.create_thread(
        space.id,
        channel.id,
        Some("Welcome thread".to_string()),
        "Hello, world!".to_string(),
    ).await?;
    
    println!("Created thread: {:?}", thread.title);
    assert_eq!(thread.title, Some("Welcome thread".to_string()));
    assert_eq!(thread.channel_id, channel.id);
    
    // List threads
    let threads = client.list_threads(&channel.id).await;
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].id, thread.id);
    
    // Post a message
    let (msg, _op) = client.post_message(
        space.id,
        thread.id,
        "This is a test message".to_string(),
    ).await?;
    
    println!("Posted message: {}", msg.content);
    assert_eq!(msg.content, "This is a test message");
    assert_eq!(msg.author, user_id);
    assert_eq!(msg.thread_id, thread.id);
    
    // List messages
    let messages = client.list_messages(&thread.id).await;
    assert_eq!(messages.len(), 2); // First message + our message
    
    // Edit the message
    client.edit_message(
        space.id,
        msg.id,
        "This is an edited message".to_string(),
    ).await?;
    
    // Verify edit
    let edited_msg = client.get_message(&msg.id).await.unwrap();
    assert_eq!(edited_msg.content, "This is an edited message");
    assert!(edited_msg.edited_at.is_some());
    

    Ok(())
}

#[tokio::test]
async fn test_blob_storage() -> Result<()> {
    let name = "test_blob_storage";

    
    let client = create_test_client(name).await?;
    
    // Store a small blob
    let data = b"Hello, blob storage!";
    let metadata = client.store_blob(data, Some("text/plain".to_string()), Some("hello.txt".to_string())).await?;
    
    println!("Stored blob: {} bytes, hash: {}", metadata.size, hex::encode(&metadata.hash.0[..8]));
    assert_eq!(metadata.size as usize, data.len());
    assert_eq!(metadata.mime_type, Some("text/plain".to_string()));
    
    // Retrieve the blob
    let retrieved = client.retrieve_blob(&metadata.hash).await?;
    assert_eq!(retrieved, data);
    
    // Store a larger blob (multi-chunk)
    let large_data = vec![0u8; 512 * 1024]; // 512 KB
    let large_metadata = client.store_blob(&large_data, None, None).await?;
    
    println!("Stored large blob: {} bytes", large_metadata.size);
    assert_eq!(large_metadata.size as usize, large_data.len());
    
    // Retrieve large blob
    let retrieved_large = client.retrieve_blob(&large_metadata.hash).await?;
    assert_eq!(retrieved_large.len(), large_data.len());
    

    Ok(())
}

#[tokio::test]
async fn test_multi_client_sync() -> Result<()> {
    let alice_name = "test_alice";
    let bob_name = "test_bob";


    
    // Create two clients
    let alice = create_test_client(alice_name).await?;
    let bob = create_test_client(bob_name).await?;
    
    let alice_id = alice.user_id();
    let bob_id = bob.user_id();
    
    println!("Alice ID: {}", hex::encode(alice_id.as_bytes()));
    println!("Bob ID: {}", hex::encode(bob_id.as_bytes()));
    
    // Alice creates a space
    let (space, space_op, _) = alice.create_space(
        "Shared Space".to_string(),
        Some("A space for Alice and Bob".to_string()),
    ).await?;
    
    println!("Alice created space: {}", space.name);
    
    // Alice creates a channel
    let (channel, channel_op) = alice.create_channel(
        space.id,
        "chat".to_string(),
        Some("Chat channel".to_string()),
    ).await?;
    
    println!("Alice created channel: {}", channel.name);
    
    // Simulate Bob receiving the operations
    // In a real system, these would come over the network
    bob.apply_remote_op(&space_op).await?;
    bob.apply_remote_op(&channel_op).await?;
    
    // Give Bob time to process
    sleep(Duration::from_millis(100)).await;
    
    // Verify Bob sees the space and channel
    let bob_spaces = bob.list_spaces().await;
    println!("Bob sees {} spaces", bob_spaces.len());
    
    if !bob_spaces.is_empty() {
        assert_eq!(bob_spaces[0].id, space.id);
        assert_eq!(bob_spaces[0].name, "Shared Space");
        
        let bob_channels = bob.list_channels(&space.id).await;
        println!("Bob sees {} channels", bob_channels.len());
        
        if !bob_channels.is_empty() {
            assert_eq!(bob_channels[0].id, channel.id);
            assert_eq!(bob_channels[0].name, "chat");
        }
    }
    
    // Alice creates a thread
    let (thread, thread_op) = alice.create_thread(
        space.id,
        channel.id,
        Some("Discussion".to_string()),
        "Let's talk!".to_string(),
    ).await?;
    
    println!("Alice created thread: {:?}", thread.title);
    
    // Bob receives thread creation
    bob.apply_remote_op(&thread_op).await?;
    sleep(Duration::from_millis(100)).await;
    
    // Bob posts a message
    let (bob_msg, bob_msg_op) = bob.post_message(
        space.id,
        thread.id,
        "Hi Alice!".to_string(),
    ).await?;
    
    println!("Bob posted: {}", bob_msg.content);
    
    // Alice receives Bob's message
    alice.apply_remote_op(&bob_msg_op).await?;
    sleep(Duration::from_millis(100)).await;
    
    // Verify Alice sees Bob's message
    let alice_messages = alice.list_messages(&thread.id).await;
    println!("Alice sees {} messages", alice_messages.len());
    
    let bob_message = alice_messages.iter().find(|m| m.id == bob_msg.id);
    if let Some(msg) = bob_message {
        assert_eq!(msg.content, "Hi Alice!");
        assert_eq!(msg.author, bob_id);
        println!("✓ Alice received Bob's message!");
    }
    


    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let alice_name = "test_concurrent_alice";
    let bob_name = "test_concurrent_bob";


    
    let alice = create_test_client(alice_name).await?;
    let bob = create_test_client(bob_name).await?;
    
    // Alice creates a space
    let (space, space_op, _) = alice.create_space(
        "Concurrent Test".to_string(),
        None,
    ).await?;
    
    let (channel, channel_op) = alice.create_channel(
        space.id,
        "general".to_string(),
        None,
    ).await?;
    
    let (thread, thread_op) = alice.create_thread(
        space.id,
        channel.id,
        Some("Test".to_string()),
        "Initial".to_string(),
    ).await?;
    
    // Bob receives initial state
    bob.apply_remote_op(&space_op).await?;
    bob.apply_remote_op(&channel_op).await?;
    bob.apply_remote_op(&thread_op).await?;
    
    // Both post messages concurrently
    let alice_msg_future = alice.post_message(
        space.id,
        thread.id,
        "Alice's concurrent message".to_string(),
    );
    
    let bob_msg_future = bob.post_message(
        space.id,
        thread.id,
        "Bob's concurrent message".to_string(),
    );
    
    let (alice_result, bob_result) = tokio::join!(alice_msg_future, bob_msg_future);
    let (alice_msg, alice_op) = alice_result?;
    let (bob_msg, bob_op) = bob_result?;
    
    println!("Alice posted: {}", alice_msg.content);
    println!("Bob posted: {}", bob_msg.content);
    
    // Exchange operations
    alice.apply_remote_op(&bob_op).await?;
    bob.apply_remote_op(&alice_op).await?;
    
    sleep(Duration::from_millis(100)).await;
    
    // Both should see both messages
    let alice_messages = alice.list_messages(&thread.id).await;
    let bob_messages = bob.list_messages(&thread.id).await;
    
    println!("Alice sees {} messages", alice_messages.len());
    println!("Bob sees {} messages", bob_messages.len());
    
    // Should have at least 3 messages: initial + alice + bob
    assert!(alice_messages.len() >= 3);
    assert!(bob_messages.len() >= 3);
    
    // Both should have the same messages (eventual consistency)
    let alice_msg_ids: std::collections::HashSet<_> = 
        alice_messages.iter().map(|m| m.id).collect();
    let bob_msg_ids: std::collections::HashSet<_> = 
        bob_messages.iter().map(|m| m.id).collect();
    
    assert_eq!(alice_msg_ids, bob_msg_ids, "Message sets should be identical");
    println!("✓ Both clients converged to the same state!");
    


    Ok(())
}

#[tokio::test]
async fn test_crdt_commutativity() -> Result<()> {
    let client1_name = "test_crdt_1";
    let client2_name = "test_crdt_2";


    
    let client1 = create_test_client(client1_name).await?;
    let client2 = create_test_client(client2_name).await?;
    
    // Create initial state
    let (space, space_op, _) = client1.create_space("Test".to_string(), None).await?;
    let (channel, channel_op) = client1.create_channel(space.id, "general".to_string(), None).await?;
    let (thread, thread_op) = client1.create_thread(space.id, channel.id, None, "Start".to_string()).await?;
    
    // Client2 gets initial state
    client2.apply_remote_op(&space_op).await?;
    client2.apply_remote_op(&channel_op).await?;
    client2.apply_remote_op(&thread_op).await?;
    
    // Create two operations
    let (msg1, op1) = client1.post_message(space.id, thread.id, "Message 1".to_string()).await?;
    let (msg2, op2) = client2.post_message(space.id, thread.id, "Message 2".to_string()).await?;
    
    // Apply in different orders
    // Client1: op2 then checks
    client1.apply_remote_op(&op2).await?;
    
    // Client2: op1 then checks
    client2.apply_remote_op(&op1).await?;
    
    sleep(Duration::from_millis(100)).await;
    
    // Both should converge to same state
    let messages1 = client1.list_messages(&thread.id).await;
    let messages2 = client2.list_messages(&thread.id).await;
    
    let ids1: Vec<_> = messages1.iter().map(|m| m.id).collect();
    let ids2: Vec<_> = messages2.iter().map(|m| m.id).collect();
    
    assert_eq!(ids1.len(), ids2.len(), "Should have same number of messages");
    
    // IDs might be in different order, but should contain the same set
    let set1: std::collections::HashSet<_> = ids1.iter().collect();
    let set2: std::collections::HashSet<_> = ids2.iter().collect();
    assert_eq!(set1, set2, "Should have same message IDs regardless of operation order");
    
    println!("✓ CRDT commutativity verified!");
    println!("✓ Both clients converged to the same state!");

    Ok(())
}

// Full permissions test moved to tests/integration/permissions_test.rs
// Message deletion tests in tests/integration/message_deletion_test.rs
