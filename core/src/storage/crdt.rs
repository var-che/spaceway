/// CRDT state management for offline synchronization
/// 
/// Implements vector clocks and tombstones for causal ordering
/// and conflict-free message deletion.

use super::Storage;
use crate::types::{ThreadId, UserId, MessageId};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};

/// Vector clock for causal ordering
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    /// Map of user_id -> message counter
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    /// Create new empty vector clock
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Get counter for a user
    pub fn get(&self, user_id: &UserId) -> u64 {
        self.clocks.get(&user_id.to_string()).copied().unwrap_or(0)
    }

    /// Increment counter for a user
    pub fn increment(&mut self, user_id: &UserId) {
        let key = user_id.to_string();
        let counter = self.clocks.entry(key).or_insert(0);
        *counter += 1;
    }

    /// Check if this clock happens-before another
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut strictly_less = false;

        for (user, &count) in &self.clocks {
            let other_count = other.clocks.get(user).copied().unwrap_or(0);
            if count > other_count {
                return false; // Not happens-before
            }
            if count < other_count {
                strictly_less = true;
            }
        }

        // Check for users in other but not in self
        for user in other.clocks.keys() {
            if !self.clocks.contains_key(user) {
                strictly_less = true;
            }
        }

        strictly_less
    }

    /// Check if two clocks are concurrent (neither happens-before the other)
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    /// Merge two vector clocks (take max of each counter)
    pub fn merge(&mut self, other: &VectorClock) {
        for (user, &count) in &other.clocks {
            let entry = self.clocks.entry(user.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }
}

/// Tombstone set for deleted messages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TombstoneSet {
    /// Set of deleted message IDs
    pub tombstones: HashSet<String>,
}

impl TombstoneSet {
    /// Create new empty tombstone set
    pub fn new() -> Self {
        Self {
            tombstones: HashSet::new(),
        }
    }

    /// Add a tombstone
    pub fn add(&mut self, message_id: &MessageId) {
        self.tombstones.insert(message_id.to_string());
    }

    /// Check if a message is deleted
    pub fn contains(&self, message_id: &MessageId) -> bool {
        self.tombstones.contains(&message_id.to_string())
    }

    /// Merge another tombstone set
    pub fn merge(&mut self, other: &TombstoneSet) {
        self.tombstones.extend(other.tombstones.iter().cloned());
    }
}

impl Storage {
    /// Get vector clock for a thread
    pub fn get_vector_clock(&self, thread_id: &ThreadId) -> Result<VectorClock> {
        let cf = self.db.cf_handle(Self::CF_VECTOR_CLOCKS)
            .context("Missing vector_clocks column family")?;

        let key = format!("{}:vector_clock", thread_id.to_string());
        let value = self.db.get_cf(&cf, key.as_bytes())
            .context("Failed to read vector clock")?;

        match value {
            Some(bytes) => {
                bincode::deserialize(&bytes)
                    .context("Failed to deserialize vector clock")
            }
            None => Ok(VectorClock::new()),
        }
    }

    /// Update vector clock for a thread
    pub fn update_vector_clock(&self, thread_id: &ThreadId, clock: &VectorClock) -> Result<()> {
        let cf = self.db.cf_handle(Self::CF_VECTOR_CLOCKS)
            .context("Missing vector_clocks column family")?;

        let key = format!("{}:vector_clock", thread_id.to_string());
        let value = bincode::serialize(clock)
            .context("Failed to serialize vector clock")?;

        self.db.put_cf(&cf, key.as_bytes(), &value)
            .context("Failed to write vector clock")?;

        Ok(())
    }

    /// Increment vector clock for a user in a thread
    pub fn increment_vector_clock(&self, thread_id: &ThreadId, user_id: &UserId) -> Result<VectorClock> {
        let mut clock = self.get_vector_clock(thread_id)?;
        clock.increment(user_id);
        self.update_vector_clock(thread_id, &clock)?;
        Ok(clock)
    }

    /// Get tombstone set for a thread
    pub fn get_tombstones(&self, thread_id: &ThreadId) -> Result<TombstoneSet> {
        let cf = self.db.cf_handle(Self::CF_TOMBSTONES)
            .context("Missing tombstones column family")?;

        let key = format!("{}:tombstones", thread_id.to_string());
        let value = self.db.get_cf(&cf, key.as_bytes())
            .context("Failed to read tombstones")?;

        match value {
            Some(bytes) => {
                bincode::deserialize(&bytes)
                    .context("Failed to deserialize tombstones")
            }
            None => Ok(TombstoneSet::new()),
        }
    }

