# Connect Peers Feature

## Overview

The **Connect Peers** action establishes P2P network connections between the three isolated clients (Alice, Bob, and Charlie) in the dashboard.

## Problem

By default, all three clients are isolated with no bootstrap peers configured:

- Alice listens on a random TCP port (e.g., `/ip4/127.0.0.1/tcp/33099`)
- Bob listens on another port (e.g., `/ip4/127.0.0.1/tcp/41977`)
- Charlie on yet another (e.g., `/ip4/127.0.0.1/tcp/41115`)

Without connections, DHT and GossipSub operations fail:

```
✗ DHT GET failed: NotFound
✗ Publish failed: NoPeersSubscribedToTopic
```

## Solution

The `ConnectPeers` action:

1. **Fetches peer info** for all three clients:
   - Listening addresses (`listening_addrs()`)
   - Peer IDs (`peer_id()`)
2. **Establishes connections**:
   - Bob → Alice
   - Charlie → Alice
   - Charlie → Bob
3. **Waits for connections** to establish (1 second delay)

## Usage

### Frontend

1. Go to the **Dashboard** tab
2. Select any client (client selection doesn't matter for this action)
3. Choose action: **🔗 Connect Peers (P2P Network)**
4. Click **Execute Action**

### Expected Result

```
✓ Peer connections initiated. Alice, Bob, and Charlie are now connected!
```

### Terminal Output (Backend)

```
INFO dashboard_backend: 🔗 Connecting all peers together...
INFO dashboard_backend: Alice: 12D3KooW... at ["/ip4/127.0.0.1/tcp/33099"]
INFO dashboard_backend: Bob: 12D3KooW... at ["/ip4/127.0.0.1/tcp/41977"]
INFO dashboard_backend: Charlie: 12D3KooW... at ["/ip4/127.0.0.1/tcp/41115"]
INFO dashboard_backend: Connecting Bob → Alice: /ip4/127.0.0.1/tcp/33099/p2p/12D3KooW...
INFO dashboard_backend: ✓ Bob connected to Alice
INFO dashboard_backend: Connecting Charlie → Alice: /ip4/127.0.0.1/tcp/33099/p2p/12D3KooW...
INFO dashboard_backend: ✓ Charlie connected to Alice
INFO dashboard_backend: Connecting Charlie → Bob: /ip4/127.0.0.1/tcp/41977/p2p/12D3KooW...
INFO dashboard_backend: ✓ Charlie connected to Bob
```

## Impact on Other Actions

Once peers are connected:

### ✅ DHT Operations Work

- `CreateSpace` can store space metadata in DHT
- `JoinSpace` can retrieve space data from DHT
- No more "NotFound" errors

### ✅ GossipSub Works

- Space operations are broadcast to all members
- No more "NoPeersSubscribedToTopic" errors
- Real-time sync between clients

### ✅ Join Workflow Succeeds

Instead of:

```
⚠️ Found space 'red' but P2P network not connected
```

Bob can now successfully join Alice's space:

```
✓ Joined space 'red'
```

## Complete Workflow Example

### Step 1: Connect Peers

```
Client: Any
Action: ConnectPeers
Result: ✓ Peers connected
```

### Step 2: Alice Creates Space

```
Client: Alice
Action: CreateSpace
Name: "red"
Result: Created space 'red' with ID: 505c2b95...
```

### Step 3: Alice Creates Invite

```
Client: Alice
Action: CreateInvite
Space ID: 505c2b9529156de1286241b879685afd5aa5fccc627833ec975ed5af024403d2
Result: Created invite! Code: ph18Csmh
```

### Step 4: Bob Joins Space

```
Client: Bob
Action: JoinSpace
Space ID: 505c2b9529156de1286241b879685afd5aa5fccc627833ec975ed5af024403d2
Result: ✓ Joined space 'red'
```

### Step 5: Verify Membership

Both Alice and Bob now show:

```
Spaces: 1
  - Space "red" (ID: 505c2b95...)
    Members: 2
```

## Technical Details

### Backend Implementation

File: `dashboard-backend/src/main.rs`

```rust
Action::ConnectPeers => {
    // Get peer addresses
    let alice_addrs = state.alice.read().await.listening_addrs().await;
    let alice_peer_id = state.alice.read().await.peer_id().await;

    // Connect Bob to Alice
    let full_addr = format!("{}/p2p/{}", alice_addr, alice_peer_id);
    state.bob.read().await.network_dial(&full_addr).await?;

    // ... similar for other connections
}
```

### Frontend Update

File: `dashboard-frontend/src/components/ActionPanel.tsx`

Added `ConnectPeers` to action dropdown (no additional input fields needed).

## Network Graph

After connecting peers, the network graph should show edges between nodes based on shared GossipSub topics.

## Troubleshooting

### Connections Fail

If you see `✗ Bob → Alice failed`, check:

- Are all three clients running? (Check terminal output)
- Are listening addresses valid? (Should show `/ip4/127.0.0.1/tcp/XXXXX`)

### Still Getting DHT Errors

- Make sure you ran `ConnectPeers` action BEFORE creating/joining spaces
- Wait a few seconds after connecting for DHT to stabilize
- Try connecting again

### Join Still Fails

- Verify peers are connected (check backend logs)
- Make sure Space ID is the full 64-character hex string
- Try copying Space ID using the "Copy ID" button

## Notes

- **Dashboard-specific**: This is a demo feature for the dashboard. In production, clients would discover each other via bootstrap nodes and DHT.
- **Connection order**: Connections are unidirectional in code but libp2p makes them bidirectional.
- **Persistence**: Connections are in-memory only. Restarting the backend requires reconnecting.
- **Relay servers**: Not needed for local testing since all clients are on 127.0.0.1.

## Related Documentation

- `NETWORK_CONNECTIVITY_ISSUE.md` - Explains why this is needed
- `TUTORIAL_FEATURE.md` - Full tutorial including this step
- `FIX_JSON_PARSE_ERROR.md` - JoinSpace field fix
