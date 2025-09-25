# Ruby Butler Test Setup
# Run this script once before running Pester tests
# Compiles Ruby Butler and sets up environment variables for testing

Write-Host "Ruby Butler Test Setup" -ForegroundColor Cyan
Write-Host "=====================" -ForegroundColor Cyan

# Build Ruby Butler in release mode
Write-Host "Building Ruby Butler..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\.."
try {
    $BuildOutput = cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed!" -ForegroundColor Red
        Write-Host $BuildOutput -ForegroundColor Red
        exit 1
    }
    Write-Host "Build completed successfully." -ForegroundColor Green
} finally {
    Pop-Location
}

# Set up environment variable for tests
$exe = if ($IsWindows) { ".exe" } else { "" }
$RbPath = Resolve-Path (Join-Path $PSScriptRoot ".." "target" "release" ("rb$exe"))
$env:RB_TEST_PATH = $RbPath

Write-Host ""
Write-Host "Environment Setup Complete!" -ForegroundColor Green
Write-Host "RB_TEST_PATH = $env:RB_TEST_PATH" -ForegroundColor Cyan
Write-Host ""
Write-Host "You can now run Pester tests repeatedly:" -ForegroundColor White
Write-Host "  Invoke-Pester .\tests\" -ForegroundColor Gray
Write-Host ""
