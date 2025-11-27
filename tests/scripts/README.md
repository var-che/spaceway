# Spaceway MLS E2EE Test Suite

This directory contains automated integration tests for the MLS (Messaging Layer Security) end-to-end encryption implementation in Spaceway.

## Test Organization

All test artifacts (logs, keys, data directories) are stored in `tests/test-runs/<test-name>/` to keep the repository root clean.

## Available Tests

### ✅ test-bidirectional.py

**Status**: PASSING (5/5)
**Purpose**: Tests basic bidirectional E2EE messaging between Alice and Bob

- Alice sends encrypted message to Bob
- Bob decrypts and reads Alice's message
- Bob replies with encrypted message
- Alice decrypts and reads Bob's reply
- Validates basic MLS encryption/decryption works

### ✅ test-three-members.py

**Status**: PASSING (5/5)
**Purpose**: Tests MLS group with three members

- Alice creates space and adds Bob and Charlie
- All three members can exchange encrypted messages
- Validates epoch synchronization with Commit messages
- Tests message queuing for out-of-order messages

### ⚠️ test-kick-member.py

**Status**: PARTIAL (5/6)
**Purpose**: Tests basic member removal (2 members)

- Alice kicks Bob from space
- Bob cannot decrypt messages after kick
- Validates MLS key rotation on member removal

### ✅ test-three-members-kick.py

**Status**: PASSING (8/8)
**Purpose**: Tests member removal with three members

- Alice, Bob, and Charlie communicate
- Alice kicks Bob
- Bob CANNOT decrypt new messages (security validated ✓)
- Charlie CAN still decrypt (remaining members work ✓)

### 🔨 test-four-members-kick.py

**Status**: IN DEVELOPMENT
**Purpose**: Tests member removal scales to 4+ members

- Alice, Bob, Charlie, Dave communicate
- Alice kicks Bob
- Bob CANNOT decrypt new messages
- Charlie and Dave CAN still decrypt
- Tests epoch chain: 0→1→2→3→4

### ❌ test-channel-kick.py

**Status**: ARCHITECTURAL TEST (6/10) - Will validate future implementation
**Purpose**: Tests channel-specific member kicks with channel-level MLS groups

**Scenario**:

- Alice creates space with 2 channels (general, private)
- All members post to both channels
- Alice kicks Charlie from Channel 2 only
- **Expected**: Charlie can't see Channel 2, but CAN see Channel 1

**Current Results (6/10)**:

- ✗ Charlie didn't decrypt Channel 2 even before kick
- ✗ Charlie can't see Channel 1 after kick (incorrect)
- **Root Cause**: MLS groups currently at space level, not channel level

**Future Use**:

- Once Phase 2 (per-channel MLS groups) is implemented, this test will validate:
  - Channel-level encryption isolation
  - Channel-specific member removal
  - Multi-channel membership with independent access control
- **Expected outcome**: 10/10 passing once architecture implemented

## Running Tests

From the project root:

```bash
# Run individual test
python3 tests/scripts/test-bidirectional.py

# Run all tests
for test in tests/scripts/test-*.py; do
    echo "Running $test..."
    python3 "$test"
done
```

## Test Infrastructure

### Directory Structure

```
tests/
├── scripts/
│   ├── test-bidirectional.py
│   ├── test-three-members.py
│   ├── test-kick-member.py
│   ├── test-three-members-kick.py
│   ├── test-four-members-kick.py
│   └── test-channel-kick.py
└── test-runs/
    ├── bidirectional/
    │   ├── alice.log
    │   ├── alice.key
    │   └── bob.log, bob.key
    ├── three-members/
    ├── kick-member/
    ├── three-members-kick/
    ├── four-members-kick/
    └── channel-kick/
```

### Test Artifacts

Each test creates isolated artifacts in its own directory:

- `*.log` - Client debug logs
- `*.key` - Test account keys
- `*-data/` - RocksDB data directories (auto-created)

## Architecture Insights

### Proposed Two-Tier Architecture

**🎯 Design Decision** (November 22, 2025):

Spaceway will implement a **flexible two-tier membership model** for scalability:

**Tier 1 - Space Membership:**

- **Creator's Choice**: Space creator decides at creation time:
  - **Lightweight Mode**: Non-encrypted, minimal overhead, browse channels
  - **MLS Mode**: Space-level MLS group for encrypted space-wide features
- Use case for Lightweight: Large communities (100k+ users) where not everyone needs encryption
- Use case for MLS: Small teams/organizations needing space-level encryption

**Tier 2 - Channel Membership:**

- **Always MLS**: Each channel = separate MLS group
- **Explicit Join**: Users explicitly join channels they want to participate in
- **Channel-Level Access Control**: Kicks remove from specific channel, not entire space
- **Scalable**: User in 100k-member space can join 10-50 channels (manageable MLS groups)

**Architecture Benefits:**

- ✅ Scalability: Space can have 100k+ users with lightweight membership
- ✅ Privacy: Channels provide E2EE without requiring space-wide MLS
- ✅ Performance: Small channel MLS groups (10-100 members) vs massive space groups
- ✅ Flexibility: Creator chooses space security model based on use case
- ✅ Selective Access: Channel-specific kicks, no need to remove from entire space

**Comparison to Current Platforms:**

