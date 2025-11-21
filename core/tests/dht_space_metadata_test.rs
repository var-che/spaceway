use spaceway_core::client::{Client, ClientConfig};
use spaceway_core::crypto::Keypair;
use spaceway_core::{SpaceVisibility, InvitePermissions, Signature, SpaceId, EpochId};
use anyhow::Result;
use tokio;
use std::path::PathBuf;

/// Test storing Space metadata in DHT and retrieving it
#[tokio::test]
async fn test_store_and_retrieve_space_metadata() -> Result<()> {
    // Create two clients with separate storage paths
    let alice_keypair = Keypair::generate();
    let alice_config = ClientConfig {
        storage_path: PathBuf::from("./test-data/alice-dht-space-1"),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let alice = Client::new(alice_keypair.clone(), alice_config)?;
    
    let bob_keypair = Keypair::generate();
    let bob_config = ClientConfig {
        storage_path: PathBuf::from("./test-data/bob-dht-space-1"),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let bob = Client::new(bob_keypair.clone(), bob_config)?;
    
    // Start both clients
    alice.start().await?;
    bob.start().await?;
    
    // Wait for network initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Alice creates a Space
    let (space, _, _) = alice.create_space_with_visibility(
        "Test Space".to_string(),
        Some("A test space for DHT metadata".to_string()),
        SpaceVisibility::Public,
    ).await?;
    
    let space_id = space.id.clone();
    
    // Wait for DHT propagation
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Bob retrieves the Space metadata from DHT
    let retrieved_space = bob.dht_get_space(&space_id).await?;
    
    // Verify metadata matches
    assert_eq!(retrieved_space.id, space.id);
    assert_eq!(retrieved_space.name, space.name);
    assert_eq!(retrieved_space.description, space.description);
    assert_eq!(retrieved_space.owner, space.owner);
    assert_eq!(retrieved_space.visibility, space.visibility);
    
    println!("✓ Successfully stored and retrieved Space metadata from DHT");
    Ok(())
}

/// Test that Bob can join Alice's Space even when Alice is offline
#[tokio::test]
async fn test_offline_space_joining() -> Result<()> {
    // Create two clients with separate storage paths
    let alice_keypair = Keypair::generate();
    let alice_config = ClientConfig {
        storage_path: PathBuf::from("./test-data/alice-dht-space-2"),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let alice = Client::new(alice_keypair.clone(), alice_config)?;
    
    let bob_keypair = Keypair::generate();
    let bob_config = ClientConfig {
        storage_path: PathBuf::from("./test-data/bob-dht-space-2"),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    let bob = Client::new(bob_keypair.clone(), bob_config)?;
    
    // Start both clients
    alice.start().await?;
    bob.start().await?;
    
    // Wait for network initialization
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Alice creates a Public Space
    let (space, _, _) = alice.create_space_with_visibility(
        "Public Space".to_string(),
        Some("Anyone can join".to_string()),
        SpaceVisibility::Public,
    ).await?;
    
    let space_id = space.id.clone();
    
    // Wait for DHT propagation
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Alice goes offline (drop client)
    drop(alice);
    
    println!("Alice went offline");
    
    // Wait a moment
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Bob joins the Space from DHT (Alice is offline!)
    let joined_space = bob.join_space_from_dht(space_id.clone()).await?;
    
    // Verify Bob successfully joined
    assert_eq!(joined_space.id, space_id);
    assert_eq!(joined_space.name, "Public Space");
    assert_eq!(joined_space.visibility, SpaceVisibility::Public);
    
    println!("✓ Bob successfully joined Space while Alice was offline!");
    Ok(())
}

/// Test that signature verification works
#[tokio::test]
async fn test_space_metadata_signature_verification() -> Result<()> {
    use spaceway_core::forum::{SpaceMetadata, Space};
    use std::collections::HashMap;
    use ed25519_dalek::SigningKey;
    
    // Create a keypair
    let keypair = Keypair::generate();
    let owner = keypair.user_id();
    
    // Create a Space
    let space_id = SpaceId::from_content(&owner, "Test Space", 1234567890);
    let space = Space {
        id: space_id.clone(),
        name: "Test Space".to_string(),
        description: Some("Test description".to_string()),
        owner: owner.clone(),
        visibility: SpaceVisibility::Public,
        members: HashMap::new(),
        invites: HashMap::new(),
        invite_permissions: InvitePermissions::default(),
        epoch: EpochId(0),
        created_at: 1234567890,
    };
    
    // Create metadata with valid signature
    let signing_key = SigningKey::from_bytes(&keypair.to_bytes());
    let metadata = SpaceMetadata::from_space(&space, &signing_key);
    
    // Verify signature is valid
    assert!(metadata.verify_signature());
    
    println!("✓ Signature verification works correctly");
    Ok(())
}

/// Test encryption and decryption round-trip
#[tokio::test]
async fn test_encrypted_metadata_round_trip() -> Result<()> {
    use spaceway_core::forum::{SpaceMetadata, EncryptedSpaceMetadata, Space};
    use std::collections::HashMap;
    use ed25519_dalek::SigningKey;
    
    // Create a keypair
    let keypair = Keypair::generate();
    let owner = keypair.user_id();
    
    // Create a Space
    let space_id = SpaceId::from_content(&owner, "Test Space", 1234567890);
    let space = Space {
        id: space_id.clone(),
        name: "Test Space".to_string(),
        description: Some("Test description".to_string()),
        owner: owner.clone(),
        visibility: SpaceVisibility::Public,
        members: HashMap::new(),
        invites: HashMap::new(),
        invite_permissions: InvitePermissions::default(),
        epoch: EpochId(0),
        created_at: 1234567890,
    };
    
    // Create metadata with signature
    let signing_key = SigningKey::from_bytes(&keypair.to_bytes());
    let metadata = SpaceMetadata::from_space(&space, &signing_key);
    
    // Encrypt
    let encrypted = EncryptedSpaceMetadata::encrypt(&metadata)?;
    
    // Verify encrypted data is different from plaintext
    assert_ne!(encrypted.ciphertext, metadata.to_bytes()?);
    
    // Decrypt
    let decrypted = EncryptedSpaceMetadata::decrypt(&encrypted)?;
    
    // Verify decrypted matches original
    assert_eq!(decrypted.id, metadata.id);
    assert_eq!(decrypted.name, metadata.name);
    assert_eq!(decrypted.description, metadata.description);
    assert_eq!(decrypted.owner, metadata.owner);
    assert_eq!(decrypted.visibility, metadata.visibility);
    assert_eq!(decrypted.created_at, metadata.created_at);
    
    println!("✓ Encryption/decryption round-trip successful");
    Ok(())
}
