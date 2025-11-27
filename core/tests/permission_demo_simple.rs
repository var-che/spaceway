//! Simple Permission System Demonstrations
//! 
//! This showcases the Discord-Lite permission system with working examples.

use spaceway_core::forum::{Space, SpaceManager};
use spaceway_core::types::*;
use spaceway_core::crypto::signing::Keypair;
use spaceway_core::{SpacePermissions, ChannelPermissions, SpaceRole, RoleId};

/// Helper to create a test space
fn create_test_space() -> (Space, UserId) {
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "demo", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let space = Space::new(
        space_id,
        "Demo Space".to_string(),
        Some("Testing permissions".to_string()),
        owner,
        1000,
    );
    
    (space, owner)
}

#[test]
fn demo_1_owner_permissions() {
    println!("\n=== DEMO 1: Owner Has All Permissions ===\n");
    
    let (space, owner) = create_test_space();
    
    println!("Owner ID: {:?}", &owner.0[..8]);
    println!("\nChecking owner permissions:");
    println!("  ✓ Can create channels: {}", space.can_create_channels(&owner));
    println!("  ✓ Can delete channels: {}", space.can_delete_channels(&owner));
    println!("  ✓ Can kick members: {}", space.can_kick_members(&owner));
    println!("  ✓ Can manage roles: {}", space.can_manage_roles(&owner));
    println!("  ✓ Can delete messages: {}", space.can_delete_messages(&owner));
    println!("  ✓ Can invite members: {}", space.can_invite_members(&owner));
    
    // Verify
    assert!(space.can_create_channels(&owner));
    assert!(space.can_delete_channels(&owner));
    assert!(space.can_kick_members(&owner));
    assert!(space.can_manage_roles(&owner));
    assert!(space.can_delete_messages(&owner));
    assert!(space.can_invite_members(&owner));
    
    println!("\n✅ Owner bypasses all permission checks!");
}

#[test]
fn demo_2_default_roles() {
    println!("\n=== DEMO 2: Default Roles (Admin, Moderator, Member) ===\n");
    
    let (space, owner) = create_test_space();
    
    println!("Default roles created with space:");
    for (role_id, role) in &space.roles {
        println!("\n📋 Role: {}", role.name);
        println!("   ID: {:?}", &role_id.0.to_string()[..8]);
        println!("   Position: {}", role.position);
        println!("   Color: 0x{:06X}", role.color.unwrap_or(0));
        println!("   Permissions bits: 0x{:08X}", role.permissions.bits);
        
        // Show specific permissions
        println!("   Can create channels: {}", role.permissions.has(SpacePermissions::CREATE_CHANNELS));
        println!("   Can kick members: {}", role.permissions.has(SpacePermissions::KICK_MEMBERS));
        println!("   Can manage roles: {}", role.permissions.has(SpacePermissions::MANAGE_ROLES));
    }
    
    // Verify correct number of default roles
    assert_eq!(space.roles.len(), 3, "Should have 3 default roles");
    assert!(space.roles.values().any(|r| r.name == "Admin"));
    assert!(space.roles.values().any(|r| r.name == "Moderator"));
    assert!(space.roles.values().any(|r| r.name == "Member"));
    
    println!("\n✅ Default roles configured correctly!");
}

#[test]
fn demo_3_role_assignment() {
    println!("\n=== DEMO 3: Assigning Roles to Users ===\n");
    
    let (mut space, owner) = create_test_space();
    
    // Create test users
    let admin_user = Keypair::generate().user_id();
    let mod_user = Keypair::generate().user_id();
    let member_user = Keypair::generate().user_id();
    
    // Get role IDs
    let admin_role_id = *space.roles.iter()
        .find(|(_, r)| r.name == "Admin")
        .unwrap().0;
    let mod_role_id = *space.roles.iter()
        .find(|(_, r)| r.name == "Moderator")
        .unwrap().0;
    let member_role_id = *space.roles.iter()
        .find(|(_, r)| r.name == "Member")
        .unwrap().0;
    
    // Assign roles
    space.assign_role(admin_user, admin_role_id).unwrap();
    space.assign_role(mod_user, mod_role_id).unwrap();
    space.assign_role(member_user, member_role_id).unwrap();
    
    println!("Assigned roles:");
    println!("  Admin: {:?}", &admin_user.0[..8]);
    println!("  Moderator: {:?}", &mod_user.0[..8]);
    println!("  Member: {:?}", &member_user.0[..8]);
    
    println!("\nPermission comparison:");
    println!("  Can create channels:");
    println!("    Admin: {}", space.can_create_channels(&admin_user));
    println!("    Moderator: {}", space.can_create_channels(&mod_user));
    println!("    Member: {}", space.can_create_channels(&member_user));
    
    println!("  Can manage roles:");
    println!("    Admin: {}", space.can_manage_roles(&admin_user));
    println!("    Moderator: {}", space.can_manage_roles(&mod_user));
    println!("    Member: {}", space.can_manage_roles(&member_user));
    
    // Verify
    assert!(space.can_create_channels(&admin_user));
    assert!(space.can_create_channels(&mod_user));
    assert!(!space.can_create_channels(&member_user));
    
    assert!(space.can_manage_roles(&admin_user));
    assert!(!space.can_manage_roles(&mod_user));
    assert!(!space.can_manage_roles(&member_user));
    
    println!("\n✅ Role assignments working correctly!");
}

