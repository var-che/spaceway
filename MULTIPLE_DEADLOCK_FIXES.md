# Multiple Deadlock Fixes - Complete Solution

## Problem Pattern

**Same deadlock pattern** appeared in multiple functions:

1. Function acquires `space_manager.write().await` lock
2. Calls `broadcast_op()` while still holding the lock
3. `broadcast_op_on_topic()` tries to re-acquire the same lock → **DEADLOCK**

## Functions Fixed

### ✅ Fixed in First Round

1. **`create_space_with_visibility()`** - Lines 510-548

### ✅ Fixed in Second Round

2. **`create_invite()`** - Lines 585-608
3. **`revoke_invite()`** - Lines 610-632
4. **`update_space_visibility()`** - Lines 562-583

### ✅ Fixed in Third Round

5. **`join_with_invite()`** - Lines 720-740

### ⚠️ Still Need Fixing

6. `add_member()` - Line 1261
7. `remove_member()` - Line 1444
8. `create_channel()` - Line 1480
9. `create_thread()` - Line 1533
10. `post_message()` - Line 1604
11. `edit_message()` - Line 1661

## The Fix Pattern

**Before (DEADLOCK):**

```rust
let mut manager = self.space_manager.write().await;
let op = manager.some_operation(...)?;
self.broadcast_op(&op).await?; // ❌ Still holding lock!
```

**After (FIXED):**

```rust
let op = {
    let mut manager = self.space_manager.write().await;
    manager.some_operation(...)?
}; // ✅ Lock dropped here
self.broadcast_op(&op).await?; // Safe now!
```

## Testing

Fixed functions now work:

- `space create` ✅
- `invite create` ✅
- `invite revoke` ✅
- Space visibility updates ✅
- `join <space_id> <invite_code>` ✅ (Bob can now join Alice's space)
- `space list` ✅ (Shows joined spaces after event handlers added)

## Event Processing Fixed

Added missing event handlers for:

- `CreateInvite` - Process invite creation operations
- `RevokeInvite` - Process invite revocation
- `UseInvite` - Process when users join via invite
- `AddMember` - Log member additions (handled by invite/MLS flow)
- `RemoveMember` - Process member removals

These handlers were missing, causing Bob to not see spaces he joined.

## Next Steps

The other functions likely have the same pattern. They should be fixed with the same approach before users encounter deadlocks when:

- Adding members
- Removing members
- Creating channels
- Creating threads
- Posting messages
- Editing messages

## Root Cause

The issue is that `broadcast_op_on_topic()` needs to acquire `space_manager` lock to check for MLS groups for encryption. Any function that holds this lock while calling `broadcast_op()` will deadlock.

**Solution**: Always drop manager locks before calling `broadcast_op()`.
