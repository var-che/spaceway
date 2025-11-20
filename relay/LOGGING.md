# Relay Server Logging Guide

## What You Should See

When the relay server is running correctly, you'll see these log messages:

### Startup Logs

```
INFO descord_relay: 🚀 Starting Descord Relay Server
INFO descord_relay: Relay server peer ID: 12D3KooW...
INFO descord_relay: Listening on: /ip4/0.0.0.0/tcp/9000
INFO descord_relay: Relay server running...
INFO descord_relay: 🎧 Listening on: /ip4/192.168.100.131/tcp/9000
INFO descord_relay: 🎧 Listening on: /ip4/127.0.0.1/tcp/9000
```

### Periodic Statistics (every 60 seconds)

```
INFO descord_relay: 📊 Relay Statistics:
INFO descord_relay:   Uptime: 60.123s
INFO descord_relay:   Connected peers: 2
INFO descord_relay:   Peers with circuits: 0
INFO descord_relay:   Active circuits: 0
INFO descord_relay:   Total bandwidth: 0 MB
```

### When a Client Connects

```
INFO descord_relay: 📥 Incoming connection from: /ip4/127.0.0.1/tcp/12345
INFO descord_relay: 🔗 Connection established with peer: 12D3KooWABC...
INFO descord_relay:    Remote address: /ip4/127.0.0.1/tcp/12345
INFO descord_relay:    Total connections to this peer: 1
INFO descord_relay:    Total unique peers connected: 1
```

### When a Client Disconnects

```
INFO descord_relay: ❌ Last connection closed with peer: 12D3KooWABC... (cause: Some(...))
INFO descord_relay:    Total unique peers connected: 0
```

### When a Peer Requests Relay Reservation

```
INFO descord_relay: ✅ Reservation ACCEPTED for peer: 12D3KooWABC...
INFO descord_relay:    This peer can now be used as a relay hop
```

### When a Relay Circuit is Established

```
INFO descord_relay: 🔀 Circuit ESTABLISHED: 12D3KooWABC... -> 12D3KooWXYZ...
INFO descord_relay:    Source peer can now communicate with destination via this relay
INFO descord_relay:    Total circuits: 1
```

### When a Circuit Closes

```
INFO descord_relay: 🔌 Circuit closed: 12D3KooWABC... -> 12D3KooWXYZ...
```

## Current Status

### What's Working ✅

- **TCP connections**: Clients can connect to the relay server
- **Connection tracking**: The relay logs incoming connections and peer IDs
- **Statistics**: Real-time metrics every 60 seconds

### What's Missing ⚠️

The current test only establishes TCP connections but doesn't trigger the relay protocol because:

1. **No Reservation Requests**: Clients need to explicitly request a reservation
2. **No Circuit Requests**: Clients need to request circuits through the relay
3. **Missing Peer ID in Address**: Clients dial `/ip4/127.0.0.1/tcp/9000` but should dial `/ip4/127.0.0.1/tcp/9000/p2p/12D3KooW...` (relay's peer ID)

## Why You're Not Seeing Relay Events

When you ran the test, the logs showed:

```
📥 Incoming connection from: /ip4/127.0.0.1/tcp/59270
📥 Incoming connection from: /ip4/127.0.0.1/tcp/59271
```

This means:
- ✅ Alice and Bob **connected** to the relay (TCP handshake successful)
- ✅ The relay **accepted** the connections
- ❌ **No reservation requests** were made (clients didn't ask to be relayed)
- ❌ **No circuits** were established (no relay forwarding happened)

## Expected Behavior

The relay server logs show that **basic connectivity works**, but the clients aren't actually **using** the relay protocol yet.

To see full relay activity, clients would need to:

1. **Dial with Peer ID**:
   ```rust
   let relay_peer_id = PeerId::from_str("12D3KooW...")?;
   let relay_addr = "/ip4/127.0.0.1/tcp/9000"
       .parse::<Multiaddr>()?
       .with(Protocol::P2p(relay_peer_id));
   client.dial(relay_addr).await?;
   ```

2. **Request Reservation**:
   ```rust
   // This happens automatically when using Circuit Relay v2 client behavior
   // The client's relay_client behavior sends reservation requests
   ```

3. **Establish Circuit**:
   ```rust
   // Alice dials Bob through the relay
   client.dial_via_relay(
       relay_addr,
       relay_peer_id,
       bob_peer_id
   ).await?;
   ```

## Next Steps

To see full relay protocol in action:

1. **Update Client Test**: Make Alice dial Bob **through** the relay (not just connect to relay)
2. **Add Relay Discovery**: Clients need to know the relay's peer ID
3. **Enable Relay Client Behavior**: Ensure clients have `relay::client::Behaviour` configured

The relay server itself is **fully functional** - it's just waiting for clients to actually request relay services!
