//! Permission System Tests
//! 
//! Tests for the Discord-Lite permission system including:
//! - Space permissions (create channels, kick members, etc.)
//! - Role hierarchy (can't assign higher roles)
//! - Permission inheritance (default role)
//! - Channel-independent moderation

use spaceway_core::forum::{Space, SpaceManager};
use spaceway_core::types::*;
use spaceway_core::crypto::signing::Keypair;
use spaceway_core::mls::provider::create_provider;

#[test]
fn test_owner_has_all_permissions() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Owner should have all permissions
    assert!(space.can_create_channels(&owner));
    assert!(space.can_delete_channels(&owner));
    assert!(space.can_manage_channels(&owner));
    assert!(space.can_kick_members(&owner));
    assert!(space.can_manage_roles(&owner));
    assert!(space.can_delete_messages(&owner));
    assert!(space.can_invite_members(&owner));
    
    println!("✓ Owner has all permissions");
}

#[test]
fn test_default_role_permissions() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Add a regular member
    let member_keypair = Keypair::generate();
    let member = member_keypair.user_id();
    
    // Assign default role (Member)
    space.assign_role(member, space.default_role).unwrap();
    
    // Member should have basic permissions only
    assert!(!space.can_create_channels(&member), "Members shouldn't create channels by default");
    assert!(!space.can_delete_channels(&member));
    assert!(!space.can_manage_channels(&member));
    assert!(!space.can_kick_members(&member), "Members shouldn't kick");
    assert!(!space.can_manage_roles(&member));
    assert!(!space.can_delete_messages(&member));
    assert!(space.can_invite_members(&member), "Members should be able to invite friends");
    
    println!("✓ Default role (Member) has correct permissions");
}

#[test]
fn test_moderator_role_permissions() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Find moderator role
    let mod_role_id = space.roles.iter()
        .find(|(_, role)| role.name == "Moderator")
        .map(|(id, _)| *id)
        .expect("Moderator role should exist");
    
    // Add a moderator
    let mod_keypair = Keypair::generate();
    let moderator = mod_keypair.user_id();
    space.assign_role(moderator, mod_role_id).unwrap();
    
    // Moderator should have moderation permissions
    assert!(space.can_create_channels(&moderator), "Moderators should create channels");
    assert!(!space.can_delete_channels(&moderator), "Moderators can't delete channels");
    assert!(space.can_manage_channels(&moderator));
    assert!(space.can_kick_members(&moderator), "Moderators should kick");
    assert!(!space.can_manage_roles(&moderator), "Moderators can't manage roles");
    assert!(space.can_delete_messages(&moderator), "Moderators should delete messages");
    assert!(space.can_invite_members(&moderator));
    
    println!("✓ Moderator role has correct permissions");
}

#[test]
fn test_admin_role_permissions() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Find admin role
    let admin_role_id = space.roles.iter()
        .find(|(_, role)| role.name == "Admin")
        .map(|(id, _)| *id)
        .expect("Admin role should exist");
    
    // Add an admin
    let admin_keypair = Keypair::generate();
    let admin = admin_keypair.user_id();
    space.assign_role(admin, admin_role_id).unwrap();
    
    // Admin should have all permissions
    assert!(space.can_create_channels(&admin));
    assert!(space.can_delete_channels(&admin));
    assert!(space.can_manage_channels(&admin));
    assert!(space.can_kick_members(&admin));
    assert!(space.can_manage_roles(&admin));
    assert!(space.can_delete_messages(&admin));
    assert!(space.can_invite_members(&admin));
    
    println!("✓ Admin role has all permissions");
}

