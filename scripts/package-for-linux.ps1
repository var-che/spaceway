# Package Descord for Linux Transfer
# This creates a zip file you can copy to your Linux computer

Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "  Packaging Descord for Linux Transfer" -ForegroundColor Green
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host ""

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$zipName = "descord-linux-$timestamp.zip"

Write-Host "Creating package: $zipName" -ForegroundColor Yellow
Write-Host ""

# Files to include for Linux
$files = @(
    "core",
    "cli", 
    "relay",
    "Cargo.toml",
    "Cargo.lock",
    "start-bob.sh",
    "start-alice.sh",
    "LINUX_SETUP.md",
    "CROSS_PLATFORM_TEST.md",
    "QUICK_START.md",
    "TWO_COMPUTER_SETUP.md",
    "README.md"
)

# Create temp directory
$tempDir = "temp-linux-package"
if (Test-Path $tempDir) {
    Remove-Item -Recurse -Force $tempDir
}
New-Item -ItemType Directory -Path $tempDir | Out-Null

# Copy files
Write-Host "Copying files..." -ForegroundColor Cyan
foreach ($file in $files) {
    if (Test-Path $file) {
        Copy-Item -Path $file -Destination $tempDir -Recurse
        Write-Host "  ✓ $file" -ForegroundColor Green
    }
}

# Create zip
Write-Host ""
Write-Host "Creating zip file..." -ForegroundColor Cyan
Compress-Archive -Path "$tempDir\*" -DestinationPath $zipName -Force

# Cleanup
Remove-Item -Recurse -Force $tempDir

# Show result
$size = (Get-Item $zipName).Length / 1MB
Write-Host ""
Write-Host "=" * 60 -ForegroundColor Green
Write-Host "  ✓ Package created successfully!" -ForegroundColor Green
Write-Host "=" * 60 -ForegroundColor Green
Write-Host ""
Write-Host "File: $zipName" -ForegroundColor Yellow
Write-Host "Size: $([math]::Round($size, 2)) MB" -ForegroundColor Yellow
Write-Host ""
Write-Host "Transfer this file to your Linux computer via:" -ForegroundColor Cyan
Write-Host "  • USB drive" -ForegroundColor White
Write-Host "  • Network share" -ForegroundColor White
Write-Host "  • SCP: scp $zipName user@linux-pc:~/" -ForegroundColor White
Write-Host ""
Write-Host "On Linux, unzip and build:" -ForegroundColor Cyan
Write-Host "  unzip $zipName" -ForegroundColor White
Write-Host "  cd descord" -ForegroundColor White
Write-Host "  cargo build --release --bin descord" -ForegroundColor White
Write-Host "  chmod +x start-bob.sh" -ForegroundColor White
Write-Host "  ./start-bob.sh" -ForegroundColor White
Write-Host ""
