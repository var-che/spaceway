//! Interactive 3-Peer Demo
//! 
//! This example programmatically starts 3 peers and lets you watch them communicate.
//! Much simpler than opening 3 terminals!
//!
//! Run with: cargo +nightly run --example three_peer_demo

use spaceway_core::{Client, ClientConfig};
use spaceway_core::crypto::Keypair;
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;

async fn create_peer(name: &str, port: u16) -> (Client, String) {
    let data_dir = TempDir::new().unwrap();
    let keypair = Keypair::generate();
    
    let config = ClientConfig {
        data_dir: data_dir.path().to_path_buf(),
        keypair,
        listen_port: Some(port),
        relay_address: None,
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(config).await.unwrap();
    let peer_id = client.local_peer_id().await.unwrap().to_string();
    
    println!("✓ {} ready (127.0.0.1:{})", name, port);
    
    (client, peer_id)
}

#[tokio::main]
async fn main() {
    println!("\n╔═══════════════════════════════════════════════════════╗");
    println!("║  Spaceway 3-Peer Demo - Automated P2P Testing        ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");
    
    println!("🚀 Starting 3 peers...\n");
    
    let (alice, alice_peer_id) = create_peer("Alice", 9001).await;
    let (bob, _) = create_peer("Bob", 9002).await;
    let (charlie, _) = create_peer("Charlie", 9003).await;
    
    println!("\n⏳ Waiting for peers to start listening...");
    sleep(Duration::from_secs(2)).await;
    
    println!("\n🔗 Connecting peers...");
    let alice_addr = format!("/ip4/127.0.0.1/tcp/9001/p2p/{}", alice_peer_id);
    
    bob.connect_to_peer(&alice_addr).await.unwrap();
    println!("  ✓ Bob → Alice");
    
    charlie.connect_to_peer(&alice_addr).await.unwrap();
    println!("  ✓ Charlie → Alice");
    
    println!("\n⏳ Forming peer mesh...");
    sleep(Duration::from_secs(5)).await;
    
    println!("\n📢 Alice creating space...");
    let space_id = alice.create_space("DemoSpace").await.unwrap();
    println!("  Space ID: {}", space_id);
    sleep(Duration::from_secs(2)).await;
    
    println!("\n📁 Alice creating channel...");
    let channel_id = alice.create_channel(&space_id, "general").await.unwrap();
    sleep(Duration::from_secs(1)).await;
    
    println!("\n💬 Alice creating thread...");
    let thread_id = alice.create_thread(&space_id, &channel_id, "Demo", "Welcome!").await.unwrap();
    sleep(Duration::from_secs(1)).await;
    
    println!("\n🎟️  Generating invite...");
    let invite = alice.create_invite(&space_id).await.unwrap();
    println!("  Invite code: {}", invite);
    
    println!("\n👥 Bob and Charlie joining...");
    bob.join_space(&space_id, &invite).await.unwrap();
    println!("  ✓ Bob joined");
    sleep(Duration::from_secs(1)).await;
    
    charlie.join_space(&space_id, &invite).await.unwrap();
    println!("  ✓ Charlie joined");
    sleep(Duration::from_secs(2)).await;
    
    println!("\n💬 Sending messages...\n");
    
    alice.send_message(&space_id, &channel_id, &thread_id, "Hello everyone!").await.unwrap();
    println!("  Alice: Hello everyone!");
    sleep(Duration::from_secs(1)).await;
    
    bob.send_message(&space_id, &channel_id, &thread_id, "Hi Alice!").await.unwrap();
    println!("  Bob: Hi Alice!");
    sleep(Duration::from_secs(1)).await;
    
    charlie.send_message(&space_id, &channel_id, &thread_id, "Hey team!").await.unwrap();
    println!("  Charlie: Hey team!");
    sleep(Duration::from_secs(2)).await;
    
    println!("\n🔍 Verifying synchronization...\n");
    
    let alice_msgs = alice.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await.unwrap();
    let bob_msgs = bob.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await.unwrap();
    let charlie_msgs = charlie.get_thread_messages(&space_id, &channel_id, &thread_id, 10).await.unwrap();
    
    println!("  Alice sees: {} messages", alice_msgs.len());
    println!("  Bob sees: {} messages", bob_msgs.len());
    println!("  Charlie sees: {} messages", charlie_msgs.len());
    
    if alice_msgs.len() == 3 && bob_msgs.len() == 3 && charlie_msgs.len() == 3 {
        println!("\n✅ SUCCESS! All peers synchronized!\n");
    } else {
        println!("\n⚠️  Sync incomplete (expected 3 messages each)\n");
    }
    
    println!("Demo complete. Press Ctrl+C to exit.\n");
    sleep(Duration::from_secs(5)).await;
}
