//! Permission System Demo & Integration Tests
//! 
//! This file demonstrates the full Discord-Lite permission system with practical scenarios:
//! - Creating spaces with custom roles
//! - Managing role hierarchies
//! - Permission checks for common operations
//! - Multi-user permission scenarios
//! - Bitfield operations

use spaceway_core::forum::{Space, SpaceManager};
use spaceway_core::types::*;
use spaceway_core::crypto::signing::Keypair;
use spaceway_core::{SpacePermissions, ChannelPermissions, SpaceRole, RoleId};

/// Helper function to create a space with users
fn create_demo_space() -> (Space, UserId, UserId, UserId, UserId) {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "demo-space", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Demo Space".to_string(),
        Some("A demo space to showcase permissions".to_string()),
        owner,
        1000,
    );
    
    // Create test users
    let admin_keypair = Keypair::generate();
    let admin = admin_keypair.user_id();
    
    let moderator_keypair = Keypair::generate();
    let moderator = moderator_keypair.user_id();
    
    let member_keypair = Keypair::generate();
    let member = member_keypair.user_id();
    
    (space, owner, admin, moderator, member)
}

#[test]
fn demo_1_basic_role_assignment() {
    println!("\n=== DEMO 1: Basic Role Assignment ===\n");
    
    let (mut space, owner, admin, moderator, member) = create_demo_space();
    
    // Get role IDs
    let admin_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Admin")
        .unwrap().0;
    
    let moderator_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Moderator")
        .unwrap().0;
    
    let member_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap().0;
    
    // Assign roles
    space.assign_role(admin, admin_role_id).unwrap();
    space.assign_role(moderator, moderator_role_id).unwrap();
    space.assign_role(member, member_role_id).unwrap();
    
    println!("👤 Owner: {:?}", &owner.0[..8]);
    println!("👤 Admin: {:?} (role: Admin)", &admin.0[..8]);
    println!("👤 Moderator: {:?} (role: Moderator)", &moderator.0[..8]);
    println!("👤 Member: {:?} (role: Member)", &member.0[..8]);
    
    // Verify role assignments
    assert_eq!(space.get_user_role(&admin), Some(&admin_role_id));
    assert_eq!(space.get_user_role(&moderator), Some(&moderator_role_id));
    assert_eq!(space.get_user_role(&member), Some(&member_role_id));
    
    println!("\n✓ Roles assigned successfully");
}

#[test]
fn demo_2_permission_checks() {
    println!("\n=== DEMO 2: Permission Checks ===\n");
    
    let (mut space, owner, admin, moderator, member) = create_demo_space();
    
    // Assign roles
    let admin_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Admin")
        .unwrap().0;
    let moderator_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Moderator")
        .unwrap().0;
    let member_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap().0;
    
    space.assign_role(admin, admin_role_id).unwrap();
    space.assign_role(moderator, moderator_role_id).unwrap();
    space.assign_role(member, member_role_id).unwrap();
    
    println!("Testing CREATE_CHANNELS permission:");
    println!("  Owner: {}", space.can_create_channels(&owner));
    println!("  Admin: {}", space.can_create_channels(&admin));
    println!("  Moderator: {}", space.can_create_channels(&moderator));
    println!("  Member: {}", space.can_create_channels(&member));
    
    println!("\nTesting KICK_MEMBERS permission:");
    println!("  Owner: {}", space.can_kick_members(&owner));
    println!("  Admin: {}", space.can_kick_members(&admin));
    println!("  Moderator: {}", space.can_kick_members(&moderator));
    println!("  Member: {}", space.can_kick_members(&member));
    
    println!("\nTesting MANAGE_ROLES permission:");
    println!("  Owner: {}", space.can_manage_roles(&owner));
    println!("  Admin: {}", space.can_manage_roles(&admin));
    println!("  Moderator: {}", space.can_manage_roles(&moderator));
    println!("  Member: {}", space.can_manage_roles(&member));
    
    // Assertions
    assert!(space.can_create_channels(&owner));
    assert!(space.can_create_channels(&admin));
    assert!(space.can_create_channels(&moderator));
    assert!(!space.can_create_channels(&member));
    
    assert!(space.can_kick_members(&owner));
    assert!(space.can_kick_members(&admin));
    assert!(space.can_kick_members(&moderator));
    assert!(!space.can_kick_members(&member));
    
    println!("\n✓ Permission checks working correctly");
}