#[test]
fn demo_4_custom_role_creation() {
    println!("\n=== DEMO 4: Creating Custom Roles ===\n");
    
    let (mut space, owner) = create_test_space();
    
    // Create a "Support" role with specific permissions
    let support_perms = SpacePermissions {
        bits: SpacePermissions::INVITE_MEMBERS
            | SpacePermissions::KICK_MEMBERS
            | SpacePermissions::DELETE_MESSAGES
            | SpacePermissions::PIN_MESSAGES
    };
    
    let support_role = SpaceRole {
        id: RoleId::new(),
        name: "Support".to_string(),
        permissions: support_perms,
        position: 75, // Between Moderator (50) and Admin (100)
        color: Some(0x3498DB), // Blue
    };
    
    println!("Created custom 'Support' role:");
    println!("  Position: {}", support_role.position);
    println!("  Color: 0x{:06X}", support_role.color.unwrap());
    println!("  Permissions bits: 0x{:08X}", support_role.permissions.bits);
    println!("\nSpecific permissions:");
    println!("  Can invite members: {}", support_role.permissions.has(SpacePermissions::INVITE_MEMBERS));
    println!("  Can kick members: {}", support_role.permissions.has(SpacePermissions::KICK_MEMBERS));
    println!("  Can delete messages: {}", support_role.permissions.has(SpacePermissions::DELETE_MESSAGES));
    println!("  Can create channels: {}", support_role.permissions.has(SpacePermissions::CREATE_CHANNELS));
    println!("  Can manage roles: {}", support_role.permissions.has(SpacePermissions::MANAGE_ROLES));
    
    // Add to space and assign to user
    let role_id = support_role.id;
    space.roles.insert(role_id, support_role);
    
    let support_user = Keypair::generate().user_id();
    space.assign_role(support_user, role_id).unwrap();
    
    // Verify permissions
    assert!(space.can_kick_members(&support_user));
    assert!(space.can_delete_messages(&support_user));
    assert!(!space.can_create_channels(&support_user));
    assert!(!space.can_manage_roles(&support_user));
    
    println!("\n✅ Custom role created and working!");
}

#[test]
fn demo_5_role_hierarchy() {
    println!("\n=== DEMO 5: Role Hierarchy Prevents Privilege Escalation ===\n");
    
    let (mut space, owner) = create_test_space();
    
    // Get roles
    let admin_role_id = *space.roles.iter().find(|(_, r)| r.name == "Admin").unwrap().0;
    let mod_role_id = *space.roles.iter().find(|(_, r)| r.name == "Moderator").unwrap().0;
    
    // Assign moderator role to a user
    let mod_user = Keypair::generate().user_id();
    space.assign_role(mod_user, mod_role_id).unwrap();
    
    println!("Role positions:");
    println!("  Admin: position {}", space.roles.get(&admin_role_id).unwrap().position);
    println!("  Moderator: position {}", space.roles.get(&mod_role_id).unwrap().position);
    
    println!("\nCan moderator assign Admin role? {}", space.can_assign_role(&mod_user, &admin_role_id));
    println!("Can moderator assign Moderator role? {}", space.can_assign_role(&mod_user, &mod_role_id));
    
    // Moderator cannot assign Admin role (higher position)
    assert!(!space.can_assign_role(&mod_user, &admin_role_id));
    
    // But owner can
    assert!(space.can_assign_role(&owner, &admin_role_id));
    
    println!("\n✅ Hierarchy prevents privilege escalation!");
}

