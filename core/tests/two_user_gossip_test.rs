//! Integration test: Two users connecting via GossipSub and exchanging messages
//!
//! This test simulates the real-world scenario:
//! 1. Alice starts and creates a Space with messages
//! 2. Bob connects to Alice
//! 3. Bob joins the Space with an invite
//! 4. Bob should see Alice's messages
//! 5. Both can exchange messages

use descord_core::{Client, ClientConfig};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_two_users_gossipsub_sync() {
    // Clean up test data
    let _ = std::fs::remove_dir_all("test-alice-data");
    let _ = std::fs::remove_dir_all("test-bob-data");

    println!("\n=== Starting Two-User GossipSub Integration Test ===\n");

    // Step 1: Create Alice (listening on port 9876)
    println!("📝 Step 1: Alice creates account and starts listening...");
    let alice_keypair = descord_core::crypto::signing::Keypair::generate();
    let alice_config = ClientConfig {
        storage_path: PathBuf::from("test-alice-data"),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/9876".to_string()],
        bootstrap_peers: vec![],
    };
    let alice = Client::new(alice_keypair.clone(), alice_config).unwrap();
    alice.start().await.unwrap();
    
    sleep(Duration::from_secs(1)).await;
    
    let alice_peer_id = alice.peer_id().await;
    let alice_addrs = alice.listening_addrs().await;
    println!("✓ Alice started");
    println!("  Peer ID: {}", alice_peer_id);
    println!("  Listening on: {:?}", alice_addrs);

    // Step 2: Alice creates Space, Channel, Thread, Messages
    println!("\n📝 Step 2: Alice creates Space, Channel, Thread, and Messages...");
    let (space, _, _) = alice.create_space(
        "TechCommunity".to_string(),
        Some("A decentralized tech forum".to_string())
    ).await.unwrap();
    println!("✓ Alice created Space: {} (ID: {})", space.name, hex::encode(&space.id.0[..8]));

    let (channel, _) = alice.create_channel(
        space.id,
        "general".to_string(),
        Some("General discussion".to_string())
    ).await.unwrap();
    println!("✓ Alice created Channel: {}", channel.name);

    let (thread, _) = alice.create_thread(
        space.id,
        channel.id,
        Some("Welcome".to_string()),
        "Hello everyone!".to_string()
    ).await.unwrap();
    println!("✓ Alice created Thread: {:?}", thread.title);

    // Send more messages
    let (msg1, _) = alice.post_message(
        space.id,
        thread.id,
        "Welcome to our decentralized forum!".to_string()
    ).await.unwrap();
    println!("✓ Alice sent message 1: {}", msg1.content);

    let (msg2, _) = alice.post_message(
        space.id,
        thread.id,
        "Feel free to introduce yourself.".to_string()
    ).await.unwrap();
    println!("✓ Alice sent message 2: {}", msg2.content);

    // Step 3: Alice creates an invite
    println!("\n📝 Step 3: Alice creates invite code...");
    let invite_op = alice.create_invite(space.id, None, None).await.unwrap();
    
    // Get the actual invite from the list
    let invites = alice.list_invites(&space.id).await;
    let invite = invites.last().unwrap();
    println!("✓ Alice created invite code: {}", invite.code);
    println!("  Space ID: {}", hex::encode(&space.id.0));

    // Give GossipSub time to settle
    sleep(Duration::from_secs(1)).await;

    // Step 4: Create Bob
    println!("\n📝 Step 4: Bob creates account and starts...");
    let bob_keypair = descord_core::crypto::signing::Keypair::generate();
    let bob_config = ClientConfig {
        storage_path: PathBuf::from("test-bob-data"),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    let bob = Client::new(bob_keypair.clone(), bob_config).unwrap();
    bob.start().await.unwrap();
    
    sleep(Duration::from_secs(1)).await;
    
    let bob_peer_id = bob.peer_id().await;
    println!("✓ Bob started");
    println!("  Peer ID: {}", bob_peer_id);

    // Step 5: Bob connects to Alice
    println!("\n📝 Step 5: Bob connects to Alice...");
    let alice_multiaddr = format!("/ip4/127.0.0.1/tcp/9876/p2p/{}", alice_peer_id);
    println!("  Connecting to: {}", alice_multiaddr);
    
    bob.network_dial(&alice_multiaddr).await.unwrap();
    println!("✓ Bob initiated connection to Alice");

    // Wait for connection to establish
    sleep(Duration::from_secs(2)).await;
    println!("✓ Waiting for peer connection to establish...");

    // Step 6: Bob joins the Space with invite
    println!("\n📝 Step 6: Bob joins Space with invite code...");
    println!("  Space ID: {}", hex::encode(&space.id.0));
    println!("  Invite code: {}", invite.code);

    // Try to join
    match bob.join_with_invite(space.id, invite.code.clone()).await {
        Ok(_) => {
            println!("✓ Bob successfully used invite code");
        }
        Err(e) => {
            println!("✗ Bob failed to join: {}", e);
            println!("\n⚠️  This is the bug we're debugging!");
            println!("  Bob is connected but didn't receive Alice's Space operations via GossipSub");
            println!("  This means GossipSub only forwards NEW messages, not historical ones");
        }
    }

    // Step 7: Check if Bob can see the Space
    println!("\n📝 Step 7: Checking Bob's local state...");
    let bob_spaces = bob.list_spaces().await;
    println!("  Bob sees {} spaces locally", bob_spaces.len());
    for s in &bob_spaces {
        println!("    - {} (ID: {})", s.name, hex::encode(&s.id.0[..8]));
    }

    if bob_spaces.is_empty() {
        println!("\n❌ BUG CONFIRMED: Bob didn't receive Space data from Alice");
        println!("   Even though they're connected, GossipSub didn't backfill historical operations");
    }

    // Step 8: Test if NEW operations sync
    println!("\n📝 Step 8: Testing if NEW operations sync after connection...");
    println!("  Alice will send a new message now that Bob is connected and subscribed");
    
    // Wait longer to ensure Bob's subscription is active and mesh has formed
    println!("  Waiting 3 seconds for GossipSub mesh to form...");
    sleep(Duration::from_secs(3)).await;
    
    let (msg3, _) = alice.post_message(
        space.id,
        thread.id,
        "Bob, can you see this NEW message?".to_string()
    ).await.unwrap();
    println!("✓ Alice sent NEW message: {}", msg3.content);

    // Wait for GossipSub propagation and event processing
    println!("  Waiting 5 seconds for message propagation and processing...");
    sleep(Duration::from_secs(5)).await;

    // Check if Bob received it
    let bob_spaces_after = bob.list_spaces().await;
    println!("  Bob now sees {} spaces", bob_spaces_after.len());

    if bob_spaces_after.len() > bob_spaces.len() {
        println!("✓ SUCCESS: Bob received the NEW operation via GossipSub!");
    } else {
        println!("✗ FAILED: Bob didn't receive even NEW operations");
        println!("   This suggests Bob isn't subscribed to the Space topic properly");
    }

    // Step 9: List messages in Alice's thread
    println!("\n📝 Step 9: Alice's messages in thread:");
    let alice_messages = alice.list_messages(&thread.id).await;
    for msg in &alice_messages {
        println!("  [{}] {}", hex::encode(&msg.id.0[..4]), msg.content);
    }
    println!("  Total: {} messages", alice_messages.len());

    // Step 10: Try Bob's view
    if !bob_spaces_after.is_empty() {
        println!("\n📝 Step 10: Bob's view of messages:");
        let bob_messages = bob.list_messages(&thread.id).await;
        println!("  Bob sees {} messages", bob_messages.len());
        for msg in &bob_messages {
            println!("  [{}] {}", hex::encode(&msg.id.0[..4]), msg.content);
        }

        if bob_messages.len() == alice_messages.len() {
            println!("\n✅ PERFECT SYNC: Bob sees all of Alice's messages!");
        } else {
            println!("\n⚠️  PARTIAL SYNC: Bob sees {}/{} messages", 
                bob_messages.len(), alice_messages.len());
        }
    }

    println!("\n=== Test Complete ===\n");

    // Cleanup
    let _ = std::fs::remove_dir_all("test-alice-data");
    let _ = std::fs::remove_dir_all("test-bob-data");
}

#[tokio::test]
async fn test_sync_with_request_response() {
    println!("\n=== Testing Space Sync with Request/Response ===\n");
    
    // This test will verify if we can request operations from peers
    // when we join a Space topic
    
    println!("📝 This test will be implemented when we add sync request feature");
    println!("   The feature should:");
    println!("   1. Bob subscribes to Space topic");
    println!("   2. Bob sends a 'sync request' to all peers on that topic");
    println!("   3. Alice responds with all Space operations");
    println!("   4. Bob rebuilds Space state from operations");
    println!("   5. Bob can then use the invite code");
}
