# Quick test runner for Descord
# Usage: .\test.ps1 [quick|privacy|invite|relay|all]

param(
    [Parameter(Position=0)]
    [ValidateSet('quick', 'privacy', 'invite', 'visibility', 'relay', 'network', 'all', 'unit')]
    [string]$Suite = 'quick'
)

Write-Host "`nRunning Descord Tests: $Suite`n" -ForegroundColor Cyan

switch ($Suite) {
    'quick' {
        Write-Host "Quick unit tests (skipping slow convergence tests)..." -ForegroundColor Yellow
        cargo test --lib -- --skip convergence --skip three_person
    }
    'privacy' {
        Write-Host "Privacy tier tests (7 tests, ~3 seconds)..." -ForegroundColor Yellow
        cargo test --test privacy_tiers_test
    }
    'invite' {
        Write-Host "Invite system tests (11 tests, ~5 seconds)..." -ForegroundColor Yellow
        cargo test --test invite_system_test
    }
    'visibility' {
        Write-Host "Space visibility tests (5 tests, ~2 seconds)..." -ForegroundColor Yellow
        cargo test --test space_visibility_test
    }
    'relay' {
        Write-Host "Relay tests..." -ForegroundColor Yellow
        cargo test relay
    }
    'network' {
        Write-Host "Network integration tests..." -ForegroundColor Yellow
        cargo test --test integration_test
    }
    'unit' {
        Write-Host "Unit tests only (no integration)..." -ForegroundColor Yellow
        cargo test --lib
    }
    'all' {
        Write-Host "Full test suite (all 87 tests, ~7 minutes)..." -ForegroundColor Yellow
        cargo test
    }
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "`nTests passed!`n" -ForegroundColor Green
} else {
    Write-Host "`nTests failed!`n" -ForegroundColor Red
}