#[test]
fn demo_6_permission_bitfield_operations() {
    println!("\n=== DEMO 6: Bitfield Operations ===\n");
    
    println!("Permission constants (as u32):");
    println!("  CREATE_CHANNELS:  0x{:08X} (bit {})", SpacePermissions::CREATE_CHANNELS, SpacePermissions::CREATE_CHANNELS.trailing_zeros());
    println!("  DELETE_CHANNELS:  0x{:08X} (bit {})", SpacePermissions::DELETE_CHANNELS, SpacePermissions::DELETE_CHANNELS.trailing_zeros());
    println!("  KICK_MEMBERS:     0x{:08X} (bit {})", SpacePermissions::KICK_MEMBERS, SpacePermissions::KICK_MEMBERS.trailing_zeros());
    println!("  MANAGE_ROLES:     0x{:08X} (bit {})", SpacePermissions::MANAGE_ROLES, SpacePermissions::MANAGE_ROLES.trailing_zeros());
    
    // Combine permissions using bitwise OR
    let combined = SpacePermissions {
        bits: SpacePermissions::CREATE_CHANNELS | SpacePermissions::KICK_MEMBERS
    };
    
    println!("\nCombined CREATE_CHANNELS | KICK_MEMBERS:");
    println!("  Result bits: 0x{:08X}", combined.bits);
    println!("  Has CREATE_CHANNELS: {}", combined.has(SpacePermissions::CREATE_CHANNELS));
    println!("  Has KICK_MEMBERS: {}", combined.has(SpacePermissions::KICK_MEMBERS));
    println!("  Has MANAGE_ROLES: {}", combined.has(SpacePermissions::MANAGE_ROLES));
    
    assert!(combined.has(SpacePermissions::CREATE_CHANNELS));
    assert!(combined.has(SpacePermissions::KICK_MEMBERS));
    assert!(!combined.has(SpacePermissions::MANAGE_ROLES));
    
    println!("\n✅ Bitfield operations work correctly!");
}

#[test]
fn demo_7_channel_permissions() {
    println!("\n=== DEMO 7: Channel Permissions (Independent) ===\n");
    
    let (space, owner) = create_test_space();
    
    println!("Channel permissions are independent from space permissions:");
    println!("\nChannel permission constants:");
    println!("  SEND_MESSAGES:    0x{:08X}", ChannelPermissions::SEND_MESSAGES);
    println!("  DELETE_MESSAGES:  0x{:08X}", ChannelPermissions::DELETE_MESSAGES);
    println!("  KICK_MEMBERS:     0x{:08X}", ChannelPermissions::KICK_MEMBERS);
    println!("  ADD_MEMBERS:      0x{:08X}", ChannelPermissions::ADD_MEMBERS);
    println!("  MANAGE_CHANNEL:   0x{:08X}", ChannelPermissions::MANAGE_CHANNEL);
    println!("  PIN_MESSAGES:     0x{:08X}", ChannelPermissions::PIN_MESSAGES);
    println!("  READ_HISTORY:     0x{:08X}", ChannelPermissions::READ_HISTORY);
    
    let channel_perms = ChannelPermissions {
        bits: ChannelPermissions::SEND_MESSAGES | ChannelPermissions::ADD_MEMBERS
    };
    
    println!("\nExample channel permissions (SEND_MESSAGES | ADD_MEMBERS):");
    println!("  Bits: 0x{:08X}", channel_perms.bits);
    println!("  Can send messages: {}", channel_perms.has(ChannelPermissions::SEND_MESSAGES));
    println!("  Can add members: {}", channel_perms.has(ChannelPermissions::ADD_MEMBERS));
    println!("  Can kick members: {}", channel_perms.has(ChannelPermissions::KICK_MEMBERS));
    
    assert!(channel_perms.has(ChannelPermissions::SEND_MESSAGES));
    assert!(channel_perms.has(ChannelPermissions::ADD_MEMBERS));
    assert!(!channel_perms.has(ChannelPermissions::KICK_MEMBERS));
    
    println!("\n✅ Channel permissions work independently!");
}

#[test]
fn demo_8_space_manager_integration() {
    println!("\n=== DEMO 8: SpaceManager Integration ===\n");
    
    let mut manager = SpaceManager::new();
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "managed-space", 1000);
    
    // Create provider
    let provider = spaceway_core::mls::provider::create_provider();
    
    // Create space through manager
    let result = manager.create_space(
        space_id,
        "Managed Space".to_string(),
        Some("Created through SpaceManager".to_string()),
        owner,
        &owner_keypair,
        &provider,
    );
    
    assert!(result.is_ok(), "Space creation failed");
    println!("✓ Space created through SpaceManager");
    
    let space = manager.get_space(&space_id).unwrap();
    
    println!("  Space name: {}", space.name);
    println!("  Owner: {:?}", &space.owner.0[..8]);
    println!("  Default roles: {}", space.roles.len());
    
    // Verify roles
    assert_eq!(space.roles.len(), 3);
    println!("  ✓ Admin role: {}", space.roles.values().any(|r| r.name == "Admin"));
    println!("  ✓ Moderator role: {}", space.roles.values().any(|r| r.name == "Moderator"));
    println!("  ✓ Member role: {}", space.roles.values().any(|r| r.name == "Member"));
    
    // Verify owner permissions
    assert!(space.can_create_channels(&owner));
    assert!(space.can_manage_roles(&owner));
    println!("  ✓ Owner has all permissions");
    
    println!("\n✅ SpaceManager integration works!");
}

