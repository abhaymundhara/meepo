#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$TaskName = "Meepo"

Write-Host "Uninstalling Meepo scheduled task..."

$task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($task) {
    Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    Write-Host "Removed scheduled task '$TaskName'" -ForegroundColor Green
} else {
    Write-Host "No scheduled task found named '$TaskName'"
}

Write-Host ""
Write-Host "Meepo scheduled task uninstalled."
Write-Host "Config and data remain at ~/.meepo/ - delete manually if desired."
