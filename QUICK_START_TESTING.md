# Quick Start: Automated Testing

## TL;DR - Run This Now!

```bash
# Make sure you're in the spaceway directory
cd /path/to/spaceway

# Run the automated test
./test-automation.py
```

That's it! The script will:

1. Build Spaceway
2. Start Alice and Bob
3. Run the complete MLS E2EE workflow
4. Test 14 different scenarios
5. Generate detailed logs
6. Show you a pass/fail summary

## What You'll See

The script will output color-coded progress:

- 🔵 Blue = Test steps/phases
- 🟡 Yellow = Commands being executed
- 🟢 Green = Success
- 🔴 Red = Failure

## After It Runs

Check these files:

```bash
# See what Alice did
cat alice_output.log

# See what Bob did
cat bob_output.log

# See test summary
cat test_results.txt
```

## Expected Output (Success)

```
============================================================
Summary
============================================================

Results: 14/14 tests passed

🎉 All tests passed!
```

## If Tests Fail

1. **Look at the logs**:

   ```bash
   grep -i error alice_output.log bob_output.log
   ```

2. **Run again** (sometimes timing issues):

   ```bash
   ./test-automation.py
   ```

3. **Check specific test**:

   ```bash
   # Did KeyPackages publish?
   grep "Published 10 KeyPackages" alice_output.log bob_output.log

   # Did MLS Welcome arrive?
   grep "Received MLS Welcome" bob_output.log
   ```

## Common Issues

### "Permission denied"

```bash
chmod +x test-automation.py
```

### "command not found: python3"

Use the bash version:

```bash
./test-automation.sh
```

### "Build failed"

Build manually first:

```bash
cargo +nightly build --release
```

### Processes don't stop

Kill them manually:

```bash
pkill -f spaceway
```

## Next Steps

Once tests pass:

1. ✅ MLS infrastructure is working
2. ✅ KeyPackage publication works
3. ✅ Invite system works
4. ✅ Welcome messages arrive
5. ⚠️ Message encryption needs implementation

See `AUTOMATED_TESTING_README.md` for details on customizing tests.

## Real-World Usage

Instead of manually:

```bash
# Terminal 1 (Alice)
$ cargo run --release -- --account alice.key --port 9001
> keypackage publish
> space create test
> invite create
> # copy invite code
> members
> # copy bob's user ID
> member add <bob_user_id>

# Terminal 2 (Bob)
$ cargo run --release -- --account bob.key --port 9002
> keypackage publish
> connect /ip4/127.0.0.1/tcp/9001/p2p/...  # copy multiaddr
> join <space_id> <invite_code>  # paste both
> whoami  # copy user ID
```

Just run:

```bash
./test-automation.py
```

**Time saved**: 5-10 minutes per test run → 30 seconds! 🚀
