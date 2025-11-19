//! CRDT (Conflict-free Replicated Data Types) module
//!
//! This module implements the core CRDT operations for Descord, including:
//! - Hybrid Logical Clocks (HLC) for causal ordering
//! - CRDT operation types and envelopes
//! - Operation validation and acceptance logic
//! - Deterministic conflict resolution
//! - Causal dependency tracking

pub mod hlc;
pub mod ops;
pub mod validator;

pub use hlc::Hlc;
pub use ops::{CrdtOp, OpPayload, OpType};
pub use validator::{OpValidator, ValidationResult, RejectionReason};
