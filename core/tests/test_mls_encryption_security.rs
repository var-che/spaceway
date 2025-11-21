use spaceway_core::{Client, ClientConfig, types::SpaceId, mls::group::MlsGroup};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};
use anyhow::Result;

#[tokio::test]
async fn test_kicked_member_cannot_decrypt_messages() -> Result<()> {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║   MLS ENCRYPTION SECURITY TEST                             ║");
    println!("║   Verify: Kicked members cannot decrypt new messages      ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Step 1: Setup Alice
    println!("📝 Step 1: Creating Alice with MLS provider...");
    let alice_keypair = spaceway_core::crypto::signing::Keypair::generate();
    let alice_config = ClientConfig {
        storage_path: PathBuf::from("test-alice-mls-security"),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    let alice = Client::new(alice_keypair.clone(), alice_config)?;
    let alice_id = alice.user_id();
    alice.start().await?;
    sleep(Duration::from_millis(500)).await;
    println!("✓ Alice started (user: {})", hex::encode(&alice_id.0[..4]));

    // Step 2: Setup Bob
    println!("\n📝 Step 2: Creating Bob with MLS provider...");
    let bob_keypair = spaceway_core::crypto::signing::Keypair::generate();
    let bob_config = ClientConfig {
        storage_path: PathBuf::from("test-bob-mls-security"),
        listen_addrs: vec![],
        bootstrap_peers: vec![],
    };
    let bob = Client::new(bob_keypair.clone(), bob_config)?;
    let bob_id = bob.user_id();
    bob.start().await?;
    sleep(Duration::from_millis(500)).await;
    println!("✓ Bob started (user: {})", hex::encode(&bob_id.0[..4]));

    // Step 3: Alice creates MLS group for a space
    println!("\n📝 Step 3: Alice creates MLS group...");
    let space_id = SpaceId(rand::random());
    let mut alice_space_manager = alice.space_manager.write().await;
    let alice_provider = alice.mls_provider.read().await;
    
    let alice_mls_group = MlsGroup::new(
        &alice_id,
        &space_id,
        &alice_provider,
    )?;
    
    alice_space_manager.create_mls_group(space_id, alice_mls_group);
    drop(alice_space_manager);
    drop(alice_provider);
    println!("✓ MLS group created (epoch 0)");

    // Step 4: Generate a KeyPackage for Bob
    println!("\n📝 Step 4: Generating Bob's KeyPackage...");
    let bob_provider = bob.mls_provider.read().await;
    let bob_keypackage = bob_provider.generate_key_package(&bob_id)?;
    drop(bob_provider);
    println!("✓ Bob's KeyPackage generated");

    // Step 5: Alice adds Bob to the MLS group
    println!("\n📝 Step 5: Alice adds Bob to MLS group...");
    let mut alice_space_manager = alice.space_manager.write().await;
    let alice_provider = alice.mls_provider.read().await;
    
    let alice_mls_group = alice_space_manager.get_mls_group_mut(&space_id)
        .ok_or_else(|| anyhow::anyhow!("MLS group not found"))?;
    
    let welcome_msg = alice_mls_group.add_members(&[bob_keypackage], &alice_provider)?;
    drop(alice_space_manager);
    drop(alice_provider);
    println!("✓ Bob added to group (epoch 0 → 1)");
    println!("  Welcome message size: {} bytes", welcome_msg.len());

    // Step 6: Bob processes Welcome and joins the group
    println!("\n📝 Step 6: Bob processes Welcome message...");
    let mut bob_space_manager = bob.space_manager.write().await;
    let bob_provider = bob.mls_provider.read().await;
    
    let bob_mls_group = MlsGroup::from_welcome(
        &welcome_msg,
        &bob_id,
        &bob_provider,
    )?;
    
    bob_space_manager.create_mls_group(space_id, bob_mls_group);
    drop(bob_space_manager);
    drop(bob_provider);
    println!("✓ Bob joined group (epoch 1)");

    // Step 7: Alice encrypts a message (Bob is a member)
    println!("\n📝 Step 7: Alice encrypts message while Bob is a member...");
    let message1 = b"This message should be readable by Bob";
    
    let mut alice_space_manager = alice.space_manager.write().await;
    let alice_provider = alice.mls_provider.read().await;
    let alice_mls_group = alice_space_manager.get_mls_group_mut(&space_id).unwrap();
    
    let encrypted_msg1 = alice_mls_group.encrypt_application_message(message1, &alice_provider)?;
    let encrypted_bytes1 = encrypted_msg1.to_bytes()?;
    drop(alice_space_manager);
    drop(alice_provider);
    println!("✓ Message encrypted ({} bytes)", encrypted_bytes1.len());

    // Step 8: Bob decrypts the message (should succeed)
    println!("\n📝 Step 8: Bob decrypts message...");
    let mut bob_space_manager = bob.space_manager.write().await;
    let bob_provider = bob.mls_provider.read().await;
    let bob_mls_group = bob_space_manager.get_mls_group_mut(&space_id).unwrap();
    
    match bob_mls_group.decrypt_application_message(&encrypted_bytes1, &bob_provider) {
        Ok(decrypted) => {
            println!("✓ Bob decrypted message: {:?}", String::from_utf8_lossy(&decrypted));
            assert_eq!(decrypted, message1, "Decrypted message should match original");
        }
        Err(e) => {
            panic!("❌ Bob should be able to decrypt (he's a member): {}", e);
        }
    }
    drop(bob_space_manager);
    drop(bob_provider);

    // Step 9: Alice removes Bob from the group
    println!("\n📝 Step 9: Alice removes Bob from MLS group...");
    let mut alice_space_manager = alice.space_manager.write().await;
    let alice_provider = alice.mls_provider.read().await;
    let alice_mls_group = alice_space_manager.get_mls_group_mut(&space_id).unwrap();
    
    alice_mls_group.remove_members(&[bob_id], &alice_provider)?;
    drop(alice_space_manager);
    drop(alice_provider);
    println!("✓ Bob removed from group (epoch 1 → 2)");
    println!("  Alice has new epoch keys, Bob does NOT");

    // Step 10: Alice encrypts another message (after kicking Bob)
    println!("\n📝 Step 10: Alice encrypts message AFTER kicking Bob...");
    let message2 = b"This message should NOT be readable by Bob";
    
    let mut alice_space_manager = alice.space_manager.write().await;
    let alice_provider = alice.mls_provider.read().await;
    let alice_mls_group = alice_space_manager.get_mls_group_mut(&space_id).unwrap();
    
    let encrypted_msg2 = alice_mls_group.encrypt_application_message(message2, &alice_provider)?;
    let encrypted_bytes2 = encrypted_msg2.to_bytes()?;
    drop(alice_space_manager);
    drop(alice_provider);
    println!("✓ Message encrypted with NEW epoch keys ({} bytes)", encrypted_bytes2.len());

    // Step 11: Bob tries to decrypt (should FAIL - he was kicked)
    println!("\n📝 Step 11: Bob tries to decrypt post-kick message...");
    let mut bob_space_manager = bob.space_manager.write().await;
    let bob_provider = bob.mls_provider.read().await;
    let bob_mls_group = bob_space_manager.get_mls_group_mut(&space_id).unwrap();
    
    match bob_mls_group.decrypt_application_message(&encrypted_bytes2, &bob_provider) {
        Ok(decrypted) => {
            panic!("❌ SECURITY VIOLATION: Bob decrypted post-kick message: {:?}", 
                String::from_utf8_lossy(&decrypted));
        }
        Err(e) => {
            println!("✓ Bob CANNOT decrypt (correct behavior!)");
            println!("  Error: {}", e);
            println!("\n✅ TEST PASSED: Forward secrecy working!");
            println!("   Kicked members cannot decrypt new messages.");
        }
    }

    // Cleanup
    cleanup_test_dirs();
    Ok(())
}

fn cleanup_test_dirs() {
    let _ = std::fs::remove_dir_all("test-alice-mls-security");
    let _ = std::fs::remove_dir_all("test-bob-mls-security");
}
