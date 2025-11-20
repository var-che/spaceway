# Fast Test Running Guide

## Problem
`cargo test --lib` takes 7+ minutes because it runs all 57+ tests sequentially.

## Solutions

### 1. Run Specific Test Files (Fastest - Seconds)

```powershell
# Test only privacy features (7 tests, ~3 seconds)
cargo test --test privacy_tiers_test

# Test only invites (11 tests, ~5 seconds)
cargo test --test invite_system_test

# Test only visibility (5 tests, ~2 seconds)
cargo test --test space_visibility_test

# Test only integration (11 tests, ~10 seconds)
cargo test --test integration_test
```

### 2. Run Specific Individual Tests (Ultra-fast)

```powershell
# Run just one test by name
cargo test test_public_space_privacy_info

# Run tests matching a pattern
cargo test privacy_info

# Run all tests with "create" in the name
cargo test create
```

### 3. Run Only Unit Tests (Skip Integration - 2 minutes)

```powershell
# Skip the slow integration/convergence tests
cargo test --lib -- --skip convergence --skip integration
```

### 4. Parallel Execution (Use all CPU cores)

```powershell
# Run tests in parallel (default, but explicit)
cargo test -- --test-threads=8

# For flaky network tests, serialize:
cargo test -- --test-threads=1
```

### 5. Quick Smoke Test (30 seconds)

```powershell
# Just check if code compiles and one test passes
cargo test --test privacy_tiers_test test_public_space_privacy_info
```

## Recommended Workflow

### During Development (What you're changing)
```powershell
# If working on privacy features:
cargo test --test privacy_tiers_test

# If working on invites:
cargo test --test invite_system_test

# If working on relay:
cargo test relay  # Runs all tests with "relay" in name
```

### Before Committing (Full validation)
```powershell
# Run everything (7 minutes)
cargo test
```

### CI/CD (Automated)
```powershell
# Full test suite with verbose output
cargo test --all -- --nocapture
```

## Test File Breakdown (by speed)

| Test File | Tests | Time | What It Tests |
|-----------|-------|------|---------------|
| `space_visibility_test.rs` | 5 | ~2s | Public/Private/Hidden spaces |
| `privacy_tiers_test.rs` | 7 | ~3s | Privacy warnings, transport modes |
| `invite_system_test.rs` | 11 | ~5s | Invite codes, expiration, permissions |
| `three_person_test.rs` | 1 | ~8s | Multi-user convergence |
| `integration_test.rs` | 11 | ~10s | Network operations |
| Unit tests (in `src/`) | 57 | ~90s | CRDT, storage, MLS, relay |

**Slowest tests:**
- `test_concurrent_operations` (~60s) - Simulates race conditions
- `test_eventual_consistency` (~60s) - Network convergence

## Quick Test Commands (Copy-Paste)

```powershell
# Privacy features only
cargo test --test privacy_tiers_test

# Network/relay features
cargo test relay

# Quick sanity check
cargo test --lib -- --skip convergence

# Single test (replace with actual test name)
cargo test test_create_invite

# All tests, verbose
cargo test -- --nocapture
```

## PowerShell Function (Add to your profile)

```powershell
# Add to $PROFILE
function Test-Descord {
    param(
        [Parameter()]
        [ValidateSet('quick', 'privacy', 'invite', 'network', 'all')]
        [string]$Suite = 'quick'
    )
    
    switch ($Suite) {
        'quick'   { cargo test --lib -- --skip convergence }
        'privacy' { cargo test --test privacy_tiers_test }
        'invite'  { cargo test --test invite_system_test }
        'network' { cargo test relay }
        'all'     { cargo test }
    }
}

# Usage:
# Test-Descord -Suite privacy
# Test-Descord -Suite quick
# Test-Descord -Suite all
```

## VS Code Tasks (`.vscode/tasks.json`)

```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Test: Quick",
            "type": "shell",
            "command": "cargo",
            "args": ["test", "--lib", "--", "--skip", "convergence"],
            "group": "test",
            "problemMatcher": "$rustc"
        },
        {
            "label": "Test: Privacy",
            "type": "shell",
            "command": "cargo",
            "args": ["test", "--test", "privacy_tiers_test"],
            "group": "test"
        },
        {
            "label": "Test: All",
            "type": "shell",
            "command": "cargo",
            "args": ["test"],
            "group": {
                "kind": "test",
                "isDefault": true
            }
        }
    ]
}
```

Then: `Ctrl+Shift+P` → "Run Test Task" → Select "Test: Quick"

## Measuring Test Times

```powershell
# Benchmark each test file
Measure-Command { cargo test --test privacy_tiers_test }
Measure-Command { cargo test --test invite_system_test }
Measure-Command { cargo test --lib }
```

## Why Tests Are Slow

1. **CRDT Convergence Tests** (~120s total)
   - Simulate concurrent operations with delays
   - Test eventual consistency with sleep timers
   - Can't be parallelized (test race conditions)

2. **Network Tests** (~30s)
   - Spin up libp2p swarms
   - mDNS discovery waits for broadcasts
   - TCP connection establishment

3. **MLS Encryption** (~20s)
   - Key generation is CPU-intensive
   - Group operations require crypto

4. **Storage I/O** (~10s)
   - Even with tempdir, disk writes are slow
   - RocksDB initialization overhead

## Optimization Ideas (Future)

- [ ] Mock network layer for unit tests
- [ ] Reduce sleep/timeout values in convergence tests
- [ ] Use in-memory storage for non-I/O tests
- [ ] Split slow tests into separate `tests/slow/` directory
- [ ] Add `#[ignore]` to slow tests, run with `--ignored` only in CI

## Bottom Line

**For daily development:**
```powershell
cargo test --test privacy_tiers_test  # 3 seconds
```

**Before pushing:**
```powershell
cargo test  # 7 minutes, get coffee ☕
```
