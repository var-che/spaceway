# Test Directory

This directory contains automated tests and test utilities for Spaceway.

## Directory Structure

```
tests/
├── scripts/          # Test automation scripts
│   ├── test-*.py    # Python test scripts for E2EE scenarios
│   ├── test-*.sh    # Shell scripts for testing
│   └── *.sh         # Helper scripts
└── logs/            # Test output logs (gitignored)
    ├── alice_*.log  # Alice's test logs
    ├── bob_*.log    # Bob's test logs
    └── charlie_*.log # Charlie's test logs
```

## Available Tests

### E2EE Tests

- **test-bidirectional.py** - Tests two-way encrypted messaging between Alice and Bob
- **test-e2ee.py** - Basic end-to-end encryption test
- **test-kick-member.py** - Tests member removal and key rotation
- **test-three-members.py** - Tests three-way encrypted group communication
- **test-three-members-kick.py** - Tests member removal in a 3-person group

### Running Tests

From the project root:

```bash
# Run a specific test
python3 tests/scripts/test-bidirectional.py

# Run automation suite
python3 tests/scripts/test-automation.py
```

### Test Logs

Logs are automatically generated in `tests/logs/` during test runs. These are gitignored to keep the repository clean.

## Notes

- All tests use the debug binary from `target/debug/spaceway`
- Tests create temporary key files (\*.key) that are gitignored
- Each test cleans up its data directories on start
