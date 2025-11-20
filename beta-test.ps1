# Descord Beta Test Runner
# Windows PowerShell Script

Write-Host ""
Write-Host "╔═══════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           DESCORD BETA TEST AUTOMATION SCRIPT                    ║" -ForegroundColor Cyan
Write-Host "╚═══════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

$RELAY_RUNNING = $false

# Function to check if relay is running
function Test-RelayRunning {
    try {
        $response = Invoke-WebRequest -Uri "http://localhost:8080/stats" -TimeoutSec 2 -ErrorAction SilentlyContinue
        return $true
    } catch {
        return $false
    }
}

# Function to start relay server
function Start-RelayServer {
    Write-Host "🚀 Starting relay server..." -ForegroundColor Yellow
    Write-Host ""
    
    $relay = Start-Process -FilePath "cargo" -ArgumentList "run", "--package", "descord-relay", "--release" -PassThru -WindowStyle Normal
    
    Write-Host "⏳ Waiting for relay to start..." -ForegroundColor Yellow
    $timeout = 30
    $elapsed = 0
    
    while (-not (Test-RelayRunning) -and $elapsed -lt $timeout) {
        Start-Sleep -Seconds 1
        $elapsed++
        Write-Host "." -NoNewline
    }
    
    Write-Host ""
    
    if (Test-RelayRunning) {
        Write-Host "✅ Relay server is running!" -ForegroundColor Green
        Write-Host ""
        return $relay
    } else {
        Write-Host "❌ Relay server failed to start" -ForegroundColor Red
        Write-Host ""
        return $null
    }
}

# Main menu
function Show-Menu {
    Write-Host "📋 Select Beta Test Option:" -ForegroundColor White
    Write-Host ""
    Write-Host "  1. Quick Test (runs automated beta test)" -ForegroundColor White
    Write-Host "  2. Full Test (with relay server auto-start)" -ForegroundColor White
    Write-Host "  3. Run All Unit Tests" -ForegroundColor White
    Write-Host "  4. Run All Integration Tests" -ForegroundColor White
    Write-Host "  5. Check Relay Status" -ForegroundColor White
    Write-Host "  6. View Relay Stats" -ForegroundColor White
    Write-Host "  7. Exit" -ForegroundColor White
    Write-Host ""
}

# Check relay status
function Show-RelayStatus {
    if (Test-RelayRunning) {
        Write-Host "✅ Relay server is running" -ForegroundColor Green
        Write-Host ""
        try {
            $stats = Invoke-RestMethod -Uri "http://localhost:8080/stats" -TimeoutSec 2
            Write-Host "📊 Relay Statistics:" -ForegroundColor Cyan
            Write-Host "   Active connections: $($stats.active_connections)"
            Write-Host "   Total bytes sent: $([math]::Round($stats.total_bytes_sent / 1MB, 2)) MB"
            Write-Host "   Total bytes received: $([math]::Round($stats.total_bytes_received / 1MB, 2)) MB"
            Write-Host "   Uptime: $([math]::Round($stats.uptime_seconds / 60, 2)) minutes"
            Write-Host "   Reputation: $($stats.relay_reputation)"
            Write-Host ""
        } catch {
            Write-Host "⚠️  Could not fetch stats" -ForegroundColor Yellow
        }
    } else {
        Write-Host "❌ Relay server is not running" -ForegroundColor Red
        Write-Host ""
        Write-Host "To start relay server:" -ForegroundColor Yellow
        Write-Host "  cargo run --package descord-relay --release" -ForegroundColor Gray
        Write-Host ""
    }
}