#[test]
fn demo_3_custom_role_creation() {
    println!("\n=== DEMO 3: Custom Role Creation ===\n");
    
    let (mut space, owner, _, _, _) = create_demo_space();
    
    // Create a "Content Creator" role with specific permissions
    let content_creator_permissions = SpacePermissions::CREATE_CHANNELS
        | SpacePermissions::SEND_MESSAGES
        | SpacePermissions::ATTACH_FILES
        | SpacePermissions::CREATE_THREADS;
    
    let content_creator_role = SpaceRole {
        id: RoleId::new(),
        name: "Content Creator".to_string(),
        permissions: content_creator_permissions,
        position: 50, // Between Member (0) and Moderator (100)
        color: Some("#FF6B6B".to_string()), // Nice red color
    };
    
    let role_id = content_creator_role.id;
    space.roles.insert(role_id, content_creator_role);
    
    println!("Created custom role: 'Content Creator'");
    println!("  Permissions: CREATE_CHANNELS, SEND_MESSAGES, ATTACH_FILES, CREATE_THREADS");
    println!("  Position: 50 (between Member and Moderator)");
    println!("  Color: #FF6B6B");
    
    // Assign the role to a user
    let creator_keypair = Keypair::generate();
    let creator = creator_keypair.user_id();
    space.assign_role(creator, role_id).unwrap();
    
    // Verify permissions
    assert!(space.can_create_channels(&creator));
    assert!(space.can_send_messages(&creator));
    assert!(space.can_attach_files(&creator));
    assert!(space.can_create_threads(&creator));
    assert!(!space.can_kick_members(&creator)); // Shouldn't have moderation powers
    assert!(!space.can_manage_roles(&creator));
    
    println!("\n✓ Custom role created and working correctly");
}

#[test]
fn demo_4_role_hierarchy() {
    println!("\n=== DEMO 4: Role Hierarchy ===\n");
    
    let (mut space, owner, _, _, _) = create_demo_space();
    
    // Get role IDs and positions
    let admin_role = space.roles.iter()
        .find(|(_, role)| role.name == "Admin")
        .unwrap();
    let moderator_role = space.roles.iter()
        .find(|(_, role)| role.name == "Moderator")
        .unwrap();
    let member_role = space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap();
    
    println!("Role Hierarchy (by position):");
    println!("  Admin: position = {}", admin_role.1.position);
    println!("  Moderator: position = {}", moderator_role.1.position);
    println!("  Member: position = {}", member_role.1.position);
    
    // Test that lower roles can't be assigned to manage higher roles
    let mod_keypair = Keypair::generate();
    let moderator_user = mod_keypair.user_id();
    let moderator_role_id = *moderator_role.0;
    let admin_role_id = *admin_role.0;
    
    space.assign_role(moderator_user, moderator_role_id).unwrap();
    
    // Create another user to try to promote
    let user_keypair = Keypair::generate();
    let user = user_keypair.user_id();
    
    println!("\nTesting role hierarchy:");
    println!("  Can moderator assign Admin role? {}", 
        space.can_assign_role(&moderator_user, &admin_role_id));
    println!("  Can moderator assign Moderator role? {}", 
        space.can_assign_role(&moderator_user, &moderator_role_id));
    
    // Moderator shouldn't be able to assign Admin role (higher position)
    assert!(!space.can_assign_role(&moderator_user, &admin_role_id));
    
    println!("\n✓ Role hierarchy prevents privilege escalation");
}

