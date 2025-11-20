# Descord Relay Server

A dedicated Circuit Relay v2 server for IP privacy and NAT traversal in the Descord P2P network.

## Features

- **Circuit Relay v2**: Full support for libp2p Circuit Relay protocol
- **NAT Traversal**: Enables peers behind NAT/firewalls to communicate
- **IP Privacy**: Peers connect via relay without exposing their IP addresses
- **Multi-Transport**: Supports both TCP and QUIC transports
- **Resource Limits**: Configurable bandwidth and circuit limits
- **Statistics**: Real-time monitoring of relay usage

## Quick Start

### Build

```bash
cargo build --package descord-relay --release
```

### Run

```bash
# Run on default port (9000)
./target/release/descord-relay

# Or via cargo
cargo run --package descord-relay --release
```

## Configuration

The relay server currently uses sensible defaults:

- **Listen Address**: `0.0.0.0:9000` (all interfaces)
- **Max Reservations**: 1024 total, 3 per peer
- **Max Circuits**: 512 total, 5 per peer
- **Circuit Duration**: 1 hour max
- **Circuit Bandwidth**: 100 MB max per circuit

## Architecture

### Circuit Relay v2

The relay implements libp2p's Circuit Relay v2 protocol, which provides:

1. **Reservation**: Peers reserve a slot on the relay
2. **Circuit Establishment**: Source peer requests circuit to destination peer
3. **Data Relay**: Relay forwards encrypted data between peers
4. **Resource Management**: Automatic cleanup and limit enforcement

### Privacy Model

```
User A <--encrypted--> Relay Server <--encrypted--> User B
         (A's IP hidden)              (B's IP hidden)
```

- Peers never see each other's IP addresses
- Relay cannot decrypt MLS-encrypted message payloads
- Multiple relays can be used simultaneously for redundancy

## Statistics

The relay server prints statistics every 60 seconds:

- **Uptime**: How long the relay has been running
- **Active Circuits**: Current number of relay circuits
- **Connected Peers**: Number of peers with reservations
- **Total Bandwidth**: Cumulative data relayed (MB)

## Events

The relay server logs detailed events for monitoring:

- ✅ **Reservation Accepted**: Peer successfully reserved a slot
- 🔄 **Reservation Renewed**: Existing reservation extended
- ⏰ **Reservation Timeout**: Reservation expired
- 🔀 **Circuit Accepted**: New circuit established between peers
- 🔌 **Circuit Closed**: Circuit terminated (normal or error)
- ⛔ **Request Denied**: Reservation or circuit denied (overload)

## Testing

### Start Relay Server

```bash
cargo run --package descord-relay --release
```

### Run Integration Test

```bash
cargo test --test network_integration_test test_relay_connection -- --ignored --nocapture
```

The test verifies that clients can:
- Connect to the relay server
- Establish reservations
- Use the relay for peer-to-peer connectivity

## Deployment

### VPS Deployment

The relay server is designed for deployment on a VPS or cloud server:

1. **Firewall**: Open TCP port 9000
2. **Systemd Service**: Create a systemd unit for automatic startup
3. **Monitoring**: Use logs for health checking
4. **Updates**: Rolling updates with zero downtime (multiple relays)

### Example Systemd Unit

```ini
[Unit]
Description=Descord Relay Server
After=network.target

[Service]
Type=simple
User=descord
ExecStart=/opt/descord/descord-relay
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## Next Steps

Future enhancements planned:

1. **CLI Arguments**: Port configuration, resource limits
2. **DHT Advertisement**: Automatic relay discovery
3. **Bandwidth Tracking**: Per-peer usage monitoring
4. **Metrics Endpoint**: Prometheus/JSON API for monitoring
5. **QUIC Support**: Add QUIC transport alongside TCP
6. **Cooperative Mode**: Allow users to opt-in as relay

## Contributing

The relay server is a critical piece of Descord's privacy architecture. When contributing:

- Maintain backward compatibility with Circuit Relay v2
- Keep resource limits configurable
- Log all important events for debugging
- Test with multiple concurrent clients

## License

Same as Descord project (check root LICENSE file).