# Run beta test
function Run-BetaTest {
    param([bool]$StartRelay = $false)
    
    $relayProcess = $null
    
    if ($StartRelay) {
        if (-not (Test-RelayRunning)) {
            $relayProcess = Start-RelayServer
            if ($null -eq $relayProcess) {
                Write-Host "❌ Cannot run beta test without relay server" -ForegroundColor Red
                return
            }
            Start-Sleep -Seconds 3
        } else {
            Write-Host "✅ Relay already running" -ForegroundColor Green
            Write-Host ""
        }
    } else {
        if (-not (Test-RelayRunning)) {
            Write-Host "⚠️  WARNING: Relay server not detected!" -ForegroundColor Yellow
            Write-Host ""
            Write-Host "Beta test requires relay server. Options:" -ForegroundColor Yellow
            Write-Host "  1. Start relay manually: cargo run --package descord-relay --release"
            Write-Host "  2. Use option 2 (Full Test) to auto-start relay"
            Write-Host ""
            $continue = Read-Host "Continue anyway? (y/n)"
            if ($continue -ne 'y') {
                return
            }
        }
    }
    
    Write-Host "🧪 Running automated beta test..." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host ""
    
    cargo test --package descord-core --test beta_test -- --ignored --nocapture
    
    $exitCode = $LASTEXITCODE
    
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Gray
    Write-Host ""
    
    if ($exitCode -eq 0) {
        Write-Host "✅ BETA TEST PASSED!" -ForegroundColor Green
    } else {
        Write-Host "❌ BETA TEST FAILED" -ForegroundColor Red
        Write-Host "   Exit code: $exitCode" -ForegroundColor Red
    }
    
    Write-Host ""
    
    if ($null -ne $relayProcess -and -not $relayProcess.HasExited) {
        Write-Host "🛑 Stopping relay server..." -ForegroundColor Yellow
        Stop-Process -Id $relayProcess.Id -Force
        Write-Host "✅ Relay stopped" -ForegroundColor Green
        Write-Host ""
    }
}

# Run unit tests
function Run-UnitTests {
    Write-Host "🧪 Running all unit tests..." -ForegroundColor Yellow
    Write-Host ""
    
    cargo test --package descord-core --lib -- --test-threads=1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ ALL UNIT TESTS PASSED!" -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "❌ SOME TESTS FAILED" -ForegroundColor Red
    }
    
    Write-Host ""
}

# Run integration tests
function Run-IntegrationTests {
    Write-Host "🧪 Running all integration tests..." -ForegroundColor Yellow
    Write-Host ""
    
    cargo test --package descord-core --test '*' --test-threads=1
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✅ ALL INTEGRATION TESTS PASSED!" -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "❌ SOME TESTS FAILED" -ForegroundColor Red
    }
    
    Write-Host ""
}

# Main loop
while ($true) {
    Show-Menu
    $choice = Read-Host "Enter choice (1-7)"
    Write-Host ""
    
    switch ($choice) {
        "1" {
            Run-BetaTest -StartRelay $false
            Read-Host "Press Enter to continue"
        }
        "2" {
            Run-BetaTest -StartRelay $true
            Read-Host "Press Enter to continue"
        }
        "3" {
            Run-UnitTests
            Read-Host "Press Enter to continue"
        }
        "4" {
            Run-IntegrationTests
            Read-Host "Press Enter to continue"
        }
        "5" {
            Show-RelayStatus
            Read-Host "Press Enter to continue"
        }
        "6" {
            if (Test-RelayRunning) {
                try {
                    $stats = Invoke-RestMethod -Uri "http://localhost:8080/stats"
                    Write-Host ($stats | ConvertTo-Json -Depth 10) -ForegroundColor Cyan
                    Write-Host ""
                } catch {
                    Write-Host "❌ Failed to fetch stats" -ForegroundColor Red
                }
            } else {
                Write-Host "❌ Relay not running" -ForegroundColor Red
            }
            Write-Host ""
            Read-Host "Press Enter to continue"
        }
        "7" {
            Write-Host "👋 Goodbye!" -ForegroundColor Cyan
            Write-Host ""
            exit
        }
        default {
            Write-Host "❌ Invalid choice" -ForegroundColor Red
            Write-Host ""
        }
    }
    
    Clear-Host
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║           DESCORD BETA TEST AUTOMATION SCRIPT                    ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
}
