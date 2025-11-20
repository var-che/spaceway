//! Permissions and access control module
//!
//! Provides cryptographically-enforced permissions using MLS group membership
//! and role-based access control (RBAC).

use crate::types::Role;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Permission flags for space and channel operations.
///
/// These permissions are cryptographically enforced via MLS:
/// - `view_channel`: Requires MLS group membership to decrypt messages
/// - `send_messages`: Requires valid MLS commit signature
/// - `kick_members`: Requires admin/mod role to create MLS Remove commit
/// - `ban_members`: Requires admin/mod role to add to blacklist + MLS remove
/// - `manage_roles`: Requires admin role to update MLS GroupContext
/// - `manage_channels`: Requires admin role to create/delete MLS groups
/// - `create_invites`: Requires permission to generate signed invite tokens
/// - `administrator`: Bypass all permission checks (admin only)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Encode, Decode, Serialize, Deserialize)]
pub struct Permissions {
    /// Can view channel and decrypt messages (MLS group membership)
    #[n(0)]
    pub view_channel: bool,
    
    /// Can send messages (requires valid MLS commit signature)
    #[n(1)]
    pub send_messages: bool,
    
    /// Can remove members from space (MLS Remove commit + key rotation)
    #[n(2)]
    pub kick_members: bool,
    
    /// Can permanently ban members (blacklist CRDT + MLS remove)
    #[n(3)]
    pub ban_members: bool,
    
    /// Can change user roles (MLS GroupContext update)
    #[n(4)]
    pub manage_roles: bool,
    
    /// Can create and delete channels (create/archive MLS groups)
    #[n(5)]
    pub manage_channels: bool,
    
    /// Can generate invite tokens (signed by admin)
    #[n(6)]
    pub create_invites: bool,
    
    /// Administrator bypass (all permissions granted)
    #[n(7)]
    pub administrator: bool,
}

impl Permissions {
    /// No permissions (default for uninitialized state)
    pub const NONE: Self = Self {
        view_channel: false,
        send_messages: false,
        kick_members: false,
        ban_members: false,
        manage_roles: false,
        manage_channels: false,
        create_invites: false,
        administrator: false,
    };

    /// All permissions (administrator)
    pub const ALL: Self = Self {
        view_channel: true,
        send_messages: true,
        kick_members: true,
        ban_members: true,
        manage_roles: true,
        manage_channels: true,
        create_invites: true,
        administrator: true,
    };

    /// Default member permissions (read + write only)
    pub const MEMBER: Self = Self {
        view_channel: true,
        send_messages: true,
        kick_members: false,
        ban_members: false,
        manage_roles: false,
        manage_channels: false,
        create_invites: false,
        administrator: false,
    };

    /// Default moderator permissions (moderation powers)
    pub const MODERATOR: Self = Self {
        view_channel: true,
        send_messages: true,
        kick_members: true,
        ban_members: true,
        manage_roles: false,
        manage_channels: false,
        create_invites: true,
        administrator: false,
    };

    /// Default admin permissions (full control)
    pub const ADMIN: Self = Self::ALL;

    /// Create permissions from a role
    pub fn from_role(role: Role) -> Self {
        match role {
            Role::Admin => Self::ADMIN,
            Role::Moderator => Self::MODERATOR,
            Role::Member => Self::MEMBER,
        }
    }

    /// Check if user has a specific permission (respects administrator bypass)
    pub fn has(&self, check: impl Fn(&Self) -> bool) -> bool {
        self.administrator || check(self)
    }

    /// Check if user can view channel (MLS membership check)
    pub fn can_view_channel(&self) -> bool {
        self.has(|p| p.view_channel)
    }

    /// Check if user can send messages (MLS signature check)
    pub fn can_send_messages(&self) -> bool {
        self.has(|p| p.send_messages)
    }

    /// Check if user can kick members (moderation action)
    pub fn can_kick_members(&self) -> bool {
        self.has(|p| p.kick_members)
    }

    /// Check if user can ban members (moderation action)
    pub fn can_ban_members(&self) -> bool {
        self.has(|p| p.ban_members)
    }

    /// Check if user can manage roles (admin action)
    pub fn can_manage_roles(&self) -> bool {
        self.has(|p| p.manage_roles)
    }

