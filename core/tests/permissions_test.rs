use descord_core::{Permissions, Role, PermissionResult};
use anyhow::Result;

/// Test basic role-based permissions
#[test]
fn test_role_based_permissions() {
    // Admin has all permissions
    let admin = Permissions::from_role(Role::Admin);
    assert!(admin.is_administrator());
    assert!(admin.can_view_channel());
    assert!(admin.can_send_messages());
    assert!(admin.can_kick_members());
    assert!(admin.can_ban_members());
    assert!(admin.can_manage_roles());
    assert!(admin.can_manage_channels());
    assert!(admin.can_create_invites());

    // Moderator has moderation permissions
    let moderator = Permissions::from_role(Role::Moderator);
    assert!(!moderator.is_administrator());
    assert!(moderator.can_view_channel());
    assert!(moderator.can_send_messages());
    assert!(moderator.can_kick_members());
    assert!(moderator.can_ban_members());
    assert!(!moderator.can_manage_roles());
    assert!(!moderator.can_manage_channels());
    assert!(moderator.can_create_invites());

    // Member has basic permissions only
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

/// Test administrator bypass
#[test]
fn test_administrator_bypass() {
    let admin = Permissions::ADMIN;
    
    // Administrator should bypass any permission check
    assert!(admin.has(|p| p.view_channel));
    assert!(admin.has(|p| p.send_messages));
    assert!(admin.has(|_| false)); // Even false checks pass for admin
}

/// Test permission constants
#[test]
fn test_permission_constants() {
    // NONE
    let none = Permissions::NONE;
    assert!(!none.can_view_channel());
    assert!(!none.can_send_messages());
    assert!(!none.is_administrator());

    // ALL
    let all = Permissions::ALL;
    assert!(all.can_view_channel());
    assert!(all.can_send_messages());
    assert!(all.can_kick_members());
    assert!(all.can_ban_members());
    assert!(all.can_manage_roles());
    assert!(all.can_manage_channels());
    assert!(all.can_create_invites());
    assert!(all.is_administrator());

    // MEMBER
    let member = Permissions::MEMBER;
    assert!(member.can_view_channel());
    assert!(member.can_send_messages());
    assert!(!member.can_kick_members());

    // MODERATOR
    let moderator = Permissions::MODERATOR;
    assert!(moderator.can_view_channel());
    assert!(moderator.can_send_messages());
    assert!(moderator.can_kick_members());
    assert!(moderator.can_ban_members());
    assert!(!moderator.can_manage_roles());

    // ADMIN
    let admin = Permissions::ADMIN;
    assert_eq!(admin, Permissions::ALL);
}

/// Test permission merging
#[test]
fn test_permission_merge() {
    let member = Permissions::MEMBER;
    let moderator = Permissions::MODERATOR;
    
    // Merge should combine permissions (union)
    let merged = member.merge(&moderator);
    assert!(merged.can_view_channel());
    assert!(merged.can_send_messages());
    assert!(merged.can_kick_members());
    assert!(merged.can_ban_members());
    assert!(merged.can_create_invites());
    assert!(!merged.can_manage_roles());
    assert!(!merged.can_manage_channels());
    assert!(!merged.is_administrator());
}

/// Test permission intersection
#[test]
fn test_permission_intersect() {
    let moderator = Permissions::MODERATOR;
    let admin = Permissions::ADMIN;
    
    // Intersect should only keep common permissions
    let intersected = moderator.intersect(&admin);
    assert!(intersected.can_view_channel());
    assert!(intersected.can_send_messages());
    assert!(intersected.can_kick_members());
    assert!(intersected.can_ban_members());
    assert!(intersected.can_create_invites());
    assert!(!intersected.can_manage_roles()); // Moderator doesn't have this
    assert!(!intersected.can_manage_channels()); // Moderator doesn't have this
    assert!(!intersected.is_administrator()); // Moderator is not admin
}

/// Test permission serialization
#[test]
fn test_permission_serialization() -> Result<()> {
    let original = Permissions::MODERATOR;
    
    // Serialize to JSON
    let json = serde_json::to_string(&original)?;
    
    // Deserialize back
    let deserialized: Permissions = serde_json::from_str(&json)?;
    
    assert_eq!(original, deserialized);
    Ok(())
}

/// Test cryptographic enforcement scenario
#[test]
fn test_cryptographic_enforcement() {
    // Scenario: User tries to send message but doesn't have permission
    let mut no_send_perm = Permissions::MEMBER;
    no_send_perm.send_messages = false;
    
    // Check should fail
    assert!(!no_send_perm.can_send_messages());
    
    // Scenario: User is kicked (MLS remove) - loses view_channel permission
    let mut kicked_user = Permissions::MEMBER;
    kicked_user.view_channel = false;
    
    // Can't view channel after kick
    assert!(!kicked_user.can_view_channel());
    
    // Scenario: Admin can do anything
    let admin = Permissions::ADMIN;
    assert!(admin.can_view_channel());
    assert!(admin.can_send_messages());
    assert!(admin.can_kick_members());
}

/// Test moderation permissions
#[test]
fn test_moderation_permissions() {
    let admin = Permissions::from_role(Role::Admin);
    let moderator = Permissions::from_role(Role::Moderator);
    let member = Permissions::from_role(Role::Member);
    
    // Admin can kick and ban
    assert!(admin.can_kick_members());
    assert!(admin.can_ban_members());
    
    // Moderator can kick and ban
    assert!(moderator.can_kick_members());
    assert!(moderator.can_ban_members());
    
    // Member cannot kick or ban
    assert!(!member.can_kick_members());
    assert!(!member.can_ban_members());
    
    // Only admin can manage roles
    assert!(admin.can_manage_roles());
    assert!(!moderator.can_manage_roles());
    assert!(!member.can_manage_roles());
}

/// Test channel management permissions
#[test]
fn test_channel_management_permissions() {
    let admin = Permissions::from_role(Role::Admin);
    let moderator = Permissions::from_role(Role::Moderator);
    let member = Permissions::from_role(Role::Member);
    
    // Only admin can manage channels
    assert!(admin.can_manage_channels());
    assert!(!moderator.can_manage_channels());
    assert!(!member.can_manage_channels());
}

/// Test invite creation permissions
#[test]
fn test_invite_permissions() {
    let admin = Permissions::from_role(Role::Admin);
    let moderator = Permissions::from_role(Role::Moderator);
    let member = Permissions::from_role(Role::Member);
    
    // Admin and moderator can create invites
    assert!(admin.can_create_invites());
    assert!(moderator.can_create_invites());
    
    // Members cannot create invites by default
    assert!(!member.can_create_invites());
}

/// Test default permissions
#[test]
fn test_default_permissions() {
    let default = Permissions::default();
    
    // Default should be member permissions
    assert_eq!(default, Permissions::MEMBER);
    assert!(default.can_view_channel());
    assert!(default.can_send_messages());
    assert!(!default.can_kick_members());
}

/// Test role precedence
#[test]
fn test_role_precedence() {
    assert!(Role::Admin.precedence() > Role::Moderator.precedence());
    assert!(Role::Moderator.precedence() > Role::Member.precedence());
    
    assert!(Role::Admin.is_admin());
    assert!(!Role::Moderator.is_admin());
    assert!(!Role::Member.is_admin());
    
    assert!(Role::Admin.can_moderate());
    assert!(Role::Moderator.can_moderate());
    assert!(!Role::Member.can_moderate());
}

/// Test permission edge cases
#[test]
fn test_permission_edge_cases() {
    // Empty permissions (all false)
    let none = Permissions::NONE;
    assert!(!none.can_view_channel());
    assert!(!none.can_send_messages());
    
    // Custom permissions
    let custom = Permissions {
        view_channel: true,
        send_messages: false,
        kick_members: true,
        ban_members: false,
        manage_roles: false,
        manage_channels: false,
        create_invites: true,
        administrator: false,
    };
    
    assert!(custom.can_view_channel());
    assert!(!custom.can_send_messages());
    assert!(custom.can_kick_members());
    assert!(!custom.can_ban_members());
    assert!(custom.can_create_invites());
}
