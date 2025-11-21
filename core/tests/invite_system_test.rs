use spaceway_core::{Client, ClientConfig, crypto::Keypair};
use spaceway_core::types::{SpaceVisibility, InviteCreatorRole, InvitePermissions};
use anyhow::Result;

/// Helper to create a test client
fn create_test_client(_name: &str) -> Result<Client> {
    let keypair = Keypair::generate();
    let temp_dir = tempfile::tempdir().unwrap();
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".to_string()],
        bootstrap_peers: vec![],
    };
    
    Ok(Client::new(keypair, config)?)
}

#[tokio::test]
async fn test_create_invite() -> Result<()> {
    let admin = create_test_client("test_create_invite")?;
    let (space, _, _) = admin.create_space(
        "Test Space".to_string(),
        Some("Test description".to_string()),
    ).await?;
    
    // Admin creates an invite
    let invite_op = admin.create_invite(
        space.id,
        Some(10),  // max 10 uses
        Some(24),  // expires in 24 hours
    ).await?;
    
    assert!(invite_op.op_id.0.as_bytes().len() > 0);
    
    // Check the invite was created
    let invites = admin.list_invites(&space.id).await;
    assert_eq!(invites.len(), 1);
    
    let invite = &invites[0];
    assert_eq!(invite.space_id, space.id);
    assert_eq!(invite.creator, admin.user_id());
    assert_eq!(invite.code.len(), 8); // 8-character code
    assert_eq!(invite.max_uses, Some(10));
    assert_eq!(invite.uses, 0);
    assert!(!invite.revoked);
    
    Ok(())
}

#[tokio::test]
async fn test_create_unlimited_invite() -> Result<()> {
    let admin = create_test_client("test_unlimited_invite")?;
    let (space, _, _) = admin.create_space(
        "Test Space".to_string(),
        None,
    ).await?;
    
    // Create invite with no limits
    admin.create_invite(
        space.id,
        None,  // unlimited uses
        None,  // never expires
    ).await?;
    
    let invites = admin.list_invites(&space.id).await;
    assert_eq!(invites.len(), 1);
    
    let invite = &invites[0];
    assert_eq!(invite.max_uses, None);
    assert_eq!(invite.expires_at, None);
    
    Ok(())
}

