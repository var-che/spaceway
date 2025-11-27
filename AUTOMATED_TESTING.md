# Automated Testing - No Manual Terminal Work!

## 🎉 Good News!

You **don't need to manually open 3 terminals** anymore. The project already has automated tests!

---

## ✅ **Option 1: Run Existing Beta Test** (Easiest)

This simulates Alice, Bob, and Charlie automatically:

```bash
nix develop --command cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

**What it does:**

- ✅ Starts 3 users programmatically
- ✅ Connects them via relay
- ✅ Creates space, channels, threads
- ✅ Exchanges messages
- ✅ Verifies everything works

**Note:** Requires relay server running first:

```bash
# Terminal 1: Start relay
cargo +nightly run --package descord-relay --release

# Terminal 2: Run test
nix develop --command cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

---

## ✅ **Option 2: Other Automated Tests**

The project has many integration tests you can run:

```bash
# List all available tests
cargo +nightly test --package spaceway-core --tests -- --list

# Some useful ones:
cargo +nightly test --package spaceway-core --test two_user_gossip_test -- --nocapture
cargo +nightly test --package spaceway-core --test crdt_replication_test -- --nocapture
cargo +nightly test --package spaceway-core --test storage_integration_test -- --nocapture
```

---

## ✅ **Option 3: Simple Wrapper Script**

I've created a script that shows you exactly what to run:

```bash
./start-3-peers-guide.sh
```

This displays step-by-step instructions for manual testing if you prefer to see the CLI in action.

---

## 🚀 **Quick Automated Test (Recommended)**

Just run this one command:

```bash
# Start relay in background
cargo +nightly run --package descord-relay --release &

# Wait 2 seconds
sleep 2

# Run test
nix develop --command cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

That's it! No manual terminal juggling, no copying addresses, all automated!

---

## 📊 **What Gets Tested**

All tests verify:

- ✅ Peer discovery and connection
- ✅ Space creation and joining
- ✅ End-to-end encryption (MLS)
- ✅ CRDT synchronization
- ✅ Message propagation
- ✅ Privacy guarantees

---

## 🎯 **Summary**

**For automated testing:** Use the existing `beta_test`  
**For manual exploration:** Use the 3-terminal guide (`start-3-peers-guide.sh`)

Both work great - choose what you prefer!