#[test]
fn test_role_hierarchy_prevents_privilege_escalation() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Get role IDs
    let admin_role_id = space.roles.iter()
        .find(|(_, role)| role.name == "Admin")
        .map(|(id, _)| *id)
        .unwrap();
    
    let mod_role_id = space.roles.iter()
        .find(|(_, role)| role.name == "Moderator")
        .map(|(id, _)| *id)
        .unwrap();
    
    // Moderator trying to assign Admin role (should fail)
    let mod_keypair = Keypair::generate();
    let moderator = mod_keypair.user_id();
    
    // Moderator can't assign Admin role (higher position)
    assert!(!space.can_assign_role(&moderator, &admin_role_id),
        "Moderator shouldn't be able to assign Admin role");
    
    // Moderator CAN assign Member role (lower position)
    let member_role_id = space.default_role;
    assert!(space.can_assign_role(&owner, &member_role_id),
        "Owner should be able to assign Member role");
    
    // Owner can assign any role
    assert!(space.can_assign_role(&owner, &admin_role_id),
        "Owner should be able to assign Admin role");
    assert!(space.can_assign_role(&owner, &mod_role_id),
        "Owner should be able to assign Moderator role");
    
    println!("✓ Role hierarchy prevents privilege escalation");
}

#[test]
fn test_permission_bitfield_operations() {
    // Test bitfield grant/revoke
    let mut perms = SpacePermissions::none();
    
    assert!(!perms.has(SpacePermissions::CREATE_CHANNELS));
    
    perms.grant(SpacePermissions::CREATE_CHANNELS);
    assert!(perms.has(SpacePermissions::CREATE_CHANNELS));
    
    perms.grant(SpacePermissions::KICK_MEMBERS);
    assert!(perms.has(SpacePermissions::CREATE_CHANNELS));
    assert!(perms.has(SpacePermissions::KICK_MEMBERS));
    
    perms.revoke(SpacePermissions::CREATE_CHANNELS);
    assert!(!perms.has(SpacePermissions::CREATE_CHANNELS));
    assert!(perms.has(SpacePermissions::KICK_MEMBERS));
    
    println!("✓ Permission bitfield operations work correctly");
}

#[test]
fn test_space_manager_with_permissions() {
    let mut manager = SpaceManager::new();
    let provider = create_provider();
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let creator_keypair = Keypair::generate();
    let creator = creator_keypair.user_id();
    
    // Create space
    let _op = manager.create_space(
        space_id,
        "Permission Test Space".to_string(),
        Some("Testing permissions".to_string()),
        creator,
        &creator_keypair,
        &provider,
    ).unwrap();
    
    let space = manager.get_space(&space_id).unwrap();
    
    // Creator should have all permissions
    assert!(space.can_create_channels(&creator));
    assert!(space.can_kick_members(&creator));
    assert!(space.can_manage_roles(&creator));
    
    // Verify default roles exist
    assert_eq!(space.roles.len(), 3, "Should have 3 default roles");
    
    let role_names: Vec<&str> = space.roles.values()
        .map(|r| r.name.as_str())
        .collect();
    
    assert!(role_names.contains(&"Admin"));
    assert!(role_names.contains(&"Moderator"));
    assert!(role_names.contains(&"Member"));
    
    println!("✓ SpaceManager creates spaces with permission system");
}

#[test]
fn test_member_without_create_channel_permission() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Add a regular member
    let member_keypair = Keypair::generate();
    let member = member_keypair.user_id();
    space.assign_role(member, space.default_role).unwrap();
    
    // Member shouldn't be able to create channels
    assert!(!space.can_create_channels(&member),
        "Regular members should not have create_channels permission by default");
    
    // This simulates the check that would happen in create_channel API
    // In the real implementation, this check would prevent the channel creation
    
    println!("✓ Members without permission cannot create channels");
}

#[test]
fn test_custom_role_creation() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Create a custom "Channel Manager" role
    let mut perms = SpacePermissions::none();
    perms.grant(SpacePermissions::CREATE_CHANNELS);
    perms.grant(SpacePermissions::MANAGE_CHANNELS);
    perms.grant(SpacePermissions::INVITE_MEMBERS);
    
    let custom_role = SpaceRole::new(
        "Channel Manager".to_string(),
        perms,
        25, // Position between Member (0) and Moderator (50)
    );
    
    let custom_role_id = custom_role.id;
    space.roles.insert(custom_role_id, custom_role);
    
    // Assign to user
    let user_keypair = Keypair::generate();
    let user = user_keypair.user_id();
    space.assign_role(user, custom_role_id).unwrap();
    
    // Check permissions
    assert!(space.can_create_channels(&user));
    assert!(space.can_manage_channels(&user));
    assert!(space.can_invite_members(&user));
    assert!(!space.can_kick_members(&user));
    assert!(!space.can_delete_messages(&user));
    
    println!("✓ Custom roles work correctly");
}

