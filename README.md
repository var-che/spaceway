# Descord

A privacy-preserving, scalable, decentralized communication platform.

## Project Structure

```
descord/
├── core/           # Core library (CRDT, crypto, storage, network)
├── relay/          # Relay node for development and production
├── desktop/        # Desktop app (future)
└── mobile/         # Mobile apps (future)
```

## Development Setup

### Prerequisites

- Rust 1.75+ (stable)
- RocksDB

### Building

```powershell
# Build all components
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Running the Relay Node

```powershell
# Development mode (localhost:9000)
cargo run -p descord-relay

# With debug logging
$env:RUST_LOG="debug"; cargo run -p descord-relay
```

## Architecture

See `backend/project_desc.md` for the complete architectural specification.

### Core Modules

- **CRDT**: Conflict-free replicated data types with HLC ordering
- **Crypto**: Ed25519 signing, Blake3 hashing, MLS integration
- **Storage**: RocksDB persistence for ops and blobs
- **Network**: libp2p-based P2P networking (WIP)

## Current Status

**MVP Phase 1** - Core Foundation:
- ✅ Project structure with separated modules
- ✅ CRDT operation types and HLC timestamps
- ✅ Ed25519 signing and verification
- ✅ RocksDB storage layer
- ✅ CBOR serialization with minicbor
- 🚧 Networking layer (in progress)
- 🚧 MLS integration (planned)
- 🚧 Relay implementation (planned)

## License

MIT OR Apache-2.0
