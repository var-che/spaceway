use spaceway_core::{Client, ClientConfig, Result, crypto::Keypair};
use spaceway_core::types::{SpaceVisibility, NetworkTransportMode, PrivacyLevel};
use std::path::PathBuf;

#[tokio::test]
async fn test_public_space_privacy_info() -> Result<()> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    
    let (space, _op, privacy_info) = client.create_space_with_visibility(
        "Public Space".to_string(),
        Some("A test public space".to_string()),
        SpaceVisibility::Public
    ).await?;
    
    // Verify visibility
    assert_eq!(space.visibility, SpaceVisibility::Public);
    
    // Verify privacy info
    assert_eq!(privacy_info.visibility, SpaceVisibility::Public);
    assert_eq!(privacy_info.privacy_level, PrivacyLevel::Low);
    assert_eq!(privacy_info.transport_mode, NetworkTransportMode::Direct);
    
    // Verify exposed data warnings
    assert!(privacy_info.exposed_data.iter().any(|d| d.contains("IP address")));
    assert!(privacy_info.exposed_data.iter().any(|d| d.contains("Online/offline")));
    
    // Verify protected data
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("Message content")));
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("encrypted")));
    
    // Verify latency is low (Direct connection)
    assert!(privacy_info.latency_ms.start < 100);
    
    println!("Public Space Privacy Info:\n{}", privacy_info.format_for_user());
    
    Ok(())
}

#[tokio::test]
async fn test_private_space_privacy_info() -> Result<()> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    
    let (space, _op, privacy_info) = client.create_space_with_visibility(
        "Private Space".to_string(),
        Some("A test private space".to_string()),
        SpaceVisibility::Private
    ).await?;
    
    // Verify visibility
    assert_eq!(space.visibility, SpaceVisibility::Private);
    
    // Verify privacy info
    assert_eq!(privacy_info.privacy_level, PrivacyLevel::High);
    assert_eq!(privacy_info.transport_mode, NetworkTransportMode::Relay);
    
    // Verify IP is protected
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("IP address")));
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("hidden via relay")));
    
    // Verify latency is higher (Relay connection)
    assert!(privacy_info.latency_ms.start >= 50);
    assert!(privacy_info.latency_ms.end <= 200);
    
    println!("Private Space Privacy Info:\n{}", privacy_info.format_for_user());
    
    Ok(())
}

#[tokio::test]
async fn test_hidden_space_privacy_info() -> Result<()> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    
    let (space, _op, privacy_info) = client.create_space_with_visibility(
        "Hidden Space".to_string(),
        Some("Maximum privacy space".to_string()),
        SpaceVisibility::Hidden
    ).await?;
    
    // Verify visibility
    assert_eq!(space.visibility, SpaceVisibility::Hidden);
    
    // Verify privacy info
    assert_eq!(privacy_info.privacy_level, PrivacyLevel::Maximum);
    assert_eq!(privacy_info.transport_mode, NetworkTransportMode::Relay);
    
    // Verify maximum privacy protections
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("IP address")));
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("Space existence")));
    assert!(privacy_info.protected_data.iter().any(|d| d.contains("invite-only")));
    
    // Verify minimal exposed data
    assert!(privacy_info.exposed_data.len() <= 2); // Should only expose timing
    
    println!("Hidden Space Privacy Info:\n{}", privacy_info.format_for_user());
    
    Ok(())
}

#[tokio::test]
async fn test_default_space_uses_private() -> Result<()> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    
    // create_space without visibility should default to Private
    let (space, _op, privacy_info) = client.create_space(
        "Default Space".to_string(),
        None
    ).await?;
    
    assert_eq!(space.visibility, SpaceVisibility::Private);
    assert_eq!(privacy_info.privacy_level, PrivacyLevel::High);
    assert_eq!(privacy_info.transport_mode, NetworkTransportMode::Relay);
    
    Ok(())
}

#[tokio::test]
async fn test_transport_mode_selection() {
    assert_eq!(SpaceVisibility::Public.transport_mode(), NetworkTransportMode::Direct);
    assert_eq!(SpaceVisibility::Private.transport_mode(), NetworkTransportMode::Relay);
    assert_eq!(SpaceVisibility::Hidden.transport_mode(), NetworkTransportMode::Relay);
}

#[tokio::test]
async fn test_privacy_warnings() {
    let public_warning = SpaceVisibility::Public.privacy_warning();
    assert!(public_warning.contains("IP address"));
    assert!(public_warning.contains("visible"));
    
    let private_warning = SpaceVisibility::Private.privacy_warning();
    assert!(private_warning.contains("hidden"));
    assert!(private_warning.contains("relay"));
    
    let hidden_warning = SpaceVisibility::Hidden.privacy_warning();
    assert!(hidden_warning.contains("Maximum privacy"));
    assert!(hidden_warning.contains("relay"));
}

#[tokio::test]
async fn test_get_join_privacy_info() -> Result<()> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    let client = Client::new(keypair, config)?;
    
    let privacy_info = client.get_join_privacy_info(SpaceVisibility::Public);
    assert_eq!(privacy_info.privacy_level, PrivacyLevel::Low);
    assert!(privacy_info.warning.contains("IP address will be visible"));
    
    Ok(())
}
