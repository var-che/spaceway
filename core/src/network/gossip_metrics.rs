/// GossipSub metrics and monitoring
///
/// Tracks message propagation, peer connectivity, and mesh health

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Metrics for a specific topic
#[derive(Debug, Clone)]
pub struct TopicMetrics {
    /// Topic name
    pub topic: String,
    
    /// Number of messages published
    pub messages_published: u64,
    
    /// Number of messages received
    pub messages_received: u64,
    
    /// Number of duplicate messages received
    pub duplicates_received: u64,
    
    /// Number of peers in mesh
    pub mesh_peers: usize,
    
    /// Average propagation latency (if timestamps available)
    pub avg_latency_ms: f64,
    
    /// Last activity timestamp
    pub last_activity: Instant,
}

impl TopicMetrics {
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            messages_published: 0,
            messages_received: 0,
            duplicates_received: 0,
            mesh_peers: 0,
            avg_latency_ms: 0.0,
            last_activity: Instant::now(),
        }
    }
}

/// GossipSub monitoring and statistics
#[derive(Debug, Clone)]
pub struct GossipMetrics {
    metrics: Arc<RwLock<HashMap<String, TopicMetrics>>>,
}

impl GossipMetrics {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Record a message published
    pub async fn record_publish(&self, topic: &str) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(topic.to_string())
            .or_insert_with(|| TopicMetrics::new(topic.to_string()));
        entry.messages_published += 1;
        entry.last_activity = Instant::now();
    }
    
    /// Record a message received
    pub async fn record_receive(&self, topic: &str, is_duplicate: bool) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(topic.to_string())
            .or_insert_with(|| TopicMetrics::new(topic.to_string()));
        
        if is_duplicate {
            entry.duplicates_received += 1;
        } else {
            entry.messages_received += 1;
        }
        entry.last_activity = Instant::now();
    }
    
    /// Update mesh peer count for a topic
    pub async fn update_mesh_peers(&self, topic: &str, peer_count: usize) {
        let mut metrics = self.metrics.write().await;
        let entry = metrics.entry(topic.to_string())
            .or_insert_with(|| TopicMetrics::new(topic.to_string()));
        entry.mesh_peers = peer_count;
    }
    
    /// Get metrics for a specific topic
    pub async fn get_topic_metrics(&self, topic: &str) -> Option<TopicMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(topic).cloned()
    }
    
    /// Get all topic metrics
    pub async fn get_all_metrics(&self) -> Vec<TopicMetrics> {
        let metrics = self.metrics.read().await;
        metrics.values().cloned().collect()
    }
    
    /// Print statistics summary
    pub async fn print_summary(&self) {
        let metrics = self.metrics.read().await;
        
        println!("\n📊 GossipSub Metrics Summary:");
        println!("  Total topics: {}", metrics.len());
        
        for (topic, stats) in metrics.iter() {
            println!("\n  Topic: {}", topic);
            println!("    Published: {} msgs", stats.messages_published);
            println!("    Received: {} msgs", stats.messages_received);
            println!("    Duplicates: {} msgs", stats.duplicates_received);
            println!("    Mesh peers: {}", stats.mesh_peers);
            println!("    Last activity: {:?} ago", stats.last_activity.elapsed());
        }
    }
    
    /// Clear old topic metrics (cleanup)
    pub async fn cleanup_old_metrics(&self, max_age: Duration) {
        let mut metrics = self.metrics.write().await;
        let now = Instant::now();
        
        metrics.retain(|_, stats| {
            now.duration_since(stats.last_activity) < max_age
        });
    }
}

impl Default for GossipMetrics {
    fn default() -> Self {
        Self::new()
    }
}
