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

impl ThreadId {
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Post identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct PostId(pub Uuid);

/// Message identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

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

/// Space visibility and discoverability
#[derive(Clone, Copy, PartialEq, Eq, Hash, Encode, Decode, Serialize, Deserialize, Debug)]
#[cbor(index_only)]
pub enum SpaceVisibility {
    /// Public: Listed in public directory, anyone can join
    #[n(0)]
    Public,
    /// Private: Invite-only, not publicly listed
    #[n(1)]
    Private,
    /// Hidden: Maximum privacy, requires cryptographic invitation
    #[n(2)]
    Hidden,
}

impl Default for SpaceVisibility {
    fn default() -> Self {
        SpaceVisibility::Private
    }
}

impl SpaceVisibility {
    /// Whether this space is discoverable via public directory
    pub fn is_discoverable(&self) -> bool {
        matches!(self, SpaceVisibility::Public)
    }

    /// Whether this space requires invitation to join
    pub fn requires_invite(&self) -> bool {
        !matches!(self, SpaceVisibility::Public)
    }

    /// Whether this space is hidden (maximum privacy)
    pub fn is_hidden(&self) -> bool {
        matches!(self, SpaceVisibility::Hidden)
    }

    /// Get the required network transport mode for this visibility level
    pub fn transport_mode(&self) -> NetworkTransportMode {
        match self {
            SpaceVisibility::Public => NetworkTransportMode::Direct,
            SpaceVisibility::Private => NetworkTransportMode::Relay,
            SpaceVisibility::Hidden => NetworkTransportMode::Relay,
        }
    }

    /// Get user-facing privacy warning for this visibility level
    pub fn privacy_warning(&self) -> &'static str {
        match self {
            SpaceVisibility::Public => 
                "⚠️ PUBLIC SPACE - Your IP address will be visible to other members. \
                 This space prioritizes convenience and performance over privacy.",
            SpaceVisibility::Private => 
                "🔒 PRIVATE SPACE - Your IP address is hidden via relay network. \
                 Slightly higher latency (~50-100ms) for better privacy.",
            SpaceVisibility::Hidden => 
                "🔐 HIDDEN SPACE - Maximum privacy via relay network. \
                 Your IP address is hidden. Invite-only access.",
        }
    }

    /// Get privacy level description
    pub fn privacy_level(&self) -> PrivacyLevel {
        match self {
            SpaceVisibility::Public => PrivacyLevel::Low,
            SpaceVisibility::Private => PrivacyLevel::High,
            SpaceVisibility::Hidden => PrivacyLevel::Maximum,
        }
    }
}

/// Network transport mode for connections
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Encode, Decode, Serialize, Deserialize)]
#[cbor(index_only)]
pub enum NetworkTransportMode {
    /// Direct P2P connections (fast, but exposes IP addresses)
    #[n(0)]
    Direct,
    /// Relay-based connections (hides IP, adds latency)
    #[n(1)]
    Relay,
    /// Tor-based connections (maximum privacy, highest latency) - future
    #[n(2)]
    Tor,
}

/// Privacy level indicator
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PrivacyLevel {
    /// Low privacy: IP exposed, fast performance
    Low,
    /// High privacy: IP hidden via relay
    High,
    /// Maximum privacy: IP hidden, metadata minimized
    Maximum,
}

/// Privacy information for user consent
#[derive(Clone, Debug)]
pub struct PrivacyInfo {
    /// Visibility level
    pub visibility: SpaceVisibility,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Transport mode (Direct/Relay/Tor)
    pub transport_mode: NetworkTransportMode,
    /// User-facing warning message
    pub warning: String,
    /// What data is exposed
    pub exposed_data: Vec<String>,
    /// What data is protected
    pub protected_data: Vec<String>,
    /// Expected latency impact
    pub latency_ms: std::ops::Range<u32>,
}

impl PrivacyInfo {
    /// Generate privacy information from visibility level
    pub fn from_visibility(visibility: SpaceVisibility) -> Self {
        match visibility {
            SpaceVisibility::Public => Self {
                visibility,
                privacy_level: PrivacyLevel::Low,
                transport_mode: NetworkTransportMode::Direct,
                warning: visibility.privacy_warning().to_string(),
                exposed_data: vec![
                    "Your IP address".to_string(),
                    "Online/offline status".to_string(),
                    "Message timing".to_string(),
                    "Connection patterns".to_string(),
                ],
                protected_data: vec![
                    "Message content (E2E encrypted)".to_string(),
                    "Attachments (E2E encrypted)".to_string(),
                ],
                latency_ms: 10..50,
            },
            SpaceVisibility::Private => Self {
                visibility,
                privacy_level: PrivacyLevel::High,
                transport_mode: NetworkTransportMode::Relay,
                warning: visibility.privacy_warning().to_string(),
                exposed_data: vec![
                    "Online/offline status (to space members)".to_string(),
                    "Message timing".to_string(),
                ],
                protected_data: vec![
                    "Your IP address (hidden via relay)".to_string(),
                    "Message content (E2E encrypted)".to_string(),
                    "Attachments (E2E encrypted)".to_string(),
                    "Connection patterns".to_string(),
                ],
                latency_ms: 50..150,
            },
            SpaceVisibility::Hidden => Self {
                visibility,
                privacy_level: PrivacyLevel::Maximum,
                transport_mode: NetworkTransportMode::Relay,
                warning: visibility.privacy_warning().to_string(),
                exposed_data: vec![
                    "Message timing (to space members)".to_string(),
                ],
                protected_data: vec![
                    "Your IP address (hidden via relay)".to_string(),
                    "Message content (E2E encrypted)".to_string(),
                    "Attachments (E2E encrypted)".to_string(),
                    "Connection patterns".to_string(),
                    "Space existence (invite-only)".to_string(),
                ],
                latency_ms: 50..150,
            },
        }
    }

