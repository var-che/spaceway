//! SmoothTest - Smooth testing framework for descord
//!
//! Inspired by Holochain's SweetTest, this module provides utilities for testing
//! distributed features (DHT, P2P, MLS) on a single machine with multiple virtual clients.
//!
//! # Architecture
//!
//! - `SmoothClient`: Single test client with isolated storage
//! - `SmoothClientBatch`: Collection of clients that can communicate
//! - Utilities for awaiting DHT consistency, peer discovery, etc.
//!
//! # Example
//!
//! ```no_run
//! use descord_core::smoothtest::*;
//!
//! #[tokio::test]
//! async fn test_dht_with_multiple_nodes() {
//!     let mut batch = SmoothClientBatch::new(3).await;
//!     batch.connect_all().await;
//!     
//!     // Now have 3 clients that can talk to each other
//!     let alice = &batch[0];
//!     let bob = &batch[1];
//!     let carol = &batch[2];
//!     
//!     // Test DHT operations with proper quorum
//!     // ...
//! }
//! ```

mod smooth_client;
mod smooth_batch;
mod consistency;

pub use smooth_client::SmoothClient;
pub use smooth_batch::SmoothClientBatch;
pub use consistency::await_dht_consistency;
