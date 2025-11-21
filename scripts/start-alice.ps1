# Start Alice (Computer A - Listening Node)
# This computer will accept incoming connections

Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "  Starting Alice (Listening Node)" -ForegroundColor Green
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host ""

# Get local IP address
$localIP = (Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.*" } | Select-Object -First 1).IPAddress

if ($localIP) {
    Write-Host "📡 Your IP Address: " -NoNewline -ForegroundColor Yellow
    Write-Host $localIP -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "⚠️  Could not detect IP address - check manually with 'ipconfig'" -ForegroundColor Red
    Write-Host ""
}

Write-Host "🚀 Starting descord on port 9001..." -ForegroundColor Cyan
Write-Host ""
Write-Host "After startup:" -ForegroundColor Yellow
Write-Host "  1. Type 'network' to see your full multiaddr" -ForegroundColor White
Write-Host "  2. Share the multiaddr with Bob (replace 0.0.0.0 with $localIP)" -ForegroundColor White
Write-Host "  3. Type 'space MySpace' to create a space" -ForegroundColor White
Write-Host "  4. Type 'invite' to create an invite code" -ForegroundColor White
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor Gray
Write-Host ""

# Start Alice
.\target\release\descord.exe --account alice.key --port 9001
