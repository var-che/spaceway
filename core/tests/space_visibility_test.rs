use descord_core::{Client, ClientConfig, crypto::Keypair};
use descord_core::types::SpaceVisibility;
use anyhow::Result;
use std::path::PathBuf;

/// Helper to create a test client
fn create_test_client(name: &str) -> Result<Client> {
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
async fn test_create_space_with_public_visibility() -> Result<()> {
    let client = create_test_client("test_public_visibility")?;
    let (space, _op, _) = client.create_space_with_visibility(
        "Public Space".to_string(),
        Some("Test description".to_string()),
        SpaceVisibility::Public
    ).await?;
    
    assert_eq!(space.visibility, SpaceVisibility::Public);
    assert!(space.visibility.is_discoverable());
    assert!(!space.visibility.requires_invite());
    assert!(!space.visibility.is_hidden());
    
    Ok(())
}

#[tokio::test]
async fn test_create_space_with_private_visibility() -> Result<()> {
    let client = create_test_client("test_private_visibility")?;
    let (space, _op, _) = client.create_space_with_visibility(
        "Private Space".to_string(),
        Some("Test description".to_string()),
        SpaceVisibility::Private
    ).await?;
    
    assert_eq!(space.visibility, SpaceVisibility::Private);
    assert!(!space.visibility.is_discoverable());
    assert!(space.visibility.requires_invite());
    assert!(!space.visibility.is_hidden());
    
    Ok(())
}

#[tokio::test]
async fn test_create_space_with_hidden_visibility() -> Result<()> {
    let client = create_test_client("test_hidden_visibility")?;
    let (space, _op, _) = client.create_space_with_visibility(
        "Hidden Space".to_string(),
        Some("Test description".to_string()),
        SpaceVisibility::Hidden
    ).await?;
    
    assert_eq!(space.visibility, SpaceVisibility::Hidden);
    assert!(!space.visibility.is_discoverable());
    assert!(space.visibility.requires_invite());
    assert!(space.visibility.is_hidden());
    
    Ok(())
}

#[tokio::test]
async fn test_default_space_visibility() -> Result<()> {
    let client = create_test_client("test_default_visibility")?;
    // Using the original create_space method should default to Private
    let (space, _op, _) = client.create_space(
        "Default Space".to_string(),
        Some("Test description".to_string()),
    ).await?;
    
    // Default should be Private
    assert_eq!(space.visibility, SpaceVisibility::Private);
    
    Ok(())
}

#[tokio::test]
async fn test_update_visibility() -> Result<()> {
    let admin = create_test_client("test_update_visibility")?;
    let (space, _space_op, _) = admin.create_space_with_visibility(
        "Test Space".to_string(),
        Some("Test description".to_string()),
        SpaceVisibility::Private
    ).await?;
    
    // Admin should be able to change visibility
    let _visibility_op = admin.update_space_visibility(
        space.id,
        SpaceVisibility::Public
    ).await?;
    
    // Verify the space visibility was updated
    let spaces = admin.list_spaces().await;
    let updated_space = spaces.iter().find(|s| s.id == space.id).unwrap();
    assert_eq!(updated_space.visibility, SpaceVisibility::Public);
    
    Ok(())
}

#[tokio::test]
async fn test_visibility_enum_cbor_serialization() -> Result<()> {
    // Test Public
    let public = SpaceVisibility::Public;
    let mut buffer = Vec::new();
    minicbor::encode(&public, &mut buffer)?;
    let decoded: SpaceVisibility = minicbor::decode(&buffer)?;
    assert_eq!(public, decoded);
    
    // Test Private
    let private = SpaceVisibility::Private;
    buffer.clear();
    minicbor::encode(&private, &mut buffer)?;
    let decoded: SpaceVisibility = minicbor::decode(&buffer)?;
    assert_eq!(private, decoded);
    
    // Test Hidden
    let hidden = SpaceVisibility::Hidden;
    buffer.clear();
    minicbor::encode(&hidden, &mut buffer)?;
    let decoded: SpaceVisibility = minicbor::decode(&buffer)?;
    assert_eq!(hidden, decoded);
    
    Ok(())
}
