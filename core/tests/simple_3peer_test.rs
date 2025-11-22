//! Simple Local 3-Peer Integration Test
//!
//! This test programmatically:
//! - Starts 3 peers (Alice, Bob, Charlie) on localhost
//! - Connects them to each other
//! - Has Alice create a space, channel, and thread
//! - Has Bob and Charlie join the space
//! - Tests message exchange
//! - All automated - no manual terminal work!
//!
//! Run with: cargo +nightly test --package spaceway-core --test simple_3peer_test -- --nocapture

use spaceway_core::{Client, ClientConfig};
use spaceway_core::crypto::Keypair;
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;

/// Helper to create a user with a temporary data directory
async fn create_test_user(name: &str, port: u16) -> (Client, String) {
    println!("🔧 Setting up {}...", name);
    
    let data_dir = TempDir::new().expect("Failed to create temp dir");
    let keypair = Keypair::generate();
    
    let config = ClientConfig {
        storage_path: data_dir.path().to_path_buf(),
        listen_addrs: vec![format!("/ip4/0.0.0.0/tcp/{}", port)],
        bootstrap_peers: vec![], // Will connect manually
    };
    
    let client = Client::new(keypair, config)
        .expect(&format!("Failed to create {} client", name));
    
    let user_id = client.user_id();
    let peer_id = client.local_peer_id()
        .expect("Failed to get peer ID")
        .to_string();
    
    println!("  ✓ {} initialized", name);
    println!("    User ID: {}", hex::encode(&user_id.as_bytes()[..8]));
    println!("    Peer ID: {}", peer_id);
    println!("    Listening on: 127.0.0.1:{}", port);
    
    (client, peer_id)
}

