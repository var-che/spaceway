//! Single test client with isolated storage and utilities

use crate::client::{Client, ClientConfig};
use crate::crypto::signing::Keypair;
use crate::forum::Space;
use crate::types::SpaceId;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use std::path::PathBuf;

/// A test client with isolated storage and convenient test methods
/// 
/// This wraps a real Client but provides:
/// - Automatic cleanup (tempfile TempDir)
/// - Convenient helper methods for tests
/// - Isolated RocksDB instance
pub struct SmoothClient {
    client: Arc<RwLock<Client>>,
    data_dir: tempfile::TempDir,
    keypair: Keypair,
}

impl SmoothClient {
    /// Create a new test client with random identity and isolated storage
    pub fn new() -> Result<Self> {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new test client with custom configuration
    pub fn with_config(mut config: ClientConfig) -> Result<Self> {
        let data_dir = tempfile::tempdir()?;
        
        // Override storage path to use temp directory
        config.storage_path = data_dir.path().to_path_buf();
        
        // Create random keypair for test
        let keypair = Keypair::generate();
        
        // Initialize client with isolated data directory
        let client = Client::new(keypair.clone(), config)?;

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            data_dir,
            keypair,
        })
    }

    /// Create a client configured to listen on a specific port
    pub fn with_listen_port(port: u16) -> Result<Self> {
        let config = ClientConfig {
            listen_addrs: vec![format!("/ip4/127.0.0.1/tcp/{}", port)],
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Get the client's keypair
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get a clone of the underlying client
    pub fn client(&self) -> Arc<RwLock<Client>> {
        Arc::clone(&self.client)
    }

    /// Create a space and return the Space object
    pub async fn create_space(&self, name: &str, description: Option<&str>) -> Result<Space> {
        let mut client = self.client.write().await;
        let (space, _op, _privacy) = client.create_space(
            name.to_string(), 
            description.map(|s| s.to_string())
        ).await?;
        Ok(space)
    }

    /// Join a space from DHT (simulating offline creator)
    pub async fn join_space_from_dht(&self, space_id: SpaceId) -> Result<()> {
        let mut client = self.client.write().await;
        client.join_space_from_dht(space_id).await?;
        Ok(())
    }

    /// Get the number of spaces this client knows about
    pub async fn space_count(&self) -> usize {
        let client = self.client.read().await;
        client.list_spaces().await.len()
    }

    /// List all spaces
    pub async fn list_spaces(&self) -> Vec<Space> {
        let client = self.client.read().await;
        client.list_spaces().await
    }

    /// Get DHT statistics for debugging
    pub async fn dht_stats(&self) -> String {
        let client = self.client.read().await;
        let spaces = client.list_spaces().await;
        format!("Spaces: {}", spaces.len())
    }

    /// Get the data directory path (useful for debugging)
    pub fn data_path(&self) -> PathBuf {
        self.data_dir.path().to_path_buf()
    }
}

// Implement Drop to ensure cleanup
impl Drop for SmoothClient {
    fn drop(&mut self) {
        // TempDir will be automatically cleaned up when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smooth_client_creation() {
        let client = SmoothClient::new().unwrap();
        assert_eq!(client.space_count().await, 0);
    }

    #[tokio::test]
    async fn test_smooth_client_create_space() {
        let client = SmoothClient::new().unwrap();
        let _space = client.create_space("test-space", Some("A test space")).await.unwrap();
        assert_eq!(client.space_count().await, 1);
    }
}

