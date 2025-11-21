# Start Bob (Computer B - Connecting Node)
# This computer will connect to Alice

Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "  Starting Bob (Connecting Node)" -ForegroundColor Green
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host ""

Write-Host "🚀 Starting descord..." -ForegroundColor Cyan
Write-Host ""
Write-Host "After startup:" -ForegroundColor Yellow
Write-Host "  1. Get Alice's multiaddr from her terminal (network command)" -ForegroundColor White
Write-Host "  2. Type: connect /ip4/<ALICE_IP>/tcp/9001/p2p/<ALICE_PEER_ID>" -ForegroundColor White
Write-Host "  3. Get space_id and invite code from Alice" -ForegroundColor White
Write-Host "  4. Type: join <space_id> <invite_code>" -ForegroundColor White
Write-Host ""
Write-Host "Example:" -ForegroundColor Gray
Write-Host "  connect /ip4/192.168.1.100/tcp/9001/p2p/12D3KooWABC123..." -ForegroundColor DarkGray
Write-Host "  join 9d2bf8a78ca50c92... ABC12345" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Press Ctrl+C to stop" -ForegroundColor Gray
Write-Host ""

# Start Bob
.\target\release\descord.exe --account bob.key
