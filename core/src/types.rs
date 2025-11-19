//! Core types and identifiers used throughout the system

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// User identity (Ed25519 public key)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize)]
#[cbor(transparent)]
pub struct UserId(#[b(0)] pub [u8; 32]);

impl UserId {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UserId({})", hex::encode(&self.0[..8]))
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

/// Device identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct DeviceId(pub Uuid);

/// Space identifier (community/server)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct SpaceId(pub Uuid);

/// Channel identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ChannelId(pub Uuid);

/// Thread identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct ThreadId(pub Uuid);

/// Post identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct PostId(pub Uuid);

/// Operation identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct OpId(pub Uuid);

impl OpId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OpId {
    fn default() -> Self {
        Self::new()
    }
}

/// MLS epoch identifier (monotonically increasing)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(transparent)]
pub struct EpochId(#[n(0)] pub u64);

impl EpochId {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn increment(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Content hash (Blake3)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Encode, Decode, Serialize, Deserialize)]
#[cbor(transparent)]
pub struct ContentHash(#[b(0)] pub [u8; 32]);

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", hex::encode(&self.0[..8]))
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..8]))
    }
}

/// User role within a space or channel
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(index_only)]
pub enum Role {
    #[n(0)]
    Admin,
    #[n(1)]
    Moderator,
    #[n(2)]
    Member,
}

impl Role {
    /// Check if this role has moderation privileges
    pub fn can_moderate(&self) -> bool {
        matches!(self, Role::Admin | Role::Moderator)
    }

    /// Check if this role is admin
    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }

    /// Compare roles for precedence (Admin > Moderator > Member)
    pub fn precedence(&self) -> u8 {
        match self {
            Role::Admin => 2,
            Role::Moderator => 1,
            Role::Member => 0,
        }
    }
}

/// Signature bytes (Ed25519)
#[derive(Clone, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl serde::Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(&self.0[..], serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("signature must be 64 bytes"));
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(&bytes);
        Ok(Signature(sig))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({}...)", hex::encode(&self.0[..8]))
    }
}

// Manual CBOR implementations for Uuid-based types
impl<C> Encode<C> for DeviceId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for DeviceId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(DeviceId(uuid))
    }
}

impl<C> Encode<C> for SpaceId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for SpaceId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(SpaceId(uuid))
    }
}

impl<C> Encode<C> for ChannelId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ChannelId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(ChannelId(uuid))
    }
}

impl<C> Encode<C> for ThreadId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for ThreadId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(ThreadId(uuid))
    }
}

impl<C> Encode<C> for PostId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for PostId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(PostId(uuid))
    }
}

impl<C> Encode<C> for OpId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for OpId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(OpId(uuid))
    }
}

impl<C> Encode<C> for Signature {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Signature {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        if bytes.len() != 64 {
            return Err(minicbor::decode::Error::message("signature must be 64 bytes"));
        }
        let mut sig = [0u8; 64];
        sig.copy_from_slice(bytes);
        Ok(Signature(sig))
    }
}

// Helper module for hex encoding in Display/Debug
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
