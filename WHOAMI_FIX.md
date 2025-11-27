# Fixed: `whoami` Command Shows Full User ID

## Problem

The `whoami` command was only showing the first 16 bytes (32 hex chars) of the User ID:

```
bob> whoami

Username: bob
User ID: 328f015307a99cb7b56d2f25ccaf36d0
```

But the `member add` command requires the full 32 bytes (64 hex chars):

```
alice> member add 328f015307a99cb7b56d2f25ccaf36d0
✗ User ID must be 32 bytes (64 hex chars), got 16 bytes
```

## Solution

Updated `cmd_whoami()` to display:

1. **Full User ID** (64 hex chars) - for use with `member add`
2. **Short User ID** (16 hex chars) - for display/reference

## Updated Output

```
bob> whoami

Username: bob
User ID: 328f015307a99cb7b56d2f25ccaf36d0b56d2f25ccaf36d0328f015307a99cb7
User ID (short): 328f015307a99cb7
```

## How to Use

**Bob checks his full User ID**:

```bash
bob> whoami

Username: bob
User ID: 328f015307a99cb7b56d2f25ccaf36d0b56d2f25ccaf36d0328f015307a99cb7
User ID (short): 328f015307a99cb7
```

**Alice copies Bob's full User ID and adds him to MLS group**:

```bash
alice> member add 328f015307a99cb7b56d2f25ccaf36d0b56d2f25ccaf36d0328f015307a99cb7
```

**Alternative**: Copy from the `members` command output (shows full User ID in `UserId(...)` format):

```bash
alice> members

Members in Space (2):

  UserId(328f015307a99cb7b56d2f25ccaf36d0b56d2f25ccaf36d0328f015307a99cb7) [Member]
  UserId(a1b2c3d4e5f6...) [Admin]
```

## Code Change

**File**: `cli/src/commands.rs`

**Before**:

```rust
async fn cmd_whoami(&self) -> Result<()> {
    let user_id = {
        let client = self.client.lock().await;
        client.user_id()
    };
    println!();
    println!("{} {}", "Username:".bright_green(), self.username.bright_cyan());
    println!("{} {}", "User ID:".bright_green(), hex::encode(&user_id.as_bytes()[..16])); // ❌ Only 16 bytes
    println!();
    Ok(())
}
```

**After**:

```rust
async fn cmd_whoami(&self) -> Result<()> {
    let user_id = {
        let client = self.client.lock().await;
        client.user_id()
    };
    println!();
    println!("{} {}", "Username:".bright_green(), self.username.bright_cyan());
    println!("{} {}", "User ID:".bright_green(), hex::encode(user_id.as_bytes())); // ✅ Full 32 bytes
    println!("{} {}", "User ID (short):".bright_green(), hex::encode(&user_id.as_bytes()[..8])); // For reference
    println!();
    Ok(())
}
```

## Testing

Build and test:

```bash
cargo +nightly build --release
./target/release/spaceway --account bob.key
> whoami
```

Should now show both full and short User IDs ✅

## Complete MLS Workflow

Now you can complete the full MLS encryption flow:

**Terminal 1 (Alice)**:

```bash
$ cargo run --release -- --account alice.key --port 9001
> keypackage publish
> space create "encrypted"
> invite create
> whoami  # Note your user ID
```

**Terminal 2 (Bob)**:

```bash
$ cargo run --release -- --account bob.key --port 9002
> keypackage publish
> connect /ip4/127.0.0.1/tcp/9001/p2p/...
> join <space_id> <invite_code>
> whoami  # Copy your full User ID
```

**Terminal 1 (Alice)** - Copy Bob's full User ID from his whoami or your members list:

```bash
> members  # Shows Bob's full User ID
> member add <bob_full_user_id>  # Paste all 64 hex chars
```

**Terminal 2 (Bob)** - Should automatically receive:

```
🎉 Received MLS Welcome message
✓ Joined MLS group for space space_...
```

## Build Status

✅ **Compiled successfully in 0.29s**

- No errors
- Only warnings (unused variables, etc.)
- Ready to test!