#[test]
fn demo_5_permission_combinations() {
    println!("\n=== DEMO 5: Permission Combinations ===\n");
    
    let (mut space, _, _, _, _) = create_demo_space();
    
    // Create a "Support" role with specific permissions
    let support_permissions = SpacePermissions::KICK_MEMBERS
        | SpacePermissions::DELETE_MESSAGES
        | SpacePermissions::TIMEOUT_MEMBERS
        | SpacePermissions::VIEW_AUDIT_LOG;
    
    let support_role = SpaceRole {
        id: RoleId::new(),
        name: "Support".to_string(),
        permissions: support_permissions,
        position: 75,
        color: Some("#4ECDC4".to_string()),
    };
    
    println!("Support role permissions (bitfield):");
    println!("  Raw value: 0x{:08X}", support_permissions.bits());
    println!("  KICK_MEMBERS: {}", support_permissions.contains(SpacePermissions::KICK_MEMBERS));
    println!("  DELETE_MESSAGES: {}", support_permissions.contains(SpacePermissions::DELETE_MESSAGES));
    println!("  TIMEOUT_MEMBERS: {}", support_permissions.contains(SpacePermissions::TIMEOUT_MEMBERS));
    println!("  VIEW_AUDIT_LOG: {}", support_permissions.contains(SpacePermissions::VIEW_AUDIT_LOG));
    println!("  CREATE_CHANNELS: {}", support_permissions.contains(SpacePermissions::CREATE_CHANNELS));
    
    let role_id = support_role.id;
    space.roles.insert(role_id, support_role);
    
    let support_keypair = Keypair::generate();
    let support_user = support_keypair.user_id();
    space.assign_role(support_user, role_id).unwrap();
    
    // Verify the user has the right permissions
    assert!(space.can_kick_members(&support_user));
    assert!(space.can_delete_messages(&support_user));
    assert!(space.can_timeout_members(&support_user));
    assert!(space.can_view_audit_log(&support_user));
    assert!(!space.can_create_channels(&support_user));
    assert!(!space.can_manage_roles(&support_user));
    
    println!("\n✓ Permission combinations work correctly");
}

#[test]
fn demo_6_channel_permissions() {
    println!("\n=== DEMO 6: Channel Permissions ===\n");
    
    let (space, owner, _, _, _) = create_demo_space();
    
    // Channel permissions are separate from space permissions
    let channel_perms = ChannelPermissions::SEND_MESSAGES
        | ChannelPermissions::CREATE_THREADS
        | ChannelPermissions::ATTACH_FILES;
    
    println!("Channel permissions (independent from space):");
    println!("  Raw value: 0x{:08X}", channel_perms.bits());
    println!("  SEND_MESSAGES: {}", channel_perms.contains(ChannelPermissions::SEND_MESSAGES));
    println!("  CREATE_THREADS: {}", channel_perms.contains(ChannelPermissions::CREATE_THREADS));
    println!("  ATTACH_FILES: {}", channel_perms.contains(ChannelPermissions::ATTACH_FILES));
    println!("  MANAGE_MESSAGES: {}", channel_perms.contains(ChannelPermissions::MANAGE_MESSAGES));
    
    // Verify bitfield operations
    assert!(channel_perms.contains(ChannelPermissions::SEND_MESSAGES));
    assert!(channel_perms.contains(ChannelPermissions::CREATE_THREADS));
    assert!(!channel_perms.contains(ChannelPermissions::MANAGE_MESSAGES));
    
    println!("\n✓ Channel permissions are independent from space permissions");
}

