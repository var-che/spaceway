# Beta Testing - Quick Reference

## 🚀 Fastest Way to Beta Test

### Windows
```powershell
# Run interactive menu
.\beta-test.ps1
```

### Linux/macOS
```bash
chmod +x beta-test.sh
./beta-test.sh
```

---

## 📋 One-Command Beta Test

### Step 1: Start Relay
```bash
# Terminal 1
cargo run --package descord-relay --release
```

### Step 2: Run Test
```bash
# Terminal 2
cargo test --package descord-core --test beta_test -- --ignored --nocapture
```

**That's it!** The test will automatically:
- Initialize 3 users (Alice, Bob, Charlie)
- Connect them via relay
- Create space, channels, threads
- Post encrypted messages
- Test relay rotation
- Verify privacy guarantees

**Duration:** ~60 seconds

---

## ✅ What Gets Tested

### Automated Beta Test Coverage:

1. **User Initialization** ✅
   - 3 users with unique keypairs
   - Network node startup
   - Storage initialization

2. **Relay Connection** ✅
   - DHT relay discovery
   - Circuit relay connection
   - Relay address generation

3. **Space Creation** ✅
   - Space creation with metadata
   - Channel creation (general, announcements)
   - MLS group setup

4. **DHT Peer Discovery** ✅
   - Advertisement on DHT
   - Peer discovery queries
   - Relay address retrieval

5. **P2P Connection** ✅
   - Relay-mediated peer dialing
   - Connection establishment
   - No direct IP exposure

6. **Messaging** ✅
   - Thread creation with first message
   - Additional message posting
   - E2EE encryption (MLS)

7. **Relay Rotation** ✅
   - Start rotation (30s interval for demo)
   - Wait for rotation trigger
   - Verify relay switch

8. **Privacy Verification** ✅
   - Check no direct listen addresses
   - Verify relay circuit addresses only
   - Confirm IP privacy from peers

---

## 📊 Expected Output

### Successful Test Output:
```
╔═══════════════════════════════════════════════════════════════════╗
║                   DESCORD AUTOMATED BETA TEST                     ║
║                                                                   ║
║  Simulates 3 users creating spaces, channels, and messaging      ║
║  All via privacy-preserving relay architecture                   ║
╚═══════════════════════════════════════════════════════════════════╝

╔═══════════════════════════════════════════════════════════════════╗
║ PHASE 1: User Initialization                                     ║
╚═══════════════════════════════════════════════════════════════════╝

👤 Initializing Alice...
   PeerID: 12D3KooW...
✅ All users initialized

... (8 phases total)

╔═══════════════════════════════════════════════════════════════════╗
║                    BETA TEST COMPLETE ✅                          ║
╚═══════════════════════════════════════════════════════════════════╝

✨ All systems operational!

test automated_beta_test ... ok
```

---

## ❌ Common Issues & Solutions

### Issue: "No relay available"
**Solution:**
```bash
# Make sure relay is running:
cargo run --package descord-relay --release
```

### Issue: "DHT discovery returns empty"
**Solution:**
- Wait 30-60 seconds for DHT propagation
- Test uses automatic retry with fallback

### Issue: "Connection timeout"
**Solution:**
- Check firewall (ports 9000, 8080)
- Verify relay is listening

### Issue: "Test takes too long"
**Solution:**
- Normal! DHT propagation can be slow
- Test includes 5-second waits for realism

---

## 🔍 Manual Verification

### Check Relay Stats
```bash
curl http://localhost:8080/stats
```

Expected:
```json
{
  "active_connections": 3,
  "total_bytes_sent": 1048576,
  "total_bytes_received": 524288,
  "uptime_seconds": 120,
  "relay_reputation": 0.95
}
```

### Verify Privacy
Each client should have:
- ❌ No direct IP listen addresses
- ✅ Only `/p2p-circuit` addresses
- ✅ Messages encrypted (ciphertext in logs)

---

## 📈 Test Results Interpretation

### PASS Criteria:
- ✅ All 3 users initialize
- ✅ Relay connections establish
- ✅ Space/channels created
- ✅ DHT discovery works (or fallback succeeds)
- ✅ Messages posted without errors
- ✅ Privacy checks pass

### FAIL Indicators:
- ❌ Panic/crash
- ❌ Connection refused errors
- ❌ Direct IP addresses exposed
- ❌ Encryption failures

---

## 🎯 Next Steps After Successful Test

1. **Read Full Guide:** See `BETA_TESTING.md`
2. **Manual Testing:** Try multi-user scenarios
3. **Deploy Relay:** Set up public relay server
4. **Report Issues:** GitHub issues or security@descord.org
5. **Provide Feedback:** What worked? What didn't?

---

## 💡 Pro Tips

- **Use `--nocapture`** to see detailed logs
- **Run in release mode** for realistic performance
- **Monitor relay logs** in separate terminal
- **Test with real latency** using VPS relays
- **Try different relay rotation intervals**

---

## 🛠️ Development Workflow

```bash
# 1. Make code changes
vim core/src/client.rs

# 2. Run quick test
cargo test --package descord-core --lib

# 3. Run full beta test
cargo test --package descord-core --test beta_test -- --ignored --nocapture

# 4. Check privacy guarantees
cargo test --package descord-core --test full_relay_integration_test test_relay_privacy_guarantees

# 5. Deploy and test on VPS
ssh vps "cd descord && git pull && cargo build --release"
```

---

## 📞 Support

**Need Help?**
- GitHub Issues: https://github.com/your-org/descord/issues
- Email: beta@descord.org
- Docs: `BETA_TESTING.md` (comprehensive guide)

**Found a Bug?**
- Security issues: security@descord.org (private)
- Feature requests: GitHub issues (public)
- Feedback: beta@descord.org

---

**Happy Testing! 🚀**