#[test]
fn test_channel_permissions_independent() {
    // Test that channel permissions are independent from space permissions
    let chan_perms = ChannelPermissions::member();
    
    assert!(chan_perms.has(ChannelPermissions::SEND_MESSAGES));
    assert!(chan_perms.has(ChannelPermissions::ADD_MEMBERS));
    assert!(!chan_perms.has(ChannelPermissions::KICK_MEMBERS));
    assert!(!chan_perms.has(ChannelPermissions::DELETE_MESSAGES));
    
    let all_perms = ChannelPermissions::all();
    assert!(all_perms.has(ChannelPermissions::KICK_MEMBERS));
    assert!(all_perms.has(ChannelPermissions::DELETE_MESSAGES));
    assert!(all_perms.has(ChannelPermissions::MANAGE_CHANNEL));
    
    println!("✓ Channel permissions are independent");
}

#[test]
fn test_backward_compatibility_with_old_role_enum() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Old API still works (deprecated members HashMap)
    #[allow(deprecated)]
    let old_role = space.members.get(&owner);
    assert_eq!(old_role, Some(&Role::Admin));
    
    // Old Role enum methods still work
    assert!(Role::Admin.is_admin());
    assert!(Role::Admin.can_moderate());
    assert!(Role::Moderator.can_moderate());
    assert!(!Role::Member.can_moderate());
    
    println!("✓ Backward compatibility with old Role enum");
}

#[test]
fn test_multiple_users_different_permissions() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Get role IDs (clone them to avoid borrow checker issues)
    let admin_role_id = *space.roles.iter()
        .find(|(_, r)| r.name == "Admin").unwrap().0;
    let mod_role_id = *space.roles.iter()
        .find(|(_, r)| r.name == "Moderator").unwrap().0;
    let member_role_id = space.default_role;
    
    // Create users with different roles
    let admin_kp = Keypair::generate();
    let admin = admin_kp.user_id();
    space.assign_role(admin, admin_role_id).unwrap();
    
    let mod_kp = Keypair::generate();
    let moderator = mod_kp.user_id();
    space.assign_role(moderator, mod_role_id).unwrap();
    
    let member_kp = Keypair::generate();
    let member = member_kp.user_id();
    space.assign_role(member, member_role_id).unwrap();
    
    // Verify each has correct permissions
    assert!(space.can_manage_roles(&admin));
    assert!(!space.can_manage_roles(&moderator));
    assert!(!space.can_manage_roles(&member));
    
    assert!(space.can_kick_members(&admin));
    assert!(space.can_kick_members(&moderator));
    assert!(!space.can_kick_members(&member));
    
    assert!(space.can_create_channels(&admin));
    assert!(space.can_create_channels(&moderator));
    assert!(!space.can_create_channels(&member));
    
    println!("✓ Multiple users with different permissions");
    println!("  - Admin: all permissions");
    println!("  - Moderator: moderation permissions");
    println!("  - Member: basic permissions");
}

#[test]
fn test_get_user_role() {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "test", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Test Space".to_string(),
        None,
        owner,
        1000,
    );
    
    // Get owner's role
    let owner_role = space.get_user_role(&owner).unwrap();
    assert_eq!(owner_role.name, "Admin");
    assert_eq!(owner_role.position, 100);
    
    // Add a moderator
    let mod_role_id = space.roles.iter()
        .find(|(_, r)| r.name == "Moderator").unwrap().0;
    
    let mod_kp = Keypair::generate();
    let moderator = mod_kp.user_id();
    space.assign_role(moderator, *mod_role_id).unwrap();
    
    let mod_role = space.get_user_role(&moderator).unwrap();
    assert_eq!(mod_role.name, "Moderator");
    assert_eq!(mod_role.position, 50);
    
    // Non-member has no role
    let stranger_kp = Keypair::generate();
    let stranger = stranger_kp.user_id();
    assert!(space.get_user_role(&stranger).is_none());
    
    println!("✓ get_user_role works correctly");
}