#[tokio::test]
async fn test_revoke_invite() -> Result<()> {
    let admin = create_test_client("test_revoke_invite")?;
    let (space, _, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Create invite
    admin.create_invite(space.id, Some(5), Some(24)).await?;
    
    let invites = admin.list_invites(&space.id).await;
    let invite_id = invites[0].id;
    
    // Revoke it
    admin.revoke_invite(space.id, invite_id).await?;
    
    // Check it's revoked
    let invites = admin.list_invites(&space.id).await;
    assert_eq!(invites.len(), 1);
    assert!(invites[0].revoked);
    
    Ok(())
}

#[tokio::test]
async fn test_join_with_invite() -> Result<()> {
    let admin = create_test_client("test_join_admin")?;
    let joiner = create_test_client("test_join_user")?;
    
    // Admin creates private space
    let (space, space_op, _) = admin.create_space_with_visibility(
        "Private Space".to_string(),
        None,
        SpaceVisibility::Private,
    ).await?;
    
    // Joiner receives the space creation operation
    joiner.handle_incoming_op(space_op).await?;
    
    // Admin creates invite
    let invite_op = admin.create_invite(space.id, Some(1), Some(24)).await?;
    
    // Joiner receives the invite creation operation
    joiner.handle_incoming_op(invite_op).await?;
    
    let invites = joiner.list_invites(&space.id).await;
    let invite_code = invites[0].code.clone();
    
    // Joiner uses the invite
    joiner.join_with_invite(space.id, invite_code).await?;
    
    // Verify joiner is now a member
    let joiner_spaces = joiner.list_spaces().await;
    assert_eq!(joiner_spaces.len(), 1);
    assert_eq!(joiner_spaces[0].id, space.id);
    
    // Verify invite use count incremented
    let invites = joiner.list_invites(&space.id).await;
    assert_eq!(invites[0].uses, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_invite_max_uses() -> Result<()> {
    let admin = create_test_client("test_max_uses_admin")?;
    let joiner1 = create_test_client("test_max_uses_user1")?;
    let joiner2 = create_test_client("test_max_uses_user2")?;
    
    let (space, space_op, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Both joiners receive the space creation operation
    joiner1.handle_incoming_op(space_op.clone()).await?;
    joiner2.handle_incoming_op(space_op).await?;
    
    // Create invite with max 1 use
    let invite_op = admin.create_invite(space.id, Some(1), None).await?;
    
    // Both joiners receive the invite
    joiner1.handle_incoming_op(invite_op.clone()).await?;
    joiner2.handle_incoming_op(invite_op).await?;
    
    let invites = joiner1.list_invites(&space.id).await;
    let invite_code = invites[0].code.clone();
    
    // First join should succeed
    let result1 = joiner1.join_with_invite(space.id, invite_code.clone()).await;
    assert!(result1.is_ok());
    
    // Joiner2 receives the use invite operation
    if let Ok(use_op) = result1 {
        joiner2.handle_incoming_op(use_op).await?;
    }
    
    // Second join should fail (max uses reached)
    let result2 = joiner2.join_with_invite(space.id, invite_code.clone()).await;
    assert!(result2.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_revoked_invite_cannot_be_used() -> Result<()> {
    let admin = create_test_client("test_revoked_admin")?;
    let joiner = create_test_client("test_revoked_user")?;
    
    let (space, _, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Create and immediately revoke invite
    admin.create_invite(space.id, Some(5), None).await?;
    let invites = admin.list_invites(&space.id).await;
    let invite_id = invites[0].id;
    let invite_code = invites[0].code.clone();
    
    admin.revoke_invite(space.id, invite_id).await?;
    
    // Try to use revoked invite
    let result = joiner.join_with_invite(space.id, invite_code).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_invalid_invite_code() -> Result<()> {
    let admin = create_test_client("test_invalid_admin")?;
    let joiner = create_test_client("test_invalid_user")?;
    
    let (space, _, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Try to use non-existent invite code
    let result = joiner.join_with_invite(space.id, "INVALID1".to_string()).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_already_member_cannot_use_invite() -> Result<()> {
    let admin = create_test_client("test_already_member_admin")?;
    
    let (space, _, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Admin creates invite
    admin.create_invite(space.id, Some(5), None).await?;
    let invites = admin.list_invites(&space.id).await;
    let invite_code = invites[0].code.clone();
    
    // Admin tries to use their own invite (already a member)
    let result = admin.join_with_invite(space.id, invite_code).await;
    assert!(result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_invites() -> Result<()> {
    let admin = create_test_client("test_multiple_invites")?;
    let (space, _, _) = admin.create_space("Test Space".to_string(), None).await?;
    
    // Create multiple invites
    admin.create_invite(space.id, Some(1), Some(24)).await?;
    admin.create_invite(space.id, Some(5), Some(48)).await?;
    admin.create_invite(space.id, None, None).await?;
    
    let invites = admin.list_invites(&space.id).await;
    assert_eq!(invites.len(), 3);
    
    // Each should have unique code
    let codes: Vec<_> = invites.iter().map(|i| i.code.clone()).collect();
    assert_eq!(codes.len(), 3);
    assert_ne!(codes[0], codes[1]);
    assert_ne!(codes[1], codes[2]);
    assert_ne!(codes[0], codes[2]);
    
    Ok(())
}

#[tokio::test]
async fn test_invite_validation() -> Result<()> {
    use spaceway_core::types::Invite;
    use uuid::Uuid;
    
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Valid invite
    let invite = Invite {
        id: spaceway_core::types::InviteId(Uuid::new_v4()),
        space_id: {
            let mut bytes = [0u8; 32];
            bytes[0] = 1; // Make it non-zero
            spaceway_core::types::SpaceId(bytes)
        },
        creator: spaceway_core::types::UserId([0u8; 32]),
        code: "TEST1234".to_string(),
        max_uses: Some(5),
        expires_at: Some(current_time + 3600),
        uses: 0,
        created_at: current_time,
        revoked: false,
    };
    
    assert!(invite.is_valid(current_time));
    
    // Revoked invite
    let mut revoked = invite.clone();
    revoked.revoked = true;
    assert!(!revoked.is_valid(current_time));
    
    // Expired invite
    let mut expired = invite.clone();
    expired.expires_at = Some(current_time - 1);
    assert!(!expired.is_valid(current_time));
    
    // Max uses reached
    let mut maxed = invite.clone();
    maxed.uses = 5;
    assert!(!maxed.is_valid(current_time));
    
    Ok(())
}

#[tokio::test]
async fn test_invite_permissions() -> Result<()> {
    use spaceway_core::types::Role;
    
    let perms = InvitePermissions {
        who_can_invite: InviteCreatorRole::AdminOnly,
        max_age_hours: Some(24),
        max_uses_default: 10,
    };
    
    assert!(spaceway_core::types::Invite::can_create(Role::Admin, &perms));
    assert!(!spaceway_core::types::Invite::can_create(Role::Moderator, &perms));
    assert!(!spaceway_core::types::Invite::can_create(Role::Member, &perms));
    
    let perms2 = InvitePermissions {
        who_can_invite: InviteCreatorRole::AdminAndModerator,
        max_age_hours: Some(48),
        max_uses_default: 5,
    };
    
    assert!(spaceway_core::types::Invite::can_create(Role::Admin, &perms2));
    assert!(spaceway_core::types::Invite::can_create(Role::Moderator, &perms2));
    assert!(!spaceway_core::types::Invite::can_create(Role::Member, &perms2));
    
    let perms3 = InvitePermissions {
        who_can_invite: InviteCreatorRole::Everyone,
        max_age_hours: None,
        max_uses_default: 1,
    };
    
    assert!(spaceway_core::types::Invite::can_create(Role::Admin, &perms3));
    assert!(spaceway_core::types::Invite::can_create(Role::Moderator, &perms3));
    assert!(spaceway_core::types::Invite::can_create(Role::Member, &perms3));
    
    Ok(())
}
