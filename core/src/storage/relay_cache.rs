///! Relay reputation caching
///!
///! Phase 3 Feature: Persist relay performance metrics to optimize relay selection

use anyhow::{Context, Result};
use super::Storage;
use serde::{Serialize, Deserialize};
use libp2p::PeerId;

/// Relay performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStats {
    pub peer_id: String,
    pub successful_circuits: u64,
    pub failed_circuits: u64,
    pub total_bytes_relayed: u64,
    pub last_seen: u64,
    pub first_seen: u64,
    pub average_latency_ms: Option<u32>,
}

impl RelayStats {
    pub fn new(peer_id: PeerId) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            peer_id: peer_id.to_string(),
            successful_circuits: 0,
            failed_circuits: 0,
            total_bytes_relayed: 0,
            last_seen: now,
            first_seen: now,
            average_latency_ms: None,
        }
    }
    
    /// Calculate uptime percentage
    pub fn uptime_percentage(&self) -> f32 {
        let total = self.successful_circuits + self.failed_circuits;
        if total == 0 {
            return 100.0;
        }
        (self.successful_circuits as f32 / total as f32) * 100.0
    }
    
    /// Calculate reputation score (0-100)
    pub fn reputation_score(&self) -> f32 {
        let uptime = self.uptime_percentage();
        let recency_bonus = self.recency_score();
        let volume_bonus = self.volume_score();
        
        // Weighted average: 60% uptime, 20% recency, 20% volume
        (uptime * 0.6) + (recency_bonus * 0.2) + (volume_bonus * 0.2)
    }
    
    fn recency_score(&self) -> f32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_hours = (now - self.last_seen) / 3600;
        
        // 100 points if seen in last hour, 0 if not seen in 24h
        if age_hours == 0 {
            100.0
        } else if age_hours >= 24 {
            0.0
        } else {
            100.0 - (age_hours as f32 * 4.17) // Linear decay
        }
    }
    
    fn volume_score(&self) -> f32 {
        // 100 points for relaying >1GB, scaled linearly
        let gb = self.total_bytes_relayed as f32 / (1024.0 * 1024.0 * 1024.0);
        (gb * 100.0).min(100.0)
    }
    
    /// Check if relay is stale (not seen in >7 days)
    pub fn is_stale(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_days = (now - self.last_seen) / (3600 * 24);
        age_days > 7
    }
}

impl Storage {
    /// Save relay statistics
    pub fn save_relay_stats(&self, stats: &RelayStats) -> Result<()> {
        let cf = self.db.cf_handle(Self::CF_RELAYS)
            .context("Missing relays column family")?;
        
        let key = format!("stats:{}", stats.peer_id);
        let value = bincode::serialize(stats)
            .context("Failed to serialize relay stats")?;
        
        self.db.put_cf(&cf, key.as_bytes(), &value)
            .context("Failed to save relay stats")?;
        
        tracing::debug!(
            peer_id = %stats.peer_id,
            reputation = %stats.reputation_score(),
            "Saved relay stats"
        );
        
        Ok(())
    }
    
    /// Load relay statistics
    pub fn load_relay_stats(&self, peer_id: &PeerId) -> Result<Option<RelayStats>> {
        let cf = self.db.cf_handle(Self::CF_RELAYS)
            .context("Missing relays column family")?;
        
        let key = format!("stats:{}", peer_id);
        let value = self.db.get_cf(&cf, key.as_bytes())
            .context("Failed to read relay stats")?;
        
        match value {
            Some(bytes) => {
                let stats = bincode::deserialize(&bytes)
                    .context("Failed to deserialize relay stats")?;
                Ok(Some(stats))
            }
            None => Ok(None),
        }
    }
    
    /// Get all known relays sorted by reputation
    pub fn get_top_relays(&self, limit: usize) -> Result<Vec<RelayStats>> {
        let cf = self.db.cf_handle(Self::CF_RELAYS)
            .context("Missing relays column family")?;
        
        let prefix = "stats:";
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        
        let mut relays = Vec::new();
        
        for item in iter {
            let (key, value) = item.context("Iterator error")?;
            let key_str = String::from_utf8_lossy(&key);
            
            if !key_str.starts_with(prefix) {
                break;
            }
            
            let stats: RelayStats = bincode::deserialize(&value)
                .context("Failed to deserialize relay stats")?;
            
            // Skip stale relays
            if !stats.is_stale() {
                relays.push(stats);
            }
        }
        
        // Sort by reputation (highest first)
        relays.sort_by(|a, b| {
            b.reputation_score().partial_cmp(&a.reputation_score()).unwrap()
        });
        
        // Take top N
        relays.truncate(limit);
        
        Ok(relays)
    }
    
