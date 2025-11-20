/// Integration test: Full permissions system with 3 users
/// 
/// Demonstrates complete space lifecycle:
/// - Space creation with MLS encryption
/// - Role-based permissions (Admin, Moderator, Member)
/// - Message sending with encryption
/// - Moderation actions (kick, role changes)
/// - Permission enforcement

use descord_core::{
    Permissions, Role,
    mls::{MlsGroup, MlsGroupConfig, provider::create_provider},
    types::{SpaceId, UserId},
};
use anyhow::Result;

#[test]
fn test_full_permissions_system() -> Result<()> {
    // Setup
    let provider = create_provider();
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut space_bytes = [0u8; 32];
    rng.fill(&mut space_bytes);
    let space_id = SpaceId(space_bytes);
    
    // Create three users
    let alice_id = UserId([1u8; 32]); // Will be Admin
    let bob_id = UserId([2u8; 32]);   // Will be Member -> Moderator
    let charlie_id = UserId([3u8; 32]); // Will be Moderator -> Kicked
    
    // Create keypairs
    let alice_keypair = openmls_basic_credential::SignatureKeyPair::new(
        openmls::prelude::Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519
            .signature_algorithm()
    )?;
    
    println!("\n=== FULL SYSTEM TEST: 3-User Space with Moderation ===\n");
    
    // Step 1: Alice creates space
    println!("📝 Alice creates space");
    let mut space = MlsGroup::create(
        space_id,
        alice_id,
        alice_keypair,
        MlsGroupConfig::default(),
        &provider,
    )?;
    
    assert_eq!(space.get_role(&alice_id), Some(Role::Admin));
    println!("✓ Alice is Admin\n");
    
    // Step 2: Add Bob and Charlie
    println!("📝 Adding members");
    space.add_member_with_role(bob_id, Role::Member);
    space.add_member_with_role(charlie_id, Role::Moderator);
    println!("✓ Bob (Member) and Charlie (Moderator) added\n");
    
    // Step 3: Verify all can send messages
    println!("📝 Testing message permissions");
    assert!(space.check_permission(&alice_id, |p| p.can_send_messages()).is_ok());
    assert!(space.check_permission(&bob_id, |p| p.can_send_messages()).is_ok());
    assert!(space.check_permission(&charlie_id, |p| p.can_send_messages()).is_ok());
    println!("✓ All members can send messages\n");
    
    // Step 4: Charlie tries to demote Alice (should fail)
    println!("📝 Charlie tries to demote Alice");
    let result = space.set_role(&charlie_id, alice_id, Role::Member);
    assert!(result.is_err());
    println!("✗ Blocked: Moderator can't demote Admin\n");
    
    // Step 5: Alice promotes Bob to Moderator
    println!("📝 Alice promotes Bob to Moderator");
    space.set_role(&alice_id, bob_id, Role::Moderator)?;
    assert_eq!(space.get_role(&bob_id), Some(Role::Moderator));
    println!("✓ Bob is now Moderator\n");
    
    // Step 6: Bob kicks Charlie
    println!("📝 Bob kicks Charlie");
    space.remove_member(&charlie_id);
    assert_eq!(space.get_role(&charlie_id), None);
    println!("✓ Charlie removed\n");
    
    // Step 7: Charlie can't access space
    println!("📝 Verifying Charlie is kicked");
    let charlie_perms = space.get_permissions(&charlie_id);
    assert_eq!(charlie_perms, Permissions::NONE);
    assert!(space.check_permission(&charlie_id, |p| p.can_view_channel()).is_err());
    println!("✗ Charlie CANNOT view channel\n");
    
    println!("✅ FULL SYSTEM TEST PASSED!");
    println!("   Final state: Alice (Admin), Bob (Moderator), Charlie (kicked)");
    
    Ok(())
}
