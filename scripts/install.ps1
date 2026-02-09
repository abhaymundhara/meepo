#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$TaskName = "Meepo"
$RepoDir = Split-Path -Parent $PSScriptRoot

# Find binary
$BinaryPath = ""
$candidates = @(
    (Join-Path $env:USERPROFILE ".cargo\bin\meepo.exe"),
    (Join-Path $RepoDir "target\release\meepo.exe")
)
foreach ($path in $candidates) {
    if (Test-Path $path) { $BinaryPath = $path; break }
}
if (-not $BinaryPath -or -not (Test-Path $BinaryPath)) {
    $found = Get-Command meepo -ErrorAction SilentlyContinue
    if ($found) { $BinaryPath = $found.Source }
}

if (-not $BinaryPath) {
    Write-Host "Error: Meepo binary not found." -ForegroundColor Red
    Write-Host ""
    Write-Host "Build it first with one of:"
    Write-Host "  cargo build --release              # Binary at target\release\meepo.exe"
    Write-Host "  cargo install --path crates\meepo-cli  # Binary at ~/.cargo/bin/meepo.exe"
    exit 1
}

Write-Host "Installing Meepo as a Windows scheduled task..." -ForegroundColor Blue
Write-Host "Using binary: $BinaryPath"

# Create log directory
$LogDir = Join-Path $env:USERPROFILE ".meepo\logs"
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

# Remove existing task if present
Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue

# Create scheduled task that runs at logon
$action = New-ScheduledTaskAction -Execute $BinaryPath -Argument "start" -WorkingDirectory (Split-Path $BinaryPath)
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -RunLevel Limited

Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description "Meepo local AI agent"

Write-Host ""
Write-Host "Meepo installed as scheduled task '$TaskName'." -ForegroundColor Green
Write-Host ""

# Show env var status
Write-Host "Environment variables (User scope):"
foreach ($var in @("ANTHROPIC_API_KEY", "TAVILY_API_KEY", "DISCORD_BOT_TOKEN", "SLACK_BOT_TOKEN")) {
    $val = [Environment]::GetEnvironmentVariable($var, "User")
    if ($val) { Write-Host "  [OK] $var" -ForegroundColor Green }
    else { Write-Host "  [-] $var (not set)" -ForegroundColor DarkGray }
}
Write-Host ""
Write-Host "Note: The scheduled task inherits User environment variables at logon."
Write-Host ""

# Start immediately
Start-ScheduledTask -TaskName $TaskName
Write-Host "Meepo started and will run on login." -ForegroundColor Green
Write-Host ""
Write-Host "Commands:"
Write-Host "  Stop-ScheduledTask -TaskName $TaskName          # Stop"
Write-Host "  Start-ScheduledTask -TaskName $TaskName         # Start"
Write-Host "  Unregister-ScheduledTask -TaskName $TaskName    # Uninstall"
Write-Host "  Get-Content $(Join-Path $LogDir 'meepo.out.log') -Tail 20  # View logs"
