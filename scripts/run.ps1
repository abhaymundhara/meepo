#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = $PSScriptRoot
$ProjectDir = Split-Path -Parent $ScriptDir
$Binary = Join-Path $ProjectDir "target\release\meepo.exe"
$ConfigDir = Join-Path $env:USERPROFILE ".meepo"

# Build if binary doesn't exist or source is newer
if (-not (Test-Path $Binary)) {
    Write-Host "Binary not found - building release..." -ForegroundColor DarkGray
    cargo build --release --manifest-path (Join-Path $ProjectDir "Cargo.toml")
    Write-Host ""
} else {
    $newerSource = Get-ChildItem -Path (Join-Path $ProjectDir "crates") -Filter "*.rs" -Recurse |
        Where-Object { $_.LastWriteTime -gt (Get-Item $Binary).LastWriteTime } |
        Select-Object -First 1
    if ($newerSource) {
        Write-Host "Source changed - rebuilding..." -ForegroundColor DarkGray
        cargo build --release --manifest-path (Join-Path $ProjectDir "Cargo.toml")
        Write-Host ""
    }
}

# Initialize config if needed
if (-not (Test-Path $ConfigDir)) {
    Write-Host "First run detected." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "  Run the setup script for guided configuration:"
    Write-Host "  $ScriptDir\setup.ps1" -ForegroundColor White
    Write-Host ""
    Write-Host "  Or initialize a bare config with:"
    Write-Host "  $Binary init" -ForegroundColor DarkGray
    exit 0
}

# Check for API key
$apiKey = $env:ANTHROPIC_API_KEY
if (-not $apiKey) {
    $apiKey = [Environment]::GetEnvironmentVariable("ANTHROPIC_API_KEY", "User")
}
if (-not $apiKey) {
    Write-Host "ANTHROPIC_API_KEY is not set." -ForegroundColor Red
    Write-Host ""
    Write-Host "  Set it now:"
    Write-Host '  $env:ANTHROPIC_API_KEY = "sk-ant-..."' -ForegroundColor DarkGray
    Write-Host ""
    Write-Host "  Or run setup again:"
    Write-Host "  $ScriptDir\setup.ps1" -ForegroundColor DarkGray
    exit 1
}

# Pass all arguments through, default to "start" if none given
if ($args.Count -eq 0) {
    Write-Host "Starting Meepo..." -ForegroundColor Green
    & $Binary start
} else {
    & $Binary @args
}
