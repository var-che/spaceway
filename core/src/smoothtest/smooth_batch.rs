//! Batch of test clients that can communicate with each other

use super::SmoothClient;
use crate::client::ClientConfig;
use anyhow::Result;
use std::ops::{Index, IndexMut};

/// A collection of SmoothClients that can communicate with each other
/// 
/// This is the primary tool for testing distributed features.
/// All clients in a batch can discover each other via bootstrap peers.
pub struct SmoothClientBatch {
    clients: Vec<SmoothClient>,
}

impl SmoothClientBatch {
    /// Create a batch of N clients with default configuration
    /// 
    /// All clients will have isolated storage but share the same
    /// network configuration (allowing them to discover each other).
    pub fn new(count: usize) -> Result<Self> {
        if count == 0 {
            return Ok(Self {
                clients: vec![],
            });
        }

        // Create N clients with default config
        let mut clients = Vec::with_capacity(count);
        for _ in 0..count {
            clients.push(SmoothClient::new()?);
        }

        Ok(Self { clients })
    }

    /// Create a batch with custom configuration for each client
    pub fn with_config(count: usize, config: ClientConfig) -> Result<Self> {
        if count == 0 {
            return Ok(Self {
                clients: vec![],
            });
        }

        let mut clients = Vec::with_capacity(count);
        for _ in 0..count {
            clients.push(SmoothClient::with_config(config.clone())?);
        }

        Ok(Self { clients })
    }

    /// Get the number of clients in this batch
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Iterate over clients
    pub fn iter(&self) -> std::slice::Iter<SmoothClient> {
        self.clients.iter()
    }

    /// Iterate over clients mutably
    pub fn iter_mut(&mut self) -> std::slice::IterMut<SmoothClient> {
        self.clients.iter_mut()
    }

    /// Get a reference to a client by index
    pub fn get(&self, index: usize) -> Option<&SmoothClient> {
        self.clients.get(index)
    }

    /// Get a mutable reference to a client by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut SmoothClient> {
        self.clients.get_mut(index)
    }

    /// Connect all clients to each other
    /// 
    /// This waits for peer discovery to complete.
    /// In the current implementation, this just provides time
    /// for the DHT network to form.
    pub async fn connect_all(&mut self) -> Result<()> {
        // Give time for DHT bootstrap and peer discovery
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        Ok(())
    }

    /// Convert into inner vector of clients
    pub fn into_inner(self) -> Vec<SmoothClient> {
        self.clients
    }
}

impl Index<usize> for SmoothClientBatch {
    type Output = SmoothClient;

    fn index(&self, index: usize) -> &Self::Output {
        &self.clients[index]
    }
}

impl IndexMut<usize> for SmoothClientBatch {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.clients[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_creation() {
        let batch = SmoothClientBatch::new(3).unwrap();
        assert_eq!(batch.len(), 3);
    }

    #[tokio::test]
    async fn test_batch_indexing() {
        let batch = SmoothClientBatch::new(3).unwrap();
        let _alice = &batch[0];
        let _bob = &batch[1];
        let _carol = &batch[2];
    }
}

