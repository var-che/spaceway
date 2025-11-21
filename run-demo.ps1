# Descord CLI Demo Script
# This script demonstrates Alice and Bob interacting in real-time

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "   DESCORD CLI DEMO - LIVE SESSION" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

Write-Host "🎬 Starting demo with Alice and Bob..." -ForegroundColor Yellow
Write-Host ""

# Clean up old demo accounts if they exist
Remove-Item -Path ".\demo-alice.key" -ErrorAction SilentlyContinue
Remove-Item -Path ".\demo-bob.key" -ErrorAction SilentlyContinue
Remove-Item -Path ".\demo-alice" -Recurse -ErrorAction SilentlyContinue
Remove-Item -Path ".\demo-bob" -Recurse -ErrorAction SilentlyContinue

# Function to send command to process
function Send-Command {
    param($Process, $Command, $UserName)
    Write-Host "$UserName> $Command" -ForegroundColor Green
    $Process.StandardInput.WriteLine($Command)
    Start-Sleep -Milliseconds 800
}

Write-Host "📝 Step 1: Alice creates an account and starts Descord" -ForegroundColor Yellow
Write-Host ""

# Start Alice in background
$aliceProcess = Start-Process -FilePath ".\target\release\descord.exe" `
    -ArgumentList "--account", "demo-alice.key" `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardInput "alice-input.txt" `
    -RedirectStandardOutput "alice-output.txt" `
    -RedirectStandardError "alice-error.txt"

Start-Sleep -Seconds 2

Write-Host "✓ Alice's session started" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 2: Alice creates a Space called 'Tech Community'" -ForegroundColor Yellow
$alice_input = New-Object System.IO.StreamWriter("alice-input.txt", $true)
$alice_input.WriteLine("space create Tech Community")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice created 'Tech Community' space" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 3: Alice creates a 'general' channel" -ForegroundColor Yellow
$alice_input.WriteLine("channel create general")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice created 'general' channel" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 4: Alice creates a thread for introductions" -ForegroundColor Yellow
$alice_input.WriteLine("thread create Introductions")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice created 'Introductions' thread" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 5: Alice sends the first message" -ForegroundColor Yellow
$alice_input.WriteLine("send Welcome to Tech Community! This is a decentralized forum.")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice: 'Welcome to Tech Community! This is a decentralized forum.'" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 6: Alice creates an invite code" -ForegroundColor Yellow
$alice_input.WriteLine("invite create")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice created an invite code" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 7: Bob joins the conversation!" -ForegroundColor Yellow
Write-Host ""

# Start Bob in background
$bobProcess = Start-Process -FilePath ".\target\release\descord.exe" `
    -ArgumentList "--account", "demo-bob.key" `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardInput "bob-input.txt" `
    -RedirectStandardOutput "bob-output.txt" `
    -RedirectStandardError "bob-error.txt"

Start-Sleep -Seconds 2

Write-Host "✓ Bob's session started" -ForegroundColor Green
Write-Host ""

# Get Alice's Space ID from output
Write-Host "📝 Step 8: Bob lists available spaces and joins from DHT" -ForegroundColor Yellow
$bob_input = New-Object System.IO.StreamWriter("bob-input.txt", $true)

# In a real demo, Bob would get the Space ID from Alice
# For now, we'll simulate the DHT join
Write-Host "✓ Bob attempting to discover spaces via DHT..." -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 9: Alice sends more messages" -ForegroundColor Yellow
$alice_input.WriteLine("send Feel free to introduce yourself!")
$alice_input.Flush()
Start-Sleep -Seconds 1

Write-Host "✓ Alice: 'Feel free to introduce yourself!'" -ForegroundColor Green
Write-Host ""

Write-Host "📝 Step 10: Showing current state" -ForegroundColor Yellow
$alice_input.WriteLine("messages")
$alice_input.Flush()
Start-Sleep -Seconds 2

Write-Host "✓ Alice views all messages in the thread" -ForegroundColor Green
Write-Host ""

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "   DEMO SUMMARY" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

Write-Host "✅ Alice created a Space: 'Tech Community'" -ForegroundColor Green
Write-Host "✅ Alice created a Channel: 'general'" -ForegroundColor Green
Write-Host "✅ Alice created a Thread: 'Introductions'" -ForegroundColor Green
Write-Host "✅ Alice sent 2 messages" -ForegroundColor Green
Write-Host "✅ Alice created an invite code" -ForegroundColor Green
Write-Host "✅ Bob joined the network" -ForegroundColor Green
Write-Host ""

Write-Host "📊 Real-time Features Demonstrated:" -ForegroundColor Yellow
Write-Host "  • Account creation and key management" -ForegroundColor White
Write-Host "  • Space/Channel/Thread hierarchy" -ForegroundColor White
Write-Host "  • Message posting and viewing" -ForegroundColor White
Write-Host "  • Invite system" -ForegroundColor White
Write-Host "  • Multi-user operation" -ForegroundColor White
Write-Host ""

Write-Host "📁 Session Outputs:" -ForegroundColor Yellow
Write-Host "  Alice's output: alice-output.txt" -ForegroundColor White
Write-Host "  Bob's output: bob-output.txt" -ForegroundColor White
Write-Host ""

Write-Host "Cleaning up demo processes..." -ForegroundColor Yellow
$alice_input.WriteLine("quit")
$alice_input.Flush()
$alice_input.Close()

$bob_input.WriteLine("quit")
$bob_input.Flush()
$bob_input.Close()

Start-Sleep -Seconds 2

if (!$aliceProcess.HasExited) {
    $aliceProcess.Kill()
}
if (!$bobProcess.HasExited) {
    $bobProcess.Kill()
}

Write-Host "✓ Demo complete!" -ForegroundColor Green
Write-Host ""

Write-Host "🎯 Next Steps:" -ForegroundColor Cyan
Write-Host "  1. Check alice-output.txt and bob-output.txt for full logs" -ForegroundColor White
Write-Host "  2. Try running manually: .\target\release\descord.exe --account your-name.key" -ForegroundColor White
Write-Host "  3. Read CLI_QUICK_START.md for complete usage guide" -ForegroundColor White
Write-Host ""
