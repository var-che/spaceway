//! Utilities for waiting for distributed system consistency

use super::{SmoothClient, SmoothClientBatch};
use crate::types::SpaceId;
use anyhow::{Result, bail};
use std::time::Duration;

/// Wait for DHT consistency across multiple clients
/// 
/// This polls all clients to check if they have the same view of the DHT.
/// Useful after DHT operations to ensure data has propagated.
/// 
/// # Arguments
/// 
/// * `timeout_secs` - Maximum time to wait in seconds
/// * `clients` - Clients to check for consistency
/// * `expected_space_count` - Expected number of spaces each client should see
/// 
/// # Returns
/// 
/// Ok(()) if consistency achieved within timeout, Err otherwise
pub async fn await_dht_consistency(
    timeout_secs: u64,
    clients: &[&SmoothClient],
    expected_space_count: usize,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    
    loop {
        if tokio::time::Instant::now() > deadline {
            bail!("DHT consistency timeout after {}s", timeout_secs);
        }

        // Check if all clients have the expected number of spaces
        let mut all_consistent = true;
        for client in clients {
            let count = client.space_count().await;
            if count != expected_space_count {
                all_consistent = false;
                break;
            }
        }

        if all_consistent {
            return Ok(());
        }

        // Wait a bit before checking again
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Wait for a specific space to appear in a client's DHT view
/// 
/// This is useful when testing offline joining - wait for the space
/// metadata to propagate through DHT before attempting to join.
pub async fn await_space_in_dht(
    timeout_secs: u64,
    client: &SmoothClient,
    space_id: SpaceId,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    
    loop {
        if tokio::time::Instant::now() > deadline {
            bail!("Space {:?} not found in DHT after {}s", space_id, timeout_secs);
        }

        // Try to retrieve space from DHT
        let client_arc = client.client();
        let client_lock = client_arc.read().await;
        match client_lock.dht_get_space(&space_id).await {
            Ok(_) => {
                drop(client_lock);
                return Ok(());
            }
            Err(_) => {
                drop(client_lock);
                // Not found yet, keep waiting
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Wait for batch to reach consistent DHT state
/// 
/// Convenience function for waiting on all clients in a batch.
pub async fn await_batch_consistency(
    timeout_secs: u64,
    batch: &SmoothClientBatch,
    expected_space_count: usize,
) -> Result<()> {
    let clients: Vec<&SmoothClient> = batch.iter().collect();
    await_dht_consistency(timeout_secs, &clients, expected_space_count).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consistency_timeout() {
        let batch = SmoothClientBatch::new(2).unwrap();
        
        // Should timeout since we're expecting 1 space but none exist
        let result = await_batch_consistency(1, &batch, 1).await;
        assert!(result.is_err());
    }
}

