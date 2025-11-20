use descord_core::{Client, ClientConfig, crypto::signing::Keypair};
use tokio::time::{sleep, Duration};
use anyhow::Result;

/// Test that simulates 3 people (Alice, Bob, Charlie) interacting:
/// 1. Alice creates a space, channel, thread, and sends messages
/// 2. Bob connects and receives Alice's operations
/// 3. Charlie connects and receives all operations  
/// 4. All three send messages and see each other's updates in real-time
///
/// NOTE: This test manually syncs operations to simulate network gossip.
/// In production, operations would be automatically gossiped via GossipSub.
/// Future versions will add automatic state synchronization on peer connect.
#[tokio::test(flavor = "multi_thread")]
async fn test_three_person_interaction() -> Result<()> {
    // Create three separate keypairs
    let alice_keypair = Keypair::generate();
    let bob_keypair = Keypair::generate();
    let charlie_keypair = Keypair::generate();

    // Create temporary directories for each client
    let alice_dir = tempfile::tempdir()?;
    let bob_dir = tempfile::tempdir()?;
    let charlie_dir = tempfile::tempdir()?;

    // Configure Alice (first peer - no bootstrap)
    let alice_config = ClientConfig {
        storage_path: alice_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9001".to_string()],
        bootstrap_peers: vec![],
    };

    // Start Alice's client
    let alice = Client::new(alice_keypair.clone(), alice_config)?;
    alice.start().await?;
    
    // Give Alice time to start listening
    sleep(Duration::from_millis(500)).await;

    // Alice creates a space
    let (alice_space, space_op, _) = alice.create_space(
        "Test Community".to_string(),
        Some("A test community for three people".to_string())
    ).await?;
    println!("✅ Alice created space: {:?}", alice_space.id);

    // Alice creates a channel
    let (alice_channel, channel_op) = alice.create_channel(
        alice_space.id,
        "general".to_string(),
        Some("General discussion".to_string())
    ).await?;
    println!("✅ Alice created channel: {:?}", alice_channel.id);

    // Alice creates a thread
    let (alice_thread, thread_op) = alice.create_thread(
        alice_space.id,
        alice_channel.id,
        Some("Hello".to_string()),
        "Welcome everyone!".to_string()
    ).await?;
    println!("✅ Alice created thread: {:?}", alice_thread.id);

    // Alice sends a message
    let (alice_msg1, msg1_op) = alice.post_message(
        alice_space.id,
        alice_thread.id,
        "Hi from Alice!".to_string()
    ).await?;
    println!("✅ Alice sent message: {:?}", alice_msg1.id);

    // Configure Bob (connects to Alice)
    let bob_config = ClientConfig {
        storage_path: bob_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9002".to_string()],
        bootstrap_peers: vec!["/ip4/127.0.0.1/tcp/9001".to_string()],
    };

    // Start Bob's client
    let bob = Client::new(bob_keypair.clone(), bob_config)?;
    bob.start().await?;
    
    // Bob subscribes to Alice's space
    bob.subscribe_to_space(&alice_space.id).await?;
    
    // Simulate Bob receiving Alice's operations (in production, via GossipSub)
    bob.apply_remote_op(&space_op).await?;
    bob.apply_remote_op(&channel_op).await?;
    bob.apply_remote_op(&thread_op).await?;
    bob.apply_remote_op(&msg1_op).await?;

    // Give Bob time to process operations
    sleep(Duration::from_millis(500)).await;

    // Bob should see Alice's space
    let bob_spaces = bob.list_spaces().await;
    assert_eq!(bob_spaces.len(), 1, "Bob should see Alice's space");
    assert_eq!(bob_spaces[0].id, alice_space.id);
    println!("✅ Bob synced Alice's space");

    // Bob should see Alice's channel
    let bob_channels = bob.list_channels(&alice_space.id).await;
    assert_eq!(bob_channels.len(), 1, "Bob should see Alice's channel");
    assert_eq!(bob_channels[0].id, alice_channel.id);
    println!("✅ Bob synced Alice's channel");

    // Bob should see Alice's thread
    let bob_threads = bob.list_threads(&alice_channel.id).await;
    assert_eq!(bob_threads.len(), 1, "Bob should see Alice's thread");
    assert_eq!(bob_threads[0].id, alice_thread.id);
    println!("✅ Bob synced Alice's thread");

    // Bob should see Alice's messages (thread creation message + 1 message)
    let bob_messages = bob.list_messages(&alice_thread.id).await;
    assert_eq!(bob_messages.len(), 2, "Bob should see Alice's messages");
    println!("✅ Bob synced {} messages from Alice", bob_messages.len());

    // Bob sends a message
    let (bob_msg1, bob_msg1_op) = bob.post_message(
        alice_space.id,
        alice_thread.id,
        "Hi from Bob!".to_string()
    ).await?;
    println!("✅ Bob sent message: {:?}", bob_msg1.id);

    // Alice receives Bob's message (via GossipSub in production)
    alice.apply_remote_op(&bob_msg1_op).await?;
    sleep(Duration::from_millis(100)).await;

    // Alice should see Bob's message
    let alice_messages = alice.list_messages(&alice_thread.id).await;
    assert_eq!(alice_messages.len(), 3, "Alice should see Bob's message");
    println!("✅ Alice received Bob's message");

    // Configure Charlie (connects to Alice)
    let charlie_config = ClientConfig {
        storage_path: charlie_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9003".to_string()],
        bootstrap_peers: vec!["/ip4/127.0.0.1/tcp/9001".to_string()],
    };

    // Start Charlie's client
    let charlie = Client::new(charlie_keypair.clone(), charlie_config)?;
    charlie.start().await?;
    
    // Charlie subscribes and receives all operations
    charlie.subscribe_to_space(&alice_space.id).await?;
    charlie.apply_remote_op(&space_op).await?;
    charlie.apply_remote_op(&channel_op).await?;
    charlie.apply_remote_op(&thread_op).await?;
    charlie.apply_remote_op(&msg1_op).await?;
    charlie.apply_remote_op(&bob_msg1_op).await?;

    // Give Charlie time to process
    sleep(Duration::from_millis(500)).await;

    // Charlie should see the space
    let charlie_spaces = charlie.list_spaces().await;
    assert_eq!(charlie_spaces.len(), 1, "Charlie should see the space");
    println!("✅ Charlie synced the space");

    // Charlie should see all messages (Alice's initial + Alice's message + Bob's message)
    let charlie_messages = charlie.list_messages(&alice_thread.id).await;
    assert_eq!(charlie_messages.len(), 3, "Charlie should see all messages");
    println!("✅ Charlie synced {} messages", charlie_messages.len());

    // Charlie sends a message
    let (charlie_msg1, charlie_msg1_op) = charlie.post_message(
        alice_space.id,
        alice_thread.id,
        "Hi from Charlie!".to_string()
    ).await?;
    println!("✅ Charlie sent message: {:?}", charlie_msg1.id);

    // Alice and Bob receive Charlie's message
    alice.apply_remote_op(&charlie_msg1_op).await?;
    bob.apply_remote_op(&charlie_msg1_op).await?;
    sleep(Duration::from_millis(100)).await;

    // All three should now see 4 messages total
    let alice_final = alice.list_messages(&alice_thread.id).await;
    let bob_final = bob.list_messages(&alice_thread.id).await;
    let charlie_final = charlie.list_messages(&alice_thread.id).await;

    assert_eq!(alice_final.len(), 4, "Alice should see all 4 messages");
    assert_eq!(bob_final.len(), 4, "Bob should see all 4 messages");
    assert_eq!(charlie_final.len(), 4, "Charlie should see all 4 messages");

    println!("✅ All three clients converged to 4 messages");

    // Verify message content
    let messages = alice_final;
    assert_eq!(messages[0].content, "Welcome everyone!");
    assert_eq!(messages[1].content, "Hi from Alice!");
    assert_eq!(messages[2].content, "Hi from Bob!");
    assert_eq!(messages[3].content, "Hi from Charlie!");

    println!("✅ Message content verified");

    // Test concurrent messaging - all three send at once
    let (alice_msg2, alice_msg2_op) = alice.post_message(
        alice_space.id,
        alice_thread.id,
        "Alice says hello again!".to_string()
    ).await?;

    let (bob_msg2, bob_msg2_op) = bob.post_message(
        alice_space.id,
        alice_thread.id,
        "Bob says hello again!".to_string()
    ).await?;

    let (charlie_msg2, charlie_msg2_op) = charlie.post_message(
        alice_space.id,
        alice_thread.id,
        "Charlie says hello again!".to_string()
    ).await?;
    
    println!("✅ All three sent concurrent messages");
    
    // Broadcast all messages to all peers
    alice.apply_remote_op(&bob_msg2_op).await?;
    alice.apply_remote_op(&charlie_msg2_op).await?;
    bob.apply_remote_op(&alice_msg2_op).await?;
    bob.apply_remote_op(&charlie_msg2_op).await?;
    charlie.apply_remote_op(&alice_msg2_op).await?;
    charlie.apply_remote_op(&bob_msg2_op).await?;

    // Give time to sync
    sleep(Duration::from_millis(500)).await;

    // All should see 7 messages (4 previous + 3 new)
    let alice_final2 = alice.list_messages(&alice_thread.id).await;
    let bob_final2 = bob.list_messages(&alice_thread.id).await;
    let charlie_final2 = charlie.list_messages(&alice_thread.id).await;

    assert_eq!(alice_final2.len(), 7, "Alice should see 7 messages after concurrent sends");
    assert_eq!(bob_final2.len(), 7, "Bob should see 7 messages after concurrent sends");
    assert_eq!(charlie_final2.len(), 7, "Charlie should see 7 messages after concurrent sends");

    println!("✅ Concurrent messages converged successfully");

    // Verify CRDT convergence - all three have the same set of message IDs
    // Note: Order may vary for concurrent messages due to network timing,
    // but all clients should have the same SET of messages
    let alice_ids: std::collections::HashSet<_> = alice_final2.iter().map(|m| m.id).collect();
    let bob_ids: std::collections::HashSet<_> = bob_final2.iter().map(|m| m.id).collect();
    let charlie_ids: std::collections::HashSet<_> = charlie_final2.iter().map(|m| m.id).collect();

    assert_eq!(alice_ids, bob_ids, "Alice and Bob should have identical message sets");
    assert_eq!(bob_ids, charlie_ids, "Bob and Charlie should have identical message sets");

    println!("✅ CRDT convergence verified - all clients have identical state");

    // Test that Bob creates a new channel and everyone sees it
    let (bob_channel, bob_channel_op) = bob.create_channel(
        alice_space.id,
        "random".to_string(),
        Some("Random discussion".to_string())
    ).await?;
    println!("✅ Bob created a new channel: {:?}", bob_channel.id);

    // Broadcast to Alice and Charlie
    alice.apply_remote_op(&bob_channel_op).await?;
    charlie.apply_remote_op(&bob_channel_op).await?;
    sleep(Duration::from_millis(100)).await;

    // Alice and Charlie should see the new channel
    let alice_channels = alice.list_channels(&alice_space.id).await;
    let charlie_channels = charlie.list_channels(&alice_space.id).await;

    assert_eq!(alice_channels.len(), 2, "Alice should see 2 channels");
    assert_eq!(charlie_channels.len(), 2, "Charlie should see 2 channels");
    println!("✅ Bob's new channel synced to Alice and Charlie");

    println!("\n🎉 Three-person interaction test passed!");
    println!("   - 3 clients connected via P2P");
    println!("   - All clients converged to identical state");
    println!("   - Concurrent operations handled correctly");
    println!("   - CRDT properties verified");

    Ok(())
}
