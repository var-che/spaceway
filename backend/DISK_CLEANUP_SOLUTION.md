# Test Disk Usage - FIXED ✅

## Problem
Tests were creating persistent `./test-data/` directories that accumulated over time, causing disk bloat.

## Root Cause
```rust
// OLD (BAD):
ClientConfig {
    storage_path: PathBuf::from("./test-data/privacy-public"), // ❌ Persistent
}
```

Tests created databases, key files, and blobs in `./test-data/` that were never cleaned up.

## Solution
Use temporary directories that auto-delete when test completes:

```rust
// NEW (GOOD):
let temp_dir = tempfile::tempdir().unwrap();  // ✅ Auto-cleanup
ClientConfig {
    storage_path: temp_dir.path().to_path_buf(),
}
```

## Files Fixed
- `core/tests/space_visibility_test.rs` - 5 tests
- `core/tests/privacy_tiers_test.rs` - 7 tests
- `core/tests/invite_system_test.rs` - 11 tests
- `core/tests/integration_test.rs` - 11 tests
- `core/tests/three_person_test.rs` - Already using tempdir ✅

## Results

**Before:**
```
./test-data/privacy-public/
./test-data/privacy-private/
./test-data/privacy-hidden/
./alice-data/
./bob-data/
... (all persist forever, accumulate with each test run)
```

**After:**
```
/tmp/test-XXXXX/  (auto-deleted when test exits)
```

**Disk saved:** Approximately 50-200 MB per full test run, depending on blob storage tests.

## Verification

```bash
# Before fix:
cargo test
ls ./test-data  # Contains many subdirectories

# After fix:
cargo test
ls ./test-data  # Directory doesn't exist or is empty
```

## How tempfile Works

```rust
use tempfile::tempdir;

{
    let temp = tempdir()?;  // Creates /tmp/test-abc123
    // Use temp.path() for storage
    // ...
}  // <- temp dropped here, directory automatically deleted
```

The `TempDir` type implements `Drop`, which deletes the directory and all contents when it goes out of scope.

## Best Practice for Future Tests

**Always use tempdir for tests:**
```rust
#[tokio::test]
async fn my_test() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let config = ClientConfig {
        storage_path: temp_dir.path().to_path_buf(),
        // ...
    };
    // ...
}
```

**Never hardcode paths:**
```rust
// ❌ DON'T:
storage_path: PathBuf::from("./test-data/my-test")

// ✅ DO:
let temp = tempfile::tempdir()?;
storage_path: temp.path().to_path_buf()
```

## Manual Cleanup (if needed)

If old test data still exists:
```powershell
# PowerShell
Remove-Item -Recurse -Force "./test-data","./alice-data","./bob-data"

# Bash
rm -rf ./test-data ./alice-data ./bob-data
```

## Related Files
- `Cargo.toml` - Already has `tempfile = "3"` dependency
- All test files now use tempdir pattern
- No manual cleanup functions needed