    /// Format privacy info as user-readable string
    pub fn format_for_user(&self) -> String {
        format!(
            "{}\n\nPrivacy Level: {:?}\nTransport: {:?}\n\n\
             Exposed: {}\nProtected: {}\n\nExpected latency: {}-{}ms",
            self.warning,
            self.privacy_level,
            self.transport_mode,
            self.exposed_data.join(", "),
            self.protected_data.join(", "),
            self.latency_ms.start,
            self.latency_ms.end,
        )
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

/// Invite identifier
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub struct InviteId(pub Uuid);

/// Invite to join a space
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Encode, Decode)]
pub struct Invite {
    /// Unique invite identifier
    #[n(0)]
    pub id: InviteId,
    /// Space this invite is for
    #[n(1)]
    pub space_id: SpaceId,
    /// User who created the invite
    #[n(2)]
    pub creator: UserId,
    /// Short alphanumeric code (e.g., "ABcd123X")
    #[n(3)]
    pub code: String,
    /// Maximum number of uses (None = unlimited)
    #[n(4)]
    pub max_uses: Option<u32>,
    /// Expiration timestamp (None = never expires)
    #[n(5)]
    pub expires_at: Option<u64>,
    /// Current use count
    #[n(6)]
    pub uses: u32,
    /// Creation timestamp
    #[n(7)]
    pub created_at: u64,
    /// Whether this invite is revoked
    #[n(8)]
    pub revoked: bool,
}

impl Invite {
    /// Check if this invite is valid (not expired, not exceeded uses, not revoked)
    pub fn is_valid(&self, current_time: u64) -> bool {
        // Check if revoked
        if self.revoked {
            return false;
        }

        // Check expiration
        if let Some(expires_at) = self.expires_at {
            if current_time >= expires_at {
                return false;
            }
        }

        // Check uses
        if let Some(max_uses) = self.max_uses {
            if self.uses >= max_uses {
                return false;
            }
        }

        true
    }

    /// Check if this invite can be created by the given role
    pub fn can_create(role: Role, permissions: &InvitePermissions) -> bool {
        match permissions.who_can_invite {
            InviteCreatorRole::AdminOnly => role.is_admin(),
            InviteCreatorRole::AdminAndModerator => role.can_moderate(),
            InviteCreatorRole::Everyone => true,
        }
    }
}

/// Who can create invites in a space
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Encode, Decode)]
#[cbor(index_only)]
pub enum InviteCreatorRole {
    /// Only admins can create invites
    #[n(0)]
    AdminOnly,
    /// Admins and moderators can create invites
    #[n(1)]
    AdminAndModerator,
    /// Everyone can create invites
    #[n(2)]
    Everyone,
}

impl Default for InviteCreatorRole {
    fn default() -> Self {
        InviteCreatorRole::AdminAndModerator
    }
}

/// Invite permissions for a space
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug, Encode, Decode)]
pub struct InvitePermissions {
    /// Who can create invites
    #[n(0)]
    pub who_can_invite: InviteCreatorRole,
    /// Default max age in hours for new invites (None = no default limit)
    #[n(1)]
    pub max_age_hours: Option<u32>,
    /// Default max uses for new invites
    #[n(2)]
    pub max_uses_default: u32,
}

impl Default for InvitePermissions {
    fn default() -> Self {
        InvitePermissions {
            who_can_invite: InviteCreatorRole::AdminAndModerator,
            max_age_hours: Some(24 * 7), // 7 days default
            max_uses_default: 10,
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

impl<C> Encode<C> for MessageId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for MessageId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(MessageId(uuid))
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

impl<C> Encode<C> for InviteId {
    fn encode<W: minicbor::encode::Write>(&self, e: &mut minicbor::Encoder<W>, _ctx: &mut C) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(self.0.as_bytes())?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for InviteId {
    fn decode(d: &mut minicbor::Decoder<'b>, _ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let bytes = d.bytes()?;
        let uuid = Uuid::from_slice(bytes).map_err(|_| minicbor::decode::Error::message("invalid UUID"))?;
        Ok(InviteId(uuid))
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