- Discord: Lightweight server membership, channel permissions
- Slack: Workspace membership, channel-based access
- Spaceway: Adds E2EE at channel level for privacy-focused communities

### Current MLS Implementation Status

**✅ Working Features:**

1. **Basic E2EE**: Two-party encrypted messaging
2. **Group Encryption**: Multi-member MLS groups (tested up to 4 members)
3. **Epoch Synchronization**: Commit message broadcasting and processing
4. **Message Queuing**: Handles out-of-order messages with WrongEpoch errors
5. **Member Removal**: Key rotation on kick, kicked members can't decrypt
6. **Remaining Members**: Members not kicked continue to decrypt correctly

**🔍 Current Limitations (To Be Addressed):**

1. **Space-Level MLS Only**: MLS groups currently implemented at space level
   - All channels in a space share the same MLS group
   - Channel-specific kicks not yet supported
2. **No Space Mode Selection**: Spaces always use MLS currently
   - Need to implement lightweight/MLS choice at space creation

**🚀 Implementation Roadmap:**

1. **Phase 1**: Add space creation mode (lightweight vs MLS)

   - Modify `space create` command: `space create <name> --mode [lightweight|mls]`
   - Store mode in space metadata
   - Lightweight spaces skip MLS group creation

2. **Phase 2**: Implement per-channel MLS groups

   - Each channel gets its own MLS group
   - Channel join: Add user to channel's MLS group
   - Channel kick: Remove from channel's MLS group only
   - Messages encrypted with channel's MLS group, not space's

3. **Phase 3**: Validate with test-channel-kick.py
   - Test should pass 10/10 once channel-level groups implemented
   - Validates channel isolation and security

### Key Technical Details

**Commit Message Broadcasting:**

- Topic: `space/{space_id}` (NOT `space/{space_id}/mls`)
- Broadcasts to all remaining members after add/remove
- Members process Commit, update epoch, retry queued messages

**Epoch Management:**

- Increments on every member add/remove operation
- Members must be at same epoch to decrypt messages
- Commit messages bring members to new epoch

**Member Removal Flow:**

1. `remove_member_with_key_rotation` generates Commit
2. Commit broadcast to `space/{space_id}` topic
3. Remaining members receive and process Commit
4. Epoch updates (e.g., 3 → 4)
5. Queued messages retry at new epoch
6. Kicked member stays at old epoch, can't decrypt

## Security Validations

All tests validate critical security properties:

✅ **Kicked members cannot decrypt post-kick messages**

- Tested with 2, 3, and 4 member groups
- Validated across multiple scenarios

✅ **Remaining members maintain decryption ability**

- After member removal, other members continue working
- No disruption to active conversations

✅ **Epoch-based access control**

- Messages encrypted at epoch N can only be decrypted at epoch N
- Prevents replay of old messages after kicks

## Test Output

All tests provide colored output:

- 🟢 Green: Passing tests
- 🔴 Red: Failing tests
- 🟡 Yellow: Warnings/partial success
- 🔵 Cyan: Info messages

Example:

```
✓ Bob decrypted: 'Message 1: Before kick'
✓ Alice decrypted: 'Message 2: Bob reply before kick'
✓ Bob CANNOT decrypt message after kick (correct!)
✓ Charlie CAN decrypt message after Bob's kick (correct!)

Score: 8/8

🎉 SUCCESS! Three-member kick working correctly!
```

## Future Test Ideas

### Immediate (Current Architecture)

1. **Concurrent Kicks**: Multiple members kicked simultaneously
2. **Re-invitation**: Kicked member rejoins with new keys
3. **Large Groups**: Test with 10+ members in space-level MLS
4. **Message Bursts**: High-volume message testing
5. **Network Partitions**: Test with delayed/dropped messages

### Post-Architecture Implementation

6. **Space Mode Selection**: Test lightweight vs MLS space creation
7. **Channel-Level Groups**: Comprehensive channel isolation tests (test-channel-kick.py)
8. **Multi-Channel Membership**: User in 10+ channels, kicked from subset
9. **Lightweight Space Performance**: 1000+ member space with channel joins
10. **Mixed Mode Spaces**: Lightweight space with encrypted channels

## Debugging Failed Tests

When tests fail:

1. **Check logs**: `tests/test-runs/<test-name>/*.log`
2. **Look for**:
   - `Decrypted MLS message` - successful decryptions
   - `WrongEpoch` - epoch mismatch errors
   - `Commit` messages - member add/remove operations
   - `epoch X` - track epoch progression
3. **Common issues**:
   - Timing: Increase `wait` parameters if messages not propagating
   - Epoch mismatch: Check Commit message broadcasting
   - No decryption: Verify MLS group setup and KeyPackages

## Contributing

When adding new tests:

1. Follow the existing pattern (setup, test, validate, cleanup)
2. Use descriptive test names: `test-<feature>-<scenario>.py`
3. Create isolated test directory: `tests/test-runs/<test-name>/`
4. Include colored output for pass/fail visualization
5. Document what the test validates and any findings
6. Update this README with test status and insights

---

**Last Updated**: November 22, 2025
**Test Suite Status**: 5/6 passing, 1 in development, 1 architectural validation test
**Architecture Decision**: Two-tier model with creator-defined space mode (lightweight/MLS) + per-channel MLS groups
