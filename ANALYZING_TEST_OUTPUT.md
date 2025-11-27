# Analyzing Automated Test Output

This guide helps you understand and debug the output from automated tests.

## Quick Analysis Commands

### Check Overall Success

```bash
# See if all tests passed
tail -20 test_results.txt

# Count passed vs failed
grep "^✓" test_results.txt | wc -l  # Passed
grep "^✗" test_results.txt | wc -l  # Failed
```

### Find Specific Events

**KeyPackage Operations:**

```bash
# Did KeyPackages get published?
grep "Published 10 KeyPackages" alice_output.log bob_output.log

# Did KeyPackages get fetched?
grep "Fetched KeyPackage" alice_output.log

# Check KeyPackage hashes
grep "Generated KeyPackage with hash" bob_output.log
```

**MLS Welcome Messages:**

```bash
# Did Bob receive the Welcome?
grep "Received MLS Welcome" bob_output.log

# Did Bob join the MLS group?
grep "Joined MLS group" bob_output.log

# Check Welcome message details
grep -A 5 "Received MLS Welcome" bob_output.log
```

**Space and Invite Operations:**

```bash
# What space was created?
grep "Created space:" alice_output.log

# What invite code was generated?
grep "Created invite code:" alice_output.log

# Did Bob join successfully?
grep "Successfully joined Space" bob_output.log
```

**DHT Operations:**

```bash
# DHT storage success
grep "Stored.*in DHT" alice_output.log

# DHT retrieval success
grep "Retrieved.*from DHT" bob_output.log

# DHT errors
grep "DHT.*failed\|DHT.*error" alice_output.log bob_output.log
```

**Network Events:**

```bash
# Connection established
grep "ConnectionEstablished" alice_output.log bob_output.log

# Peer discovery
grep "Identified.*peer" alice_output.log bob_output.log

# GossipSub subscriptions
grep "Subscribed to.*topic" alice_output.log bob_output.log
```

## Common Patterns to Look For

### Success Patterns

```bash
# Complete MLS flow (should see all of these)
grep -E "Published.*KeyPackages|Fetched KeyPackage|added to MLS group|Received MLS Welcome|Joined MLS group" alice_output.log bob_output.log
```

Expected output:

```
alice_output.log:✓ Published 10 KeyPackages to DHT
bob_output.log:✓ Published 10 KeyPackages to DHT
alice_output.log:✓ Fetched KeyPackage for user ... from DHT
alice_output.log:✓ User ... added to MLS group!
bob_output.log:  🎉 Received MLS Welcome message
bob_output.log:  ✓ Joined MLS group for space space_...
```

### Failure Patterns

```bash
# Look for errors
grep -i "error\|failed\|✗" alice_output.log bob_output.log

# Look for warnings
grep -i "warn\|⚠" alice_output.log bob_output.log

# Look for timeouts
grep -i "timeout\|timed out" alice_output.log bob_output.log
```

## Debug Specific Issues

### Issue: "Bob didn't receive Welcome message"

**Check:**

```bash
# Did Alice actually send it?
grep "Sent Welcome message" alice_output.log

# Is Bob subscribed to the right topic?
grep "Subscribed to Welcome message topic" bob_output.log

# Check Bob's user ID matches
grep "User ID:" bob_output.log
grep "Adding.*to MLS" alice_output.log
```

**Expected:**

```
bob_output.log:✓ Subscribed to Welcome message topic: user/328f015307a99cb7/welcome
alice_output.log:ℹ Adding 328f0153 to MLS encryption group...
alice_output.log:  ✓ Sent Welcome message to UserId(...) on user/328f015307a99cb7/welcome
bob_output.log:  🎉 Received MLS Welcome message
```

### Issue: "DuplicateSignatureKey error"

**Check:**

```bash
# How many times was the user added?
grep "member add" alice_output.log

# Check for already-in-group message
grep "already in the MLS encryption group" alice_output.log
```

**Expected (on second add):**

```
✗ User is already in the MLS encryption group!

  This user has already been added to the MLS group for this space.
```

### Issue: "DHT operations failing"

**Check:**

```bash
# DHT peer count
grep "peers in DHT routing table" alice_output.log

# DHT quorum
grep -i "quorum" alice_output.log bob_output.log

# Network connectivity
grep "ConnectionEstablished" alice_output.log bob_output.log
```

**Expected:**

```
alice_output.log:ConnectionEstablished { peer_id: PeerId("12D3...") }
bob_output.log:ConnectionEstablished { peer_id: PeerId("12D3...") }
```

### Issue: "KeyPackages not found"

**Check:**

```bash
# Did Bob publish?
grep "Published 10 KeyPackages" bob_output.log

# Did Alice try to fetch?
grep "Fetching KeyPackage.*from DHT" alice_output.log

# Check for NotFound error
grep "No KeyPackages found" alice_output.log
```

**Solution:**

- Ensure Bob publishes BEFORE Alice tries to add him
- Check timing in the test script