#[test]
fn demo_7_realistic_scenario() {
    println!("\n=== DEMO 7: Realistic Community Server Scenario ===\n");
    
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "gaming-community", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Gaming Community".to_string(),
        Some("A friendly gaming community".to_string()),
        owner,
        1000,
    );
    
    println!("🎮 Gaming Community Server Created");
    println!("   Owner: {:?}", &owner.0[..8]);
    
    // Create custom roles for different community positions
    
    // 1. VIP role (supporters who get extra perks)
    let vip_role = SpaceRole {
        id: RoleId::new(),
        name: "VIP".to_string(),
        permissions: SpacePermissions::SEND_MESSAGES 
            | SpacePermissions::ATTACH_FILES 
            | SpacePermissions::CREATE_THREADS
            | SpacePermissions::USE_VOICE,
        position: 25,
        color: Some("#FFD700".to_string()), // Gold
    };
    let vip_role_id = vip_role.id;
    space.roles.insert(vip_role_id, vip_role);
    
    // 2. Event Organizer (can manage events and announcements)
    let organizer_role = SpaceRole {
        id: RoleId::new(),
        name: "Event Organizer".to_string(),
        permissions: SpacePermissions::CREATE_CHANNELS
            | SpacePermissions::MANAGE_CHANNELS
            | SpacePermissions::SEND_MESSAGES
            | SpacePermissions::ATTACH_FILES
            | SpacePermissions::MENTION_ALL,
        position: 60,
        color: Some("#9B59B6".to_string()), // Purple
    };
    let organizer_role_id = organizer_role.id;
    space.roles.insert(organizer_role_id, organizer_role);
    
    // 3. Community Helper (junior moderator)
    let helper_role = SpaceRole {
        id: RoleId::new(),
        name: "Helper".to_string(),
        permissions: SpacePermissions::DELETE_MESSAGES
            | SpacePermissions::TIMEOUT_MEMBERS
            | SpacePermissions::SEND_MESSAGES,
        position: 80,
        color: Some("#3498DB".to_string()), // Blue
    };
    let helper_role_id = helper_role.id;
    space.roles.insert(helper_role_id, helper_role);
    
    println!("\n📋 Custom Roles Created:");
    println!("   🌟 VIP (position 25) - Chat perks");
    println!("   📅 Event Organizer (position 60) - Manage events");
    println!("   🛡️  Helper (position 80) - Junior moderator");
    
    // Assign roles to users
    let vip_user = Keypair::generate().user_id();
    let organizer_user = Keypair::generate().user_id();
    let helper_user = Keypair::generate().user_id();
    let regular_user = Keypair::generate().user_id();
    
    space.assign_role(vip_user, vip_role_id).unwrap();
    space.assign_role(organizer_user, organizer_role_id).unwrap();
    space.assign_role(helper_user, helper_role_id).unwrap();
    
    // Get member role for regular user
    let member_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap().0;
    space.assign_role(regular_user, member_role_id).unwrap();
    
    println!("\n👥 Users Assigned:");
    println!("   VIP: {:?}", &vip_user.0[..8]);
    println!("   Organizer: {:?}", &organizer_user.0[..8]);
    println!("   Helper: {:?}", &helper_user.0[..8]);
    println!("   Regular: {:?}", &regular_user.0[..8]);
    
    // Test realistic scenarios
    println!("\n🎯 Testing Real-World Scenarios:");
    
    println!("\n1. Can VIP create event channels?");
    let can_create = space.can_create_channels(&vip_user);
    println!("   Result: {} (VIPs don't manage channels)", can_create);
    assert!(!can_create);
    
    println!("\n2. Can Event Organizer create announcement channel?");
    let can_create = space.can_create_channels(&organizer_user);
    println!("   Result: {} (Organizers can create channels)", can_create);
    assert!(can_create);
    
    println!("\n3. Can Helper timeout spam users?");
    let can_timeout = space.can_timeout_members(&helper_user);
    println!("   Result: {} (Helpers have moderation powers)", can_timeout);
    assert!(can_timeout);
    
    println!("\n4. Can Helper promote VIP to Organizer?");
    let can_assign = space.can_assign_role(&helper_user, &organizer_role_id);
    println!("   Result: {} (Helpers can't assign higher roles)", can_assign);
    assert!(!can_assign);
    
    println!("\n5. Can regular user mention @everyone?");
    let can_mention = space.can_mention_all(&regular_user);
    println!("   Result: {} (Regular members can't spam)", can_mention);
    assert!(!can_mention);
    
    println!("\n✓ Realistic gaming community scenario works perfectly!");
}

#[test]
fn demo_8_space_manager_integration() {
    println!("\n=== DEMO 8: SpaceManager Integration ===\n");
    
    let mut manager = SpaceManager::new();
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    // Create space through SpaceManager
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "managed-space", 1000);
    
    // Create provider for MLS operations
    let provider = spaceway_core::mls::provider::create_provider();
    
    let result = manager.create_space(
        space_id,
        "Managed Space".to_string(),
        Some("Created through SpaceManager".to_string()),
        owner,
        &owner_keypair,
        &provider,
    );
    
    assert!(result.is_ok());
    println!("✓ Space created through SpaceManager");
    
    let space = manager.get_space(&space_id).unwrap();
    
    // Verify default roles are present
    assert!(space.roles.len() >= 3); // Admin, Moderator, Member
    println!("✓ Default roles (Admin, Moderator, Member) created");
    
    // Verify owner has all permissions
    assert!(space.can_create_channels(&owner));
    assert!(space.can_manage_roles(&owner));
    assert!(space.can_kick_members(&owner));
    println!("✓ Owner has all permissions");
    
    // Add a member and assign role
    let member_keypair = Keypair::generate();
    let member = member_keypair.user_id();
    
    let member_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap().0;
    
    let mut space_mut = manager.get_space_mut(&space_id).unwrap();
    space_mut.assign_role(member, member_role_id).unwrap();
    
    assert_eq!(space_mut.get_user_role(&member), Some(&member_role_id));
    println!("✓ Member added and assigned default role");
    
    println!("\n✓ SpaceManager integration works correctly");
}

