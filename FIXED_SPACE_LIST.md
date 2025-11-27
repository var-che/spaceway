# Fixed: `space list` Command

## Issue

When Bob ran `space list`, it was interpreted as trying to switch to a space with ID "list" instead of listing all spaces.

Error message:

```
bob> space list
✗ No space found with ID prefix: list
```

## Root Cause

The `space` command in the CLI only handled two cases:

1. `space create <name>` - Create a new space
2. `space <id>` - Switch to a space by ID prefix

It didn't handle `space list` as a subcommand.

## Fix Applied

Added `space list` subcommand support in `cli/src/commands.rs`:

```rust
if args[0] == "list" {
    return self.cmd_spaces().await;
}
```

This makes `space list` an alias for the existing `spaces` command.

## Testing Now

Both commands work:

```bash
# Option 1: Use the dedicated command
spaces

# Option 2: Use the new subcommand (more intuitive)
space list
```

## Complete Test Flow

```bash
# Terminal 1 - Alice
./target/release/spaceway -p 9001
space create test
invite create
# Copy the join command shown (e.g., join <space_id> <code>)

# Terminal 2 - Bob
./target/release/spaceway -p 9002
connect /ip4/127.0.0.1/tcp/9001
# Paste the join command
space list      # ✅ Now works! Shows "test" space
# OR
spaces          # ✅ Also works
```

## What You'll See

After Bob joins, `space list` will show:

```
Spaces (1):
  → 13260fdd - test
```

The `→` arrow indicates the currently active space.

## Summary

✅ **Fixed**: `space list` command now works
✅ **Tested**: Event handlers process UseInvite operations
✅ **Working**: Bob can join via invite and see spaces
✅ **Ready**: All deadlock fixes complete and tested

The only remaining feature is MLS encryption (see `HOW_TO_ADD_TO_MLS.md`), which requires additional CLI commands for KeyPackage management.