    /// Update tombstone set for a thread
    pub fn update_tombstones(&self, thread_id: &ThreadId, tombstones: &TombstoneSet) -> Result<()> {
        let cf = self.db.cf_handle(Self::CF_TOMBSTONES)
            .context("Missing tombstones column family")?;

        let key = format!("{}:tombstones", thread_id.to_string());
        let value = bincode::serialize(tombstones)
            .context("Failed to serialize tombstones")?;

        self.db.put_cf(&cf, key.as_bytes(), &value)
            .context("Failed to write tombstones")?;

        Ok(())
    }

    /// Add a tombstone (mark message as deleted)
    pub fn add_tombstone(&self, thread_id: &ThreadId, message_id: &MessageId) -> Result<()> {
        let mut tombstones = self.get_tombstones(thread_id)?;
        tombstones.add(message_id);
        self.update_tombstones(thread_id, &tombstones)?;

        tracing::info!(
            thread_id = %thread_id,
            message_id = %message_id,
            "Added tombstone"
        );

        Ok(())
    }

    /// Check if a message is deleted
    pub fn is_deleted(&self, thread_id: &ThreadId, message_id: &MessageId) -> Result<bool> {
        let tombstones = self.get_tombstones(thread_id)?;
        Ok(tombstones.contains(message_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_vector_clock_happens_before() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        let alice = UserId([1u8; 32]);
        let bob = UserId([2u8; 32]);

        // clock1: {alice: 1, bob: 0}
        clock1.increment(&alice);

        // clock2: {alice: 1, bob: 1}
        clock2.increment(&alice);
        clock2.increment(&bob);

        // clock1 happens-before clock2
        assert!(clock1.happens_before(&clock2));
        assert!(!clock2.happens_before(&clock1));
    }

    #[test]
    fn test_vector_clock_concurrent() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        let alice = UserId([1u8; 32]);
        let bob = UserId([2u8; 32]);

        // clock1: {alice: 1, bob: 0}
        clock1.increment(&alice);

        // clock2: {alice: 0, bob: 1}
        clock2.increment(&bob);

        // Concurrent (neither happens-before the other)
        assert!(clock1.is_concurrent(&clock2));
        assert!(clock2.is_concurrent(&clock1));
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        let alice = UserId([1u8; 32]);
        let bob = UserId([2u8; 32]);

        clock1.increment(&alice);
        clock1.increment(&alice);

        clock2.increment(&bob);

        // Merge clock2 into clock1
        clock1.merge(&clock2);

        // Result: {alice: 2, bob: 1}
        assert_eq!(clock1.get(&alice), 2);
        assert_eq!(clock1.get(&bob), 1);
    }

    #[test]
    fn test_storage_vector_clock() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let alice = UserId([1u8; 32]);

        // Increment clock
        let clock = storage.increment_vector_clock(&thread_id, &alice)?;
        assert_eq!(clock.get(&alice), 1);

        // Increment again
        let clock = storage.increment_vector_clock(&thread_id, &alice)?;
        assert_eq!(clock.get(&alice), 2);

        // Retrieve clock
        let stored_clock = storage.get_vector_clock(&thread_id)?;
        assert_eq!(stored_clock.get(&alice), 2);

        Ok(())
    }

    #[test]
    fn test_tombstone_set() {
        let mut tombstones = TombstoneSet::new();
        let msg1 = MessageId::new();
        let msg2 = MessageId::new();

        tombstones.add(&msg1);
        assert!(tombstones.contains(&msg1));
        assert!(!tombstones.contains(&msg2));

        tombstones.add(&msg2);
        assert!(tombstones.contains(&msg2));
    }

    #[test]
    fn test_storage_tombstones() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open(temp_dir.path())?;

        let thread_id = ThreadId::new();
        let msg1 = MessageId::new();
        let msg2 = MessageId::new();

        // Add tombstone
        storage.add_tombstone(&thread_id, &msg1)?;

        // Check deletion status
        assert!(storage.is_deleted(&thread_id, &msg1)?);
        assert!(!storage.is_deleted(&thread_id, &msg2)?);

        // Add another tombstone
        storage.add_tombstone(&thread_id, &msg2)?;
        assert!(storage.is_deleted(&thread_id, &msg2)?);

        Ok(())
    }

    #[test]
    fn test_tombstone_merge() {
        let mut set1 = TombstoneSet::new();
        let mut set2 = TombstoneSet::new();

        let msg1 = MessageId::new();
        let msg2 = MessageId::new();

        set1.add(&msg1);
        set2.add(&msg2);

        set1.merge(&set2);

        assert!(set1.contains(&msg1));
        assert!(set1.contains(&msg2));
    }
}
