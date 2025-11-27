# RocksDB Library Issue - SOLVED! ✅

## The Problem

```
error while loading shared libraries: librocksdb.so.10: cannot open shared object file
```

This happens because the compiled binary can't find the RocksDB shared library at runtime.

## ✅ Solution: Use Nix (Now Fixed!)

The `flake.nix` has been updated to properly set `LD_LIBRARY_PATH` so all shared libraries are found.

### Simple Usage - Just Use These Commands

#### Run Alice:

```bash
./run-spaceway.sh --account ./alice.key --port 9001
```

#### Run Bob:

```bash
./run-spaceway.sh --account ./bob.key --port 9002
```

#### Run Charlie:

```bash
./run-spaceway.sh --account ./charlie.key --port 9003
```

The `run-spaceway.sh` wrapper automatically uses Nix with all dependencies!

### Alternative: Manual Nix Command

If you prefer the full command:

```bash
nix develop --command cargo +nightly run --bin spaceway -- --account ./alice.key --port 9001
```

### What Was Fixed

Updated `flake.nix` to export `LD_LIBRARY_PATH` with all required libraries:

- RocksDB 10.5.1
- Snappy compression
- Zlib, Bzip2, LZ4, Zstd

Now the runtime linker can find all shared libraries!

## 🎯 Quick Start for Multi-Peer Testing

### Terminal 1 - Alice

```bash
nix develop
./scripts/start-peer.sh --name alice --port 9001
```

### Terminal 2 - Bob

```bash
nix develop
./scripts/start-peer.sh --name bob --port 9002
```

### Terminal 3 - Charlie

```bash
nix develop
./scripts/start-peer.sh --name charlie --port 9003
```

## 🔍 What Nix Provides

When you run `nix develop`, you get:

- ✅ **RocksDB** 10.5.1 (system library)
- ✅ **OpenSSL** (for encryption)
- ✅ **SQLite** (for metadata)
- ✅ **LZ4, Zstd, Snappy** (compression)
- ✅ **Rust toolchain** (configured version)
- ✅ **Build tools** (pkg-config, cmake, etc.)

All libraries are properly linked and available at runtime!

## 📝 Environment Confirmation

When you enter `nix develop`, you'll see:

```
╔════════════════════════════════════════╗
║   Descord Development Environment     ║
╚════════════════════════════════════════╝

Rust version: 1.75.0
System: x86_64-linux

Available commands:
  cargo build          - Build the project
  cargo test           - Run tests
  cargo run --bin descord - Build and run CLI
  cargo watch -x test  - Auto-run tests on changes
```

## 🚀 Running Tests

```bash
# Enter Nix shell
nix develop

# Run unit tests
cargo +nightly test --lib

# Run storage tests
cargo +nightly test --package spaceway-core --lib storage::tests

# Run beta test (automated 3-peer test)
cargo +nightly test --package spaceway-core --test beta_test -- --ignored --nocapture
```

## 🎉 All Fixed!

Your development environment is now properly configured with Nix.
All dependencies are available and properly linked.

**Remember:** Always use `nix develop` or let the scripts handle it for you!