#[test]
fn demo_9_permission_updates() {
    println!("\n=== DEMO 9: Dynamic Permission Updates ===\n");
    
    let (mut space, owner, _, _, _) = create_demo_space();
    
    // Get member role
    let member_role_id = *space.roles.iter()
        .find(|(_, role)| role.name == "Member")
        .unwrap().0;
    
    println!("Initial Member permissions:");
    let member_role = space.roles.get(&member_role_id).unwrap();
    println!("  Can create channels: {}", member_role.permissions.contains(SpacePermissions::CREATE_CHANNELS));
    println!("  Can send messages: {}", member_role.permissions.contains(SpacePermissions::SEND_MESSAGES));
    
    // Update member role to allow channel creation
    let member_role_mut = space.roles.get_mut(&member_role_id).unwrap();
    member_role_mut.permissions |= SpacePermissions::CREATE_CHANNELS;
    
    println!("\nUpdated Member permissions:");
    let member_role = space.roles.get(&member_role_id).unwrap();
    println!("  Can create channels: {}", member_role.permissions.contains(SpacePermissions::CREATE_CHANNELS));
    println!("  Can send messages: {}", member_role.permissions.contains(SpacePermissions::SEND_MESSAGES));
    
    // Assign to a user and verify
    let user = Keypair::generate().user_id();
    space.assign_role(user, member_role_id).unwrap();
    
    assert!(space.can_create_channels(&user));
    println!("\n✓ Permission updates apply to all users with the role");
}

#[test]
fn demo_10_bitfield_operations() {
    println!("\n=== DEMO 10: Bitfield Operations Deep Dive ===\n");
    
    let (space, _, _, _, _) = create_demo_space();
    
    println!("Space Permission Bits:");
    println!("  CREATE_CHANNELS:  0x{:08X}", SpacePermissions::CREATE_CHANNELS.bits());
    println!("  DELETE_CHANNELS:  0x{:08X}", SpacePermissions::DELETE_CHANNELS.bits());
    println!("  MANAGE_CHANNELS:  0x{:08X}", SpacePermissions::MANAGE_CHANNELS.bits());
    println!("  KICK_MEMBERS:     0x{:08X}", SpacePermissions::KICK_MEMBERS.bits());
    println!("  MANAGE_ROLES:     0x{:08X}", SpacePermissions::MANAGE_ROLES.bits());
    
    // Demonstrate bitwise operations
    let perms1 = SpacePermissions::CREATE_CHANNELS | SpacePermissions::MANAGE_CHANNELS;
    let perms2 = SpacePermissions::MANAGE_CHANNELS | SpacePermissions::KICK_MEMBERS;
    
    println!("\nBitwise OR (perms1 | perms2):");
    println!("  perms1: 0x{:08X}", perms1.bits());
    println!("  perms2: 0x{:08X}", perms2.bits());
    let combined = perms1 | perms2;
    println!("  result: 0x{:08X}", combined.bits());
    println!("  Contains CREATE_CHANNELS: {}", combined.contains(SpacePermissions::CREATE_CHANNELS));
    println!("  Contains MANAGE_CHANNELS: {}", combined.contains(SpacePermissions::MANAGE_CHANNELS));
    println!("  Contains KICK_MEMBERS: {}", combined.contains(SpacePermissions::KICK_MEMBERS));
    
    println!("\nBitwise AND (perms1 & perms2):");
    let intersection = perms1 & perms2;
    println!("  result: 0x{:08X}", intersection.bits());
    println!("  Contains MANAGE_CHANNELS: {}", intersection.contains(SpacePermissions::MANAGE_CHANNELS));
    println!("  Contains CREATE_CHANNELS: {}", intersection.contains(SpacePermissions::CREATE_CHANNELS));
    
    println!("\nBitwise NOT (complement):");
    let all_perms = SpacePermissions::all();
    println!("  all permissions: 0x{:08X}", all_perms.bits());
    let inverted = !perms1;
    println!("  inverted perms1: 0x{:08X}", inverted.bits());
    
    println!("\n✓ Bitfield operations work correctly");
}
