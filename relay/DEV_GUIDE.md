# Descord Relay Server - Development Helper Scripts

## Running the Relay Server

### For Development (with visible output in VS Code terminal)

```powershell
# Build and run in foreground (recommended)
cargo run --package descord-relay --release

# Stop with Ctrl+C
```

### For Testing (background with log file)

```powershell
# Start relay with output to file
Start-Process -FilePath ".\target\release\descord-relay.exe" `
    -RedirectStandardOutput "relay-stdout.log" `
    -RedirectStandardError "relay-stderr.log" `
    -WindowStyle Hidden

# View logs in real-time
Get-Content relay-stdout.log -Wait -Tail 20

# Stop relay
Stop-Process -Name "descord-relay" -Force
```

### For CI/Production (systemd on Linux)

```bash
# On Linux VPS with systemd
sudo systemctl start descord-relay
sudo journalctl -u descord-relay -f
```

## Quick Commands

### Start Relay (Foreground)
```powershell
cargo run --package descord-relay --release
```

### Start Relay (Background with Logging)
```powershell
# Create logs directory
New-Item -ItemType Directory -Force -Path ".\logs"

# Start relay
$relay = Start-Process -FilePath ".\target\release\descord-relay.exe" `
    -RedirectStandardOutput ".\logs\relay.log" `
    -RedirectStandardError ".\logs\relay-error.log" `
    -PassThru `
    -WindowStyle Hidden

Write-Host "Relay started with PID: $($relay.Id)"
Write-Host "Logs: .\logs\relay.log"

# Tail logs
Get-Content ".\logs\relay.log" -Wait -Tail 10
```

### Stop Relay
```powershell
Stop-Process -Name "descord-relay" -Force -ErrorAction SilentlyContinue
```

### Check if Relay is Running
```powershell
Get-Process -Name "descord-relay" -ErrorAction SilentlyContinue
```

### View Relay Logs
```powershell
# Live tail
Get-Content ".\logs\relay.log" -Wait -Tail 20

# Full log
Get-Content ".\logs\relay.log"

# Last 50 lines
Get-Content ".\logs\relay.log" -Tail 50
```

## For You to Use in VS Code Terminal

### Terminal 1: Run Relay Server
```powershell
# Just run it directly - output will be visible
cargo run --package descord-relay --release
```

### Terminal 2: Run Tests
```powershell
# In another terminal tab
cargo test --test network_integration_test test_relay_connection -- --ignored --nocapture
```

### Terminal 3: Monitor (Optional)
```powershell
# Watch relay process
while ($true) { 
    Get-Process -Name "descord-relay" -ErrorAction SilentlyContinue | 
    Format-Table Id, CPU, WS -AutoSize
    Start-Sleep -Seconds 2
}
```

## Logging Configuration

### Change Log Level
```powershell
# More verbose logging
$env:RUST_LOG="debug"
cargo run --package descord-relay --release

# Only errors
$env:RUST_LOG="error"
cargo run --package descord-relay --release

# Default (info)
$env:RUST_LOG="info"
cargo run --package descord-relay --release
```

### Filter Logs
```powershell
# Only relay events (hide libp2p internals)
$env:RUST_LOG="descord_relay=info,libp2p=warn"
cargo run --package descord-relay --release
```

## Best Practice for Development

**Recommended setup:**

1. **Open 2 terminal tabs in VS Code**

2. **Terminal 1 (Relay Server)**:
   ```powershell
   cargo run --package descord-relay --release
   ```
   Keep this running, watch the logs live

3. **Terminal 2 (Testing)**:
   ```powershell
   cargo test --test network_integration_test test_relay_connection -- --ignored --nocapture
   ```
   Run tests while relay is running in Terminal 1

This way:
- ✅ All output is visible in VS Code
- ✅ I can see logs directly from the terminals
- ✅ Easy to stop (Ctrl+C in Terminal 1)
- ✅ No copy-pasting needed
- ✅ Full color and formatting preserved

## The Answer to Your Question

**YES!** The best way is to simply run:

```powershell
cargo run --package descord-relay --release
```

In a VS Code terminal tab. This way:
- The output appears directly in the terminal
- I can see it when you share terminal output
- You can still use other terminal tabs for other commands
- Much easier than the popup window!
