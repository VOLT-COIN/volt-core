$PSScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$env:PATH = "$PSScriptRoot\gcc_toolchain\w64devkit\bin;$env:PATH"

Write-Host "🚀 Starting Volt PPLNS Pool (High Performance Mode)..." -ForegroundColor Cyan

if (-not (Test-Path "$PSScriptRoot\pool_key.txt")) {
    Write-Host "⚠️ WARNING: pool_key.txt not found!" -ForegroundColor Yellow
    Write-Host "Please create pool_key.txt and paste your private key inside." -ForegroundColor Yellow
    Write-Host "Using temporary placeholder key (Rewards will be lost!)..." -ForegroundColor Red
} else {
    Write-Host "✅ pool_key.txt found. Loading private key..." -ForegroundColor Green
}

Write-Host "Listening on P2P Port 6000 and Stratum Port 3333..."
./target/x86_64-pc-windows-gnu/release/volt_core.exe 6000 --mine
