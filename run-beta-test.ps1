# Descord Beta Test Runner (Simplified)
# Windows PowerShell Script

# Check if relay is running
function Test-RelayRunning {
    try {
        $null = Invoke-WebRequest -Uri "http://localhost:8080/stats" -TimeoutSec 2 -ErrorAction Stop
        return $true
    }
    catch {
        return $false
    }
}

# Run beta test
function Start-BetaTest {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║              DESCORD AUTOMATED BETA TEST                          ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
    
    if (-not (Test-RelayRunning)) {
        Write-Host "⚠️  WARNING: Relay server not detected!" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "The beta test requires a relay server to be running." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "To start the relay server, open a new terminal and run:" -ForegroundColor White
        Write-Host "  cargo run --package descord-relay --release" -ForegroundColor Cyan
        Write-Host ""
        $continue = Read-Host "Continue without relay? (yes/no)"
        if ($continue -ne "yes") {
            Write-Host "Exiting..." -ForegroundColor Gray
            return
        }
    }
    else {
        Write-Host "✅ Relay server detected and running" -ForegroundColor Green
        Write-Host ""
    }
    
    Write-Host "🧪 Starting automated beta test..." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "This will test:" -ForegroundColor White
    Write-Host "  • 3 users (Alice, Bob, Charlie)" -ForegroundColor Gray
    Write-Host "  • Relay connections (IP privacy)" -ForegroundColor Gray
    Write-Host "  • Space & channel creation" -ForegroundColor Gray
    Write-Host "  • DHT peer discovery" -ForegroundColor Gray
    Write-Host "  • E2EE messaging" -ForegroundColor Gray
    Write-Host "  • Relay rotation" -ForegroundColor Gray
    Write-Host "  • Privacy verification" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Duration: ~60 seconds" -ForegroundColor White
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
        Write-Host ""
        Write-Host "All systems operational. Ready for beta testing!" -ForegroundColor White
    }
    else {
        Write-Host "❌ BETA TEST FAILED (Exit code: $exitCode)" -ForegroundColor Red
        Write-Host ""
        Write-Host "Common issues:" -ForegroundColor Yellow
        Write-Host "  • Relay server not running" -ForegroundColor Gray
        Write-Host "  • Port 8080 or 9000 already in use" -ForegroundColor Gray
        Write-Host "  • Firewall blocking connections" -ForegroundColor Gray
    }
    
    Write-Host ""
}

# Main execution
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "           DESCORD BETA TEST AUTOMATION SCRIPT                     " -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════════════════" -ForegroundColor Cyan

Start-BetaTest

Write-Host ""
Write-Host "📚 Documentation:" -ForegroundColor White
Write-Host "  • BETA_QUICK_START.md - Quick reference" -ForegroundColor Gray
Write-Host "  • BETA_TESTING.md - Complete guide" -ForegroundColor Gray
Write-Host "  • SECURITY_ANALYSIS.md - Privacy analysis" -ForegroundColor Gray
Write-Host ""
