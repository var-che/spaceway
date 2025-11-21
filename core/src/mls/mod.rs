//! MLS (Messaging Layer Security) integration
//!
//! Provides group key management and end-to-end encryption using OpenMLS.
//! Each Space corresponds to one MLS group with epoch-based key rotation.

pub mod group;
pub mod provider;
pub mod keypackage;

pub use group::{MlsGroup, MlsGroupConfig};
pub use provider::DescordProvider;
pub use keypackage::{KeyPackageBundle, KeyPackageStore};