#[tokio::test]
#[ignore] // Run with: cargo test --package spaceway-core --test simple_3peer_test -- --ignored --nocapture
async fn simple_local_3peer_test() {
    println!("\n");
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║         SIMPLE LOCAL 3-PEER INTEGRATION TEST                      ║");
    println!("║                                                                   ║");
    println!("║  Tests Alice, Bob, and Charlie connecting and messaging          ║");
    println!("║  All on localhost - no relay server required!                    ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    // ========== STEP 1: Create All Peers ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 1: Initialize All Peers                                     ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    let (alice, alice_peer_id) = create_test_user("Alice", 9001).await;
    let (bob, bob_peer_id) = create_test_user("Bob", 9002).await;
    let (charlie, charlie_peer_id) = create_test_user("Charlie", 9003).await;
    
    println!();
    println!("✅ All 3 peers initialized");
    println!();
    
    // Give them a moment to start listening
    sleep(Duration::from_secs(2)).await;
    
    // ========== STEP 2: Connect Peers ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 2: Connect Peers to Each Other                              ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    // Alice's multiaddr
    let alice_addr = format!("/ip4/127.0.0.1/tcp/9001/p2p/{}", alice_peer_id);
    
    println!("🔗 Bob connecting to Alice...");
    bob.connect_to_peer(&alice_addr).await
        .expect("Bob failed to connect to Alice");
    println!("  ✓ Bob → Alice connected");
    
    println!("🔗 Charlie connecting to Alice...");
    charlie.connect_to_peer(&alice_addr).await
        .expect("Charlie failed to connect to Alice");
    println!("  ✓ Charlie → Alice connected");
    
    println!();
    println!("✅ All peers connected (via Alice as hub)");
    println!();
    
    // Let peer discovery and GossipSub mesh form
    println!("⏳ Waiting 5 seconds for peer discovery and mesh formation...");
    sleep(Duration::from_secs(5)).await;
    println!();
    
    // ========== STEP 3: Create Space ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 3: Alice Creates Space                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("👤 Alice creating 'TestSpace'...");
    let (space, _op, _privacy) = alice.create_space("TestSpace".to_string(), Some("Test description".to_string())).await
        .expect("Alice failed to create space");
    let space_id = space.id;
    println!("  ✓ Space created: {}", hex::encode(&space_id.0[..8]));
    println!();
    
    // Give time for broadcast
    sleep(Duration::from_secs(2)).await;
    
    // ========== STEP 4: Create Channel ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 4: Alice Creates Channel                                    ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("📁 Alice creating 'general' channel...");
    let (channel, _op) = alice.create_channel(space_id, "general".to_string(), Some("General discussion".to_string())).await
        .expect("Alice failed to create channel");
    let channel_id = channel.id;
    println!("  ✓ Channel created: {}", hex::encode(&channel_id.0[..8]));
    println!();
    
    sleep(Duration::from_secs(1)).await;
    
    // ========== STEP 5: Create Thread ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 5: Alice Creates Thread                                     ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("💬 Alice creating 'Welcome' thread...");
    let (thread, _op) = alice.create_thread(space_id, channel_id, "Welcome".to_string(), "Welcome everyone!".to_string()).await
        .expect("Alice failed to create thread");
    let thread_id = thread.id;
    println!("  ✓ Thread created: {}", hex::encode(&thread_id.0[..8]));
    println!();
    
    sleep(Duration::from_secs(1)).await;
    
    // ========== STEP 6: Generate Invite ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 6: Alice Generates Invite                                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("🎟️  Alice generating invite code...");
    let _op = alice.create_invite(space_id, None, None).await
        .expect("Alice failed to create invite");
    
    // Get the invite code
    let invites = alice.list_invites(&space_id).await;
    let invite_code = invites.last()
        .expect("No invite found")
        .code.clone();
    println!("  ✓ Invite code: {}", invite_code);
    println!();
    
    sleep(Duration::from_secs(1)).await;
    
    // ========== STEP 7: Bob Joins ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 7: Bob Joins Space                                          ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("👤 Bob joining space with invite...");
    bob.join_space(&space_id, &invite_code).await
        .expect("Bob failed to join space");
    println!("  ✓ Bob joined space");
    println!();
    
    sleep(Duration::from_secs(2)).await;
    
    // ========== STEP 8: Charlie Joins ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 8: Charlie Joins Space                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("👤 Charlie joining space with invite...");
    charlie.join_space(&space_id, &invite_code).await
        .expect("Charlie failed to join space");
    println!("  ✓ Charlie joined space");
    println!();
    
    sleep(Duration::from_secs(2)).await;
    
    // ========== STEP 9: Send Messages ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 9: Exchange Messages                                        ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("💬 Alice sending message...");
    alice.send_message(&space_id, &channel_id, &thread_id, "Hello from Alice!").await
        .expect("Alice failed to send message");
    println!("  ✓ Alice: 'Hello from Alice!'");
    
    sleep(Duration::from_secs(1)).await;
    
    println!("💬 Bob sending message...");
    bob.send_message(&space_id, &channel_id, &thread_id, "Hi Alice! This is Bob!").await
        .expect("Bob failed to send message");
    println!("  ✓ Bob: 'Hi Alice! This is Bob!'");
    
    sleep(Duration::from_secs(1)).await;
    
    println!("💬 Charlie sending message...");
    charlie.send_message(&space_id, &channel_id, &thread_id, "Hey everyone! Charlie here!").await
        .expect("Charlie failed to send message");
    println!("  ✓ Charlie: 'Hey everyone! Charlie here!'");
    
    println!();
    
    // Give time for messages to propagate
    sleep(Duration::from_secs(3)).await;
    
    // ========== STEP 10: Verify Messages ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ STEP 10: Verify Message Synchronization                          ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    
    println!("🔍 Checking if all peers received all messages...");
    
    let alice_messages = alice.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await
        .expect("Failed to get Alice's messages");
    println!("  Alice sees {} messages", alice_messages.len());
    
    let bob_messages = bob.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await
        .expect("Failed to get Bob's messages");
    println!("  Bob sees {} messages", bob_messages.len());
    
    let charlie_messages = charlie.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await
        .expect("Failed to get Charlie's messages");
    println!("  Charlie sees {} messages", charlie_messages.len());
    
    println!();
    
    // Verify all have the same count
    assert_eq!(alice_messages.len(), 3, "Alice should see 3 messages");
    assert_eq!(bob_messages.len(), 3, "Bob should see 3 messages");
    assert_eq!(charlie_messages.len(), 3, "Charlie should see 3 messages");
    
    println!("✅ All peers have synchronized messages!");
    println!();
    
    // ========== SUCCESS ==========
    
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║                         TEST PASSED! ✅                           ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("✅ All assertions passed:");
    println!("   • 3 peers initialized and connected");
    println!("   • Space, channel, and thread created");
    println!("   • Bob and Charlie joined via invite");
    println!("   • All 3 messages sent successfully");
    println!("   • All peers received all messages (CRDT sync working!)");
    println!();
    println!("🎉 P2P messaging system working correctly!");
    println!();
}