## Performance Analysis

### Measure Operation Times

```bash
# Extract timestamps for key operations
grep -E "Created space:|Successfully joined Space:|added to MLS group:" alice_output.log | \
  awk '{print $1, $2, $NF}'
```

### Count Network Events

```bash
# GossipSub messages
grep -c "Step.*Broadcast" alice_output.log

# DHT operations
grep -c "DHT.*Step" alice_output.log

# MLS operations
grep -c "MLS" alice_output.log
```

### Check Resource Usage

```bash
# While test is running (in another terminal):
ps aux | grep spaceway
top -p $(pgrep -d',' -f spaceway)
```

## Advanced Debugging

### Trace Specific Flow

**MLS Group Formation:**

```bash
# Extract complete MLS flow
grep -E "KeyPackage|MLS|Welcome|Commit|epoch" alice_output.log bob_output.log | \
  sed 's/^/[timestamp] /' | \
  sort
```

**Space Creation to Join:**

```bash
# Extract space lifecycle
grep -E "Created space:|invite code:|join.*Space|Joined.*Space" alice_output.log bob_output.log | \
  sort
```

### Compare Alice and Bob Timelines

```bash
# Create timeline files
grep "^2025-" alice_output.log > alice_timeline.txt
grep "^2025-" bob_output.log > bob_timeline.txt

# Compare side by side
sdiff alice_timeline.txt bob_timeline.txt
```

### Extract Error Context

```bash
# Get 5 lines before and after each error
grep -B 5 -A 5 -i "error\|failed" alice_output.log

# Get complete debug sections
awk '/Step A:/{flag=1} flag; /Step G:/{flag=0}' alice_output.log
```

## Generating Reports

### Create HTML Report

```bash
cat > test_report.html <<EOF
<!DOCTYPE html>
<html>
<head><title>Spaceway Test Results</title></head>
<body>
<h1>Test Results</h1>
<pre>$(cat test_results.txt)</pre>
<h2>Alice Log</h2>
<pre>$(cat alice_output.log | tail -100)</pre>
<h2>Bob Log</h2>
<pre>$(cat bob_output.log | tail -100)</pre>
</body>
</html>
EOF

# Open in browser
xdg-open test_report.html
```

### Create Summary CSV

```bash
# Extract key metrics
echo "metric,value" > metrics.csv
echo "tests_total,$(grep -c "^✓\|^✗" test_results.txt)" >> metrics.csv
echo "tests_passed,$(grep -c "^✓" test_results.txt)" >> metrics.csv
echo "keypackages_published,$(grep -c "Published 10 KeyPackages" alice_output.log bob_output.log)" >> metrics.csv
echo "mls_welcomes,$(grep -c "Received MLS Welcome" bob_output.log)" >> metrics.csv
echo "spaces_created,$(grep -c "Created space:" alice_output.log)" >> metrics.csv
```

## Integration with Development

### Watch Mode

```bash
# Run tests whenever code changes
while inotifywait -e modify core/src/**/*.rs cli/src/**/*.rs; do
    ./test-automation.py
done
```

### Regression Testing

```bash
# Save baseline
./test-automation.py
cp test_results.txt baseline_results.txt

# After code changes
./test-automation.py
diff baseline_results.txt test_results.txt
```

### Continuous Monitoring

```bash
# Run tests every hour
while true; do
    ./test-automation.py
    timestamp=$(date +%Y%m%d_%H%M%S)
    mv test_results.txt "results_${timestamp}.txt"
    sleep 3600
done
```

## Tips

1. **Always check both logs** - Alice's and Bob's perspectives are different
2. **Look for the last successful step** before a failure
3. **Timestamps help** - see if operations are taking too long
4. **Grep is your friend** - use patterns to filter relevant info
5. **Compare successful runs** - to identify what changed

## Quick Checklist

When a test fails, check in order:

- [ ] Did build succeed?
- [ ] Did both clients start?
- [ ] Did KeyPackages publish?
- [ ] Did peers connect?
- [ ] Did space get created?
- [ ] Did invite get created?
- [ ] Did Bob join the space?
- [ ] Did Alice add Bob to MLS?
- [ ] Did Bob receive Welcome?
- [ ] Check timing (too fast/too slow?)
- [ ] Check network events
- [ ] Check error messages

## Example: Complete Analysis Session

```bash
# 1. Check if test passed
tail -5 test_results.txt

# 2. If failed, find which test
grep "^✗" test_results.txt

# 3. Find related log entries
# Example: "Bob didn't receive Welcome"
grep -i "welcome" bob_output.log

# 4. Check Alice's side
grep -i "welcome" alice_output.log

# 5. Check network connectivity
grep "ConnectionEstablished" alice_output.log bob_output.log

# 6. Check timing
grep "Step 8:" alice_output.log bob_output.log

# 7. Get full context
grep -B 10 -A 10 "Sent Welcome message" alice_output.log
```

This should help you find the issue!