#[test]
fn demo_9_permission_grant_revoke() {
    println!("\n=== DEMO 9: Granting and Revoking Permissions ===\n");
    
    let mut perms = SpacePermissions::none();
    println!("Starting with no permissions: 0x{:08X}", perms.bits);
    
    // Grant permissions one by one
    perms.grant(SpacePermissions::CREATE_CHANNELS);
    println!("After granting CREATE_CHANNELS: 0x{:08X}", perms.bits);
    assert!(perms.has(SpacePermissions::CREATE_CHANNELS));
    
    perms.grant(SpacePermissions::INVITE_MEMBERS);
    println!("After granting INVITE_MEMBERS: 0x{:08X}", perms.bits);
    assert!(perms.has(SpacePermissions::INVITE_MEMBERS));
    
    // Revoke a permission
    perms.revoke(SpacePermissions::CREATE_CHANNELS);
    println!("After revoking CREATE_CHANNELS: 0x{:08X}", perms.bits);
    assert!(!perms.has(SpacePermissions::CREATE_CHANNELS));
    assert!(perms.has(SpacePermissions::INVITE_MEMBERS));
    
    println!("\n✅ Grant and revoke operations work!");
}

#[test]
fn demo_10_realistic_gaming_server() {
    println!("\n=== DEMO 10: Realistic Gaming Server Scenario ===\n");
    
    let temp_user = UserId::new();
    let space_id = SpaceId::from_content(&temp_user, "gaming-community", 1000);
    let owner_keypair = Keypair::generate();
    let owner = owner_keypair.user_id();
    
    let mut space = Space::new(
        space_id,
        "Gaming Community".to_string(),
        Some("Welcome to our gaming server!".to_string()),
        owner,
        1000,
    );
    
    println!("🎮 Gaming Community Server");
    println!("   Owner: {:?}", &owner.0[..8]);
    
    // Create VIP role (supporters)
    let vip_role = SpaceRole {
        id: RoleId::new(),
        name: "VIP".to_string(),
        permissions: SpacePermissions {
            bits: SpacePermissions::INVITE_MEMBERS | SpacePermissions::PIN_MESSAGES
        },
        position: 25,
        color: Some(0xFFD700), // Gold
    };
    
    // Create Event Organizer role
    let organizer_role = SpaceRole {
        id: RoleId::new(),
        name: "Event Organizer".to_string(),
        permissions: SpacePermissions {
            bits: SpacePermissions::CREATE_CHANNELS 
                | SpacePermissions::MANAGE_CHANNELS
                | SpacePermissions::INVITE_MEMBERS
        },
        position: 60,
        color: Some(0x9B59B6), // Purple
    };
    
    let vip_id = vip_role.id;
    let organizer_id = organizer_role.id;
    
    space.roles.insert(vip_id, vip_role);
    space.roles.insert(organizer_id, organizer_role);
    
    println!("\n📋 Custom roles created:");
    println!("   🌟 VIP (position 25)");
    println!("   📅 Event Organizer (position 60)");
    
    // Assign roles to users
    let vip_user = Keypair::generate().user_id();
    let organizer_user = Keypair::generate().user_id();
    let regular_user = Keypair::generate().user_id();
    
    space.assign_role(vip_user, vip_id).unwrap();
    space.assign_role(organizer_user, organizer_id).unwrap();
    
    let member_role_id = *space.roles.iter().find(|(_, r)| r.name == "Member").unwrap().0;
    space.assign_role(regular_user, member_role_id).unwrap();
    
    println!("\n🎯 Testing scenarios:");
    
    println!("\n1. Can VIP create channels?");
    let result = space.can_create_channels(&vip_user);
    println!("   Result: {} (VIPs don't manage channels)", result);
    assert!(!result);
    
    println!("\n2. Can Event Organizer create channels?");
    let result = space.can_create_channels(&organizer_user);
    println!("   Result: {} (Organizers can create event channels)", result);
    assert!(result);
    
    println!("\n3. Can regular member invite friends?");
    let result = space.can_invite_members(&regular_user);
    println!("   Result: {} (Members can invite)", result);
    assert!(result);
    
    println!("\n4. Can VIP invite friends?");
    let result = space.can_invite_members(&vip_user);
    println!("   Result: {} (VIP perk)", result);
    assert!(result);
    
    println!("\n✅ Gaming server scenario works perfectly!");
}
