# Automated Testing Framework for Spaceway

This directory contains automated testing scripts that eliminate the need for manual terminal management when testing the MLS E2EE workflow.

## Available Scripts

### 1. `test-automation.py` (Recommended)

**Python-based test automation** - More reliable and easier to debug

**Features:**

- ✅ Automatic client startup and management
- ✅ Automated command execution
- ✅ Log capture to files
- ✅ Comprehensive test validation
- ✅ Color-coded output
- ✅ Detailed error reporting

**Usage:**

```bash
chmod +x test-automation.py
./test-automation.py
```

**Output Files:**

- `alice_output.log` - All of Alice's terminal output
- `bob_output.log` - All of Bob's terminal output
- `test_results.txt` - Summary of test results

### 2. `test-automation.sh`

**Bash-based test automation** - Alternative implementation

**Usage:**

```bash
chmod +x test-automation.sh
./test-automation.sh
```

## What Gets Tested

The automation runs the complete MLS E2EE workflow:

### Phase 1: Setup

1. ✓ Clean old test data
2. ✓ Build Spaceway (release mode)
3. ✓ Start Alice (port 9001)
4. ✓ Start Bob (port 9002)

### Phase 2: KeyPackage Publication

5. ✓ Alice publishes KeyPackages to DHT
6. ✓ Bob publishes KeyPackages to DHT

### Phase 3: Space Creation & Invitation

7. ✓ Alice creates space "automated-test"
8. ✓ Alice creates invite code
9. ✓ Bob connects to Alice
10. ✓ Bob joins space with invite code

### Phase 4: MLS Encryption Setup

11. ✓ Alice gets Bob's User ID
12. ✓ Alice adds Bob to MLS encryption group
13. ✓ Bob receives MLS Welcome message
14. ✓ Bob joins MLS group

### Phase 5: Messaging

15. ✓ Alice creates channel "general"
16. ✓ Alice creates thread "Test Thread"
17. ✓ Alice sends encrypted message

### Phase 6: Validation

18. ✓ Bob can list and see the space
19. ✓ Duplicate add correctly rejected

## Sample Output

```
============================================================
Spaceway MLS E2EE Automated Test Suite
============================================================

Cleaning old test data...
Building Spaceway...
Build complete!

Starting Alice (port 9001)...
Alice started (PID: 12345)
Starting Bob (port 9002)...
Bob started (PID: 12346)

============================================================
Running Test Sequence
============================================================

Step 1: Publishing KeyPackages
[Alice] > keypackage publish
[Bob] > keypackage publish

Step 2: Alice creates space
[Alice] > space create automated-test
  Space ID (short): f9b6e25e926e60e9
  Space ID (full): f9b6e25e926e60e9b56d2f25ccaf36d0328f015307a99cb7b56d2f25ccaf36d0

Step 3: Alice creates invite
[Alice] > invite create
  Invite Code: voUUmLss

Step 4: Bob connects to Alice
[Bob] > connect /ip4/127.0.0.1/tcp/9001/p2p/12D3KooW...

Step 5: Bob joins space
[Bob] > join f9b6e25e... voUUmLss

Step 6: Getting Bob's User ID
[Bob] > whoami
  Bob User ID: 328f015307a99cb7b56d2f25ccaf36d0b56d2f25ccaf36d0328f015307a99cb7

Step 7: Alice checks members
[Alice] > members

Step 8: Alice adds Bob to MLS group
[Alice] > member add 328f015307a99cb7...

Step 9: Alice creates channel and thread
[Alice] > channel create general
[Alice] > thread create "Test Thread"
[Alice] > send Hello Bob! This is encrypted!

Step 10: Bob lists spaces
[Bob] > space list

Step 11: Testing duplicate add (should fail)
[Alice] > member add 328f015307a99cb7...

============================================================
Test Results
============================================================

✓ Alice published KeyPackages
✓ Bob published KeyPackages
✓ Alice created space
✓ Alice created invite
✓ Bob connected to Alice
✓ Bob joined space
✓ Alice added Bob to MLS
✓ Bob received MLS Welcome message
✓ Bob joined MLS group
✓ Bob can see the space
✓ Alice created channel
✓ Alice created thread
✓ Alice sent message
✓ Duplicate add correctly rejected

============================================================
Summary
============================================================

Results: 14/14 tests passed

🎉 All tests passed!

Output Files:
  Alice log: alice_output.log
  Bob log: bob_output.log
  Test results: test_results.txt
```

## Analyzing Results

### View Real-Time Logs

While the test is running (or after):

```bash
# Watch Alice's output
tail -f alice_output.log

# Watch Bob's output
tail -f bob_output.log

# Search for specific patterns
grep "MLS" alice_output.log
grep "Welcome" bob_output.log
```