    /// Check if user can manage channels (admin action)
    pub fn can_manage_channels(&self) -> bool {
        self.has(|p| p.manage_channels)
    }

    /// Check if user can create invites
    pub fn can_create_invites(&self) -> bool {
        self.has(|p| p.create_invites)
    }

    /// Check if user is administrator
    pub fn is_administrator(&self) -> bool {
        self.administrator
    }

    /// Merge permissions (union of two permission sets)
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            view_channel: self.view_channel || other.view_channel,
            send_messages: self.send_messages || other.send_messages,
            kick_members: self.kick_members || other.kick_members,
            ban_members: self.ban_members || other.ban_members,
            manage_roles: self.manage_roles || other.manage_roles,
            manage_channels: self.manage_channels || other.manage_channels,
            create_invites: self.create_invites || other.create_invites,
            administrator: self.administrator || other.administrator,
        }
    }

    /// Intersect permissions (only permissions present in both)
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            view_channel: self.view_channel && other.view_channel,
            send_messages: self.send_messages && other.send_messages,
            kick_members: self.kick_members && other.kick_members,
            ban_members: self.ban_members && other.ban_members,
            manage_roles: self.manage_roles && other.manage_roles,
            manage_channels: self.manage_channels && other.manage_channels,
            create_invites: self.create_invites && other.create_invites,
            administrator: self.administrator && other.administrator,
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::MEMBER
    }
}

/// Permission check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionResult {
    /// Permission granted
    Allowed,
    /// Permission denied with reason
    Denied(String),
}

impl PermissionResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    pub fn is_denied(&self) -> bool {
        !self.is_allowed()
    }

    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            Self::Denied(reason) => Some(reason),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_permissions() {
        let admin = Permissions::from_role(Role::Admin);
        assert!(admin.is_administrator());
        assert!(admin.can_view_channel());
        assert!(admin.can_send_messages());
        assert!(admin.can_kick_members());
        assert!(admin.can_ban_members());
        assert!(admin.can_manage_roles());
        assert!(admin.can_manage_channels());
        assert!(admin.can_create_invites());

        let moderator = Permissions::from_role(Role::Moderator);
        assert!(!moderator.is_administrator());
        assert!(moderator.can_view_channel());
        assert!(moderator.can_send_messages());
        assert!(moderator.can_kick_members());
        assert!(moderator.can_ban_members());
        assert!(!moderator.can_manage_roles());
        assert!(!moderator.can_manage_channels());
        assert!(moderator.can_create_invites());

        let member = Permissions::from_role(Role::Member);
        assert!(!member.is_administrator());
        assert!(member.can_view_channel());
        assert!(member.can_send_messages());
        assert!(!member.can_kick_members());
        assert!(!member.can_ban_members());
        assert!(!member.can_manage_roles());
        assert!(!member.can_manage_channels());
        assert!(!member.can_create_invites());
    }

    #[test]
    fn test_administrator_bypass() {
        let admin = Permissions::ADMIN;
        assert!(admin.has(|_| false)); // Should return true due to administrator bypass
    }

    #[test]
    fn test_permission_merge() {
        let member = Permissions::MEMBER;
        let moderator = Permissions::MODERATOR;
        
        let merged = member.merge(&moderator);
        assert!(merged.can_view_channel());
        assert!(merged.can_send_messages());
        assert!(merged.can_kick_members());
        assert!(merged.can_ban_members());
        assert!(merged.can_create_invites());
        assert!(!merged.can_manage_roles());
        assert!(!merged.can_manage_channels());
    }

    #[test]
    fn test_permission_intersect() {
        let moderator = Permissions::MODERATOR;
        let admin = Permissions::ADMIN;
        
        let intersected = moderator.intersect(&admin);
        assert!(intersected.can_view_channel());
        assert!(intersected.can_send_messages());
        assert!(intersected.can_kick_members());
        assert!(intersected.can_ban_members());
        assert!(intersected.can_create_invites());
        assert!(!intersected.can_manage_roles()); // Mod doesn't have this
        assert!(!intersected.can_manage_channels()); // Mod doesn't have this
        assert!(!intersected.is_administrator()); // Mod is not admin
    }

    #[test]
    fn test_default_permissions() {
        let default = Permissions::default();
        assert_eq!(default, Permissions::MEMBER);
    }
}
