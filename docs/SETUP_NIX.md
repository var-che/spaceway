# Setting Up Descord with Nix

This guide helps you set up a reproducible development environment for Descord using Nix.

## Prerequisites

### Install Nix

**On Linux (including Linux Mint):**
```bash
sh <(curl -L https://nixos.org/nix/install) --daemon
```

**On Windows (WSL2):**
```bash
# First install WSL2 with Ubuntu or another Linux distribution
# Then run the same command as Linux above
```

**On macOS:**
```bash
sh <(curl -L https://nixos.org/nix/install)
```

After installation, restart your terminal or run:
```bash
. /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
```

### Enable Flakes (Required)

Edit or create `~/.config/nix/nix.conf`:
```
experimental-features = nix-command flakes
```

Or for multi-user installations, edit `/etc/nix/nix.conf`:
```
experimental-features = nix-command flakes
```

Restart the Nix daemon:
```bash
sudo systemctl restart nix-daemon
```

## Quick Start

### 1. Clone the repository

```bash
git clone <your-repo-url> descord
cd descord
```

### 2. Enter development environment

```bash
nix develop
```

This will:
- Download and install the exact Rust toolchain (1.75.0)
- Install all system dependencies (OpenSSL, RocksDB, etc.)
- Set up environment variables
- Give you a shell ready for development

### 3. Build the project

```bash
cargo build
```

### 4. Run tests

```bash
cargo test --features test-utils
```

### 5. Build the CLI

```bash
cargo build --release --bin descord
./target/release/descord --help
```

## Using direnv (Optional but Recommended)

direnv automatically loads the Nix environment when you `cd` into the project.

### Install direnv

**On Linux:**
```bash
# Ubuntu/Debian
sudo apt install direnv

# Arch Linux
sudo pacman -S direnv
```

**On macOS:**
```bash
brew install direnv
```

### Set up direnv hook

Add to your shell config (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
eval "$(direnv hook bash)"  # For bash
eval "$(direnv hook zsh)"   # For zsh
```

### Allow direnv in the project

```bash
cd descord
direnv allow
```

Now the environment loads automatically when you enter the directory!

## Building the Application

### Development build
```bash
nix develop
cargo build
```

### Release build
```bash
nix develop
cargo build --release
```

### Build using Nix directly
```bash
nix build
```

The binary will be in `./result/bin/descord`

## Running Tests

### Run all tests
```bash
nix develop
cargo test --features test-utils
```

### Run specific test
```bash
nix develop
cargo test --features test-utils test_smooth_client_batch_creation
```

### Run with Nix app
```bash
nix run .#test
```

## Transferring to Another Machine

### Package everything (including Nix setup)

Create a tarball:
```bash
tar czf descord-project.tar.gz \
  --exclude=target \
  --exclude=.git \
  --exclude='*/target' \
  .
```

Transfer to Linux machine:
```bash
scp descord-project.tar.gz user@linux-machine:~/
```

On Linux machine:
```bash
tar xzf descord-project.tar.gz
cd descord
nix develop  # Exact same environment!
```

### Using Git (recommended)

On Windows:
```bash
git add flake.nix flake.lock .envrc
git commit -m "Add Nix flake for reproducible builds"
git push
```

On Linux:
```bash
git clone <your-repo>
cd descord
nix develop
cargo build  # Works identically!
```

## Troubleshooting

### "experimental-features" error
Enable flakes in nix.conf (see Prerequisites above)

### Permission denied errors
Ensure Nix daemon is running:
```bash
sudo systemctl status nix-daemon
```

### RocksDB linking errors
The flake sets `ROCKSDB_LIB_DIR` automatically. If issues persist:
```bash
nix develop
echo $ROCKSDB_LIB_DIR  # Should show path
```

### Different Rust version
The flake pins Rust 1.75.0. To change, edit `flake.nix`:
```nix
rustVersion = "1.75.0";  # Change this
```

Then:
```bash
nix flake update
```

## Benefits of Nix

✅ **Reproducible** - Same environment on Windows, Linux, macOS
✅ **Declarative** - Dependencies declared in `flake.nix`
✅ **Isolated** - Doesn't conflict with system packages
✅ **Cacheable** - Binary cache speeds up installation
✅ **Versioned** - Lock file ensures exact versions

## Next Steps

1. Install Nix on both machines
2. Transfer project using Git or tarball
3. Run `nix develop` on Linux machine
4. Build and test - should work identically!
