//! Hybrid Logical Clock (HLC) implementation
//!
//! HLC provides a logical timestamp that combines wall-clock time with a logical counter,
//! ensuring causal ordering without relying solely on potentially-skewed system clocks.

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hybrid Logical Clock timestamp
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize, Debug)]
pub struct Hlc {
    /// Wall-clock time in milliseconds since UNIX epoch
    #[n(0)]
    pub wall_time: u64,
    
    /// Logical counter for ordering events at the same wall time
    #[n(1)]
    pub logical: u64,
}

impl Hlc {
    /// Create a new HLC with current wall time and zero logical counter
    pub fn now() -> Self {
        let wall_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        Self {
            wall_time,
            logical: 0,
        }
    }

    /// Update HLC based on receiving a remote timestamp
    ///
    /// This implements the HLC update rules:
    /// - If remote.wall_time > local.wall_time: use remote.wall_time, logical = remote.logical + 1
    /// - If remote.wall_time == local.wall_time: keep wall_time, logical = max(local, remote) + 1
    /// - If remote.wall_time < local.wall_time: use local.wall_time, logical = local.logical + 1
    pub fn update(&mut self, remote: Hlc) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        if remote.wall_time > self.wall_time && remote.wall_time > now {
            // Remote is ahead of both local clock and wall clock
            self.wall_time = remote.wall_time;
            self.logical = remote.logical + 1;
        } else if remote.wall_time > self.wall_time {
            // Remote is ahead of local but not wall clock
            self.wall_time = now;
            self.logical = remote.logical + 1;
        } else if remote.wall_time == self.wall_time {
            // Same wall time, increment logical counter
            self.wall_time = now.max(self.wall_time);
            self.logical = self.logical.max(remote.logical) + 1;
        } else {
            // Local is ahead, just increment logical counter
            self.wall_time = now.max(self.wall_time);
            self.logical += 1;
        }
    }

    /// Create a new HLC by incrementing this one
    pub fn tick(&self) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        if now > self.wall_time {
            Self {
                wall_time: now,
                logical: 0,
            }
        } else {
            Self {
                wall_time: self.wall_time,
                logical: self.logical + 1,
            }
        }
    }
}

/// Thread-safe HLC generator
pub struct HlcGenerator {
    last: AtomicU64,
}

impl HlcGenerator {
    pub fn new() -> Self {
        Self {
            last: AtomicU64::new(0),
        }
    }

    /// Generate a new HLC timestamp
    pub fn generate(&self) -> Hlc {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        let packed_now = (now << 16) | 0;
        let last = self.last.load(Ordering::Relaxed);

        let packed_new = if packed_now > last {
            packed_now
        } else {
            last + 1
        };

        self.last.store(packed_new, Ordering::Relaxed);

        Hlc {
            wall_time: packed_new >> 16,
            logical: packed_new & 0xFFFF,
        }
    }

    /// Update the generator with a received HLC and return new timestamp
    pub fn update(&self, remote: Hlc) -> Hlc {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis() as u64;

        let packed_now = (now << 16) | 0;
        let packed_remote = (remote.wall_time << 16) | (remote.logical & 0xFFFF);
        let last = self.last.load(Ordering::Relaxed);

        let packed_new = packed_now.max(packed_remote).max(last) + 1;
        self.last.store(packed_new, Ordering::Relaxed);

        Hlc {
            wall_time: packed_new >> 16,
            logical: packed_new & 0xFFFF,
        }
    }
}

impl Default for HlcGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_ordering() {
        let hlc1 = Hlc { wall_time: 1000, logical: 0 };
        let hlc2 = Hlc { wall_time: 1000, logical: 1 };
        let hlc3 = Hlc { wall_time: 1001, logical: 0 };

        assert!(hlc1 < hlc2);
        assert!(hlc2 < hlc3);
        assert!(hlc1 < hlc3);
    }

    #[test]
    fn test_hlc_tick() {
        let hlc = Hlc::now();
        let ticked = hlc.tick();
        
        assert!(ticked >= hlc);
    }

    #[test]
    fn test_hlc_generator() {
        let gen = HlcGenerator::new();
        
        let hlc1 = gen.generate();
        let hlc2 = gen.generate();
        
        assert!(hlc2 > hlc1);
    }
}