    /// Prune stale relays (not seen in >7 days)
    pub fn prune_stale_relays(&self) -> Result<usize> {
        let cf = self.db.cf_handle(Self::CF_RELAYS)
            .context("Missing relays column family")?;
        
        let prefix = "stats:";
        let iter = self.db.iterator_cf(&cf, rocksdb::IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        
        let mut pruned_count = 0;
        let mut keys_to_delete = Vec::new();
        
        for item in iter {
            let (key, value) = item.context("Iterator error")?;
            let key_str = String::from_utf8_lossy(&key);
            
            if !key_str.starts_with(prefix) {
                break;
            }
            
            let stats: RelayStats = bincode::deserialize(&value)
                .context("Failed to deserialize relay stats")?;
            
            if stats.is_stale() {
                keys_to_delete.push(key.to_vec());
            }
        }
        
        // Delete stale relays
        for key in keys_to_delete {
            self.db.delete_cf(&cf, &key)
                .context("Failed to delete stale relay")?;
            pruned_count += 1;
        }
        
        if pruned_count > 0 {
            tracing::info!(
                pruned_count,
                "Pruned stale relays"
            );
        }
        
        Ok(pruned_count)
    }
    
    /// Update relay stats after circuit success/failure
    pub fn record_circuit_result(
        &self,
        peer_id: &PeerId,
        success: bool,
        bytes_relayed: u64,
        latency_ms: Option<u32>,
    ) -> Result<()> {
        let mut stats = self.load_relay_stats(peer_id)?
            .unwrap_or_else(|| RelayStats::new(*peer_id));
        
        // Update counters
        if success {
            stats.successful_circuits += 1;
            stats.total_bytes_relayed += bytes_relayed;
        } else {
            stats.failed_circuits += 1;
        }
        
        // Update latency (rolling average)
        if let Some(new_latency) = latency_ms {
            stats.average_latency_ms = Some(match stats.average_latency_ms {
                Some(avg) => (avg + new_latency) / 2,
                None => new_latency,
            });
        }
        
        // Update last seen
        stats.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.save_relay_stats(&stats)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_peer_id() -> PeerId {
        libp2p::identity::Keypair::generate_ed25519().public().to_peer_id()
    }
    
    #[test]
    fn test_relay_stats_creation() {
        let peer_id = create_test_peer_id();
        let stats = RelayStats::new(peer_id);
        
        assert_eq!(stats.successful_circuits, 0);
        assert_eq!(stats.failed_circuits, 0);
        assert_eq!(stats.uptime_percentage(), 100.0);
    }
    
    #[test]
    fn test_uptime_calculation() {
        let peer_id = create_test_peer_id();
        let mut stats = RelayStats::new(peer_id);
        
        stats.successful_circuits = 7;
        stats.failed_circuits = 3;
        
        assert_eq!(stats.uptime_percentage(), 70.0);
    }
    
    #[test]
    fn test_reputation_score() {
        let peer_id = create_test_peer_id();
        let mut stats = RelayStats::new(peer_id);
        
        stats.successful_circuits = 100;
        stats.failed_circuits = 0;
        stats.total_bytes_relayed = 1024 * 1024 * 1024; // 1 GB
        
        let score = stats.reputation_score();
        assert!(score >= 80.0, "High-performing relay should have score >= 80");
    }
    
    #[test]
    fn test_stale_detection() {
        let peer_id = create_test_peer_id();
        let mut stats = RelayStats::new(peer_id);
        
        // Fresh relay
        assert!(!stats.is_stale());
        
        // Old relay (8 days ago)
        stats.last_seen -= 8 * 24 * 3600;
        assert!(stats.is_stale());
    }
    
    #[test]
    fn test_save_and_load() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let peer_id = create_test_peer_id();
        let mut stats = RelayStats::new(peer_id);
        stats.successful_circuits = 42;
        stats.total_bytes_relayed = 1000000;
        
        storage.save_relay_stats(&stats)?;
        
        let loaded = storage.load_relay_stats(&peer_id)?;
        assert!(loaded.is_some());
        
        let loaded_stats = loaded.unwrap();
        assert_eq!(loaded_stats.successful_circuits, 42);
        assert_eq!(loaded_stats.total_bytes_relayed, 1000000);
        
        Ok(())
    }
    
    #[test]
    fn test_top_relays() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        // Create 3 relays with different reputations
        for i in 0..3 {
            let peer_id = create_test_peer_id();
            let mut stats = RelayStats::new(peer_id);
            stats.successful_circuits = (i + 1) * 10;
            stats.total_bytes_relayed = (i + 1) as u64 * 1000000;
            storage.save_relay_stats(&stats)?;
        }
        
        let top = storage.get_top_relays(2)?;
        assert_eq!(top.len(), 2);
        
        // Should be sorted by reputation
        assert!(top[0].reputation_score() >= top[1].reputation_score());
        
        Ok(())
    }
    
    #[test]
    fn test_prune_stale() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        // Create fresh relay
        let peer1 = create_test_peer_id();
        let stats1 = RelayStats::new(peer1);
        storage.save_relay_stats(&stats1)?;
        
        // Create stale relay
        let peer2 = create_test_peer_id();
        let mut stats2 = RelayStats::new(peer2);
        stats2.last_seen -= 8 * 24 * 3600; // 8 days ago
        storage.save_relay_stats(&stats2)?;
        
        let pruned = storage.prune_stale_relays()?;
        assert_eq!(pruned, 1, "Should prune 1 stale relay");
        
        // Fresh relay should still be there
        assert!(storage.load_relay_stats(&peer1)?.is_some());
        
        // Stale relay should be gone
        assert!(storage.load_relay_stats(&peer2)?.is_none());
        
        Ok(())
    }
    
    #[test]
    fn test_record_circuit_result() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;
        
        let peer_id = create_test_peer_id();
        
        // Record success
        storage.record_circuit_result(&peer_id, true, 5000, Some(50))?;
        
        let stats = storage.load_relay_stats(&peer_id)?.unwrap();
        assert_eq!(stats.successful_circuits, 1);
        assert_eq!(stats.total_bytes_relayed, 5000);
        assert_eq!(stats.average_latency_ms, Some(50));
        
        // Record failure
        storage.record_circuit_result(&peer_id, false, 0, None)?;
        
        let stats = storage.load_relay_stats(&peer_id)?.unwrap();
        assert_eq!(stats.successful_circuits, 1);
        assert_eq!(stats.failed_circuits, 1);
        assert_eq!(stats.uptime_percentage(), 50.0);
        
        Ok(())
    }
}