### Check Test Summary

```bash
cat test_results.txt
```

### Debug Failed Tests

If a test fails, check the relevant log:

```bash
# Find errors in Alice's log
grep -i "error\|failed" alice_output.log

# Find errors in Bob's log
grep -i "error\|failed" bob_output.log

# Check MLS-specific output
grep -A 5 "MLS" alice_output.log
grep -A 5 "Welcome" bob_output.log
```

## Customization

### Modify Test Parameters

**Python version (`test-automation.py`):**

```python
# Change ports
self.alice = SpacewayClient('Alice', 'alice.key', 9001, 'alice_output.log')
self.bob = SpacewayClient('Bob', 'bob.key', 9002, 'bob_output.log')

# Change space name
self.alice.send_command('space create my-custom-space', wait=3)

# Add more commands
self.alice.send_command('channel create announcements', wait=3)
self.bob.send_command('channel announcements', wait=2)
```

**Bash version (`test-automation.sh`):**

```bash
# Change configuration
ALICE_PORT=9001
BOB_PORT=9002
SPACE_NAME="automated-test"
```

### Add Custom Tests

**Python version:**

```python
# Add in the run() method after Step 11
print(f"\n{Color.CYAN}Step 12: Custom test{Color.NC}")
self.alice.send_command('my-custom-command', wait=2)

# Add test validation
self.test("My custom test",
         lambda: self.alice.check_log(r'expected pattern'))
```

## Troubleshooting

### Issue: Tests hang or timeout

**Solution**: Increase wait times in send_command() calls

```python
self.alice.send_command('keypackage publish', wait=10)  # Increased from 5
```

### Issue: Can't extract User ID or Space ID

**Solution**: Check the log file manually to see the exact format

```bash
cat alice_output.log | grep -A 2 "Created space"
cat bob_output.log | grep -A 2 "User ID"
```

### Issue: Build fails

**Solution**: Build manually first to see errors

```bash
cargo +nightly build --release
```

### Issue: Processes don't terminate

**Solution**: Kill manually

```bash
pkill -f "spaceway.*alice.key"
pkill -f "spaceway.*bob.key"
```

## Advanced Usage

### Run Multiple Test Iterations

```bash
for i in {1..5}; do
    echo "=== Test iteration $i ==="
    ./test-automation.py
    sleep 2
done
```

### Compare Results Across Runs

```bash
./test-automation.py
mv test_results.txt test_results_run1.txt
mv alice_output.log alice_output_run1.log

./test-automation.py
mv test_results.txt test_results_run2.txt
mv alice_output.log alice_output_run2.log

diff test_results_run1.txt test_results_run2.txt
```

### Extract Specific Metrics

```bash
# Count MLS operations
grep -c "MLS" alice_output.log

# Check DHT performance
grep "DHT" alice_output.log | grep -o "[0-9]\+ms"

# Verify message propagation
grep "Received MLS Welcome" bob_output.log
```

## Integration with CI/CD

The automation scripts can be integrated into CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Run Spaceway Tests
  run: |
    chmod +x test-automation.py
    ./test-automation.py

- name: Upload Test Results
  uses: actions/upload-artifact@v2
  with:
    name: test-results
    path: |
      alice_output.log
      bob_output.log
      test_results.txt
```

## Benefits

### Before Automation ❌

- Open 2 terminals manually
- Copy/paste multiaddrs between terminals
- Manually extract space IDs, invite codes, user IDs
- Manually verify each step
- Error-prone and time-consuming
- Hard to reproduce exact scenarios

### After Automation ✅

- One command: `./test-automation.py`
- Automatic coordination between clients
- All IDs extracted automatically
- Comprehensive validation
- Consistent and reproducible
- Detailed logs for analysis

## Next Steps

After running the automated tests:

1. **Analyze logs** for any warnings or errors
2. **Verify MLS group membership** is working correctly
3. **Test message encryption** once implemented
4. **Add more test scenarios** as needed
5. **Use for regression testing** during development

## Files Generated

| File               | Description                         |
| ------------------ | ----------------------------------- |
| `alice_output.log` | Complete terminal output from Alice |
| `bob_output.log`   | Complete terminal output from Bob   |
| `test_results.txt` | Test summary and pass/fail results  |
| `alice.key`        | Alice's account keypair (temporary) |
| `bob.key`          | Bob's account keypair (temporary)   |
| `alice-data/`      | Alice's RocksDB storage (temporary) |
| `bob-data/`        | Bob's RocksDB storage (temporary)   |

All temporary files are cleaned up at the start of each test run.

## Support

For issues or questions:

1. Check the log files for error messages
2. Review the test results summary
3. Try running with increased wait times
4. Test manually to verify expected behavior
5. Check the MLS implementation documentation
