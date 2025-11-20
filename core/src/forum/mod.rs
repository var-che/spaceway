//! Space, Channel, and Thread management
//!
//! Implements the hierarchical structure:
//! - Space: Top-level container with MLS group
//! - Channel: Topic-based container within a Space
//! - Thread: Discussion thread within a Channel

pub mod space;
pub mod space_metadata;
pub mod channel;
pub mod thread;

pub use space::{Space, SpaceManager};
pub use space_metadata::{SpaceMetadata, EncryptedSpaceMetadata};
pub use channel::{Channel, ChannelManager};
pub use thread::{Thread, Message, ThreadManager};
