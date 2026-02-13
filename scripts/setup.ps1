#Requires -Version 5.1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ╔══════════════════════════════════════════════════════════════════╗
# ║                     Meepo Setup Script                          ║
# ║                                                                 ║
# ║  Interactive first-time setup for Windows. Builds the binary,   ║
# ║  initializes config, walks through API keys, enables channels.  ║
# ╚══════════════════════════════════════════════════════════════════╝

$RepoDir = Split-Path -Parent $PSScriptRoot
$ConfigDir = Join-Path $env:USERPROFILE ".meepo"
$ConfigFile = Join-Path $ConfigDir "config.toml"
$TotalSteps = 7

function Print-Step($step, $title) {
    Write-Host ""
    Write-Host "[$step/$TotalSteps] $title" -ForegroundColor Blue -NoNewline
    Write-Host ""
    Write-Host ("-" * 50) -ForegroundColor DarkGray
}

function Print-Ok($msg) { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Print-Warn($msg) { Write-Host "  [!] $msg" -ForegroundColor Yellow }
function Print-Err($msg) { Write-Host "  [X] $msg" -ForegroundColor Red }
function Print-Dim($msg) { Write-Host "  $msg" -ForegroundColor DarkGray }
function Print-Url($url) { Write-Host "  -> $url" -ForegroundColor Cyan }

function Ask-YN($prompt, $default = "n") {
    if ($default -eq "y") {
        $choice = Read-Host "  $prompt [Y/n]"
        if ([string]::IsNullOrEmpty($choice)) { $choice = "y" }
    } else {
        $choice = Read-Host "  $prompt [y/N]"
        if ([string]::IsNullOrEmpty($choice)) { $choice = "n" }
    }
    return $choice -match "^[Yy]"
}

function Ask-Value($prompt, $default = "") {
    if ($default) {
        $value = Read-Host "  $prompt [$default]"
        if ([string]::IsNullOrEmpty($value)) { return $default }
        return $value
    } else {
        return Read-Host "  $prompt"
    }
}

function Save-EnvVar($name, $value, $comment = "") {
    [Environment]::SetEnvironmentVariable($name, $value, "User")
    Set-Item -Path "Env:\$name" -Value $value  # Also set for current session
    Print-Dim "Saved to User environment variables"
}

function Capture-Key($displayName, $url, $envVar, $prefix = "", $comment = "Meepo") {
    $current = [Environment]::GetEnvironmentVariable($envVar, "User")
    if (-not [string]::IsNullOrEmpty($current)) {
        $masked = $current.Substring(0, [Math]::Min(7, $current.Length)) + "..." + $current.Substring([Math]::Max(0, $current.Length - 4))
        Print-Ok "$envVar already set ($masked)"
        Set-Item -Path "Env:\$envVar" -Value $current
        return $true
    }

    Write-Host ""
    Print-Url $url
    Write-Host ""

    if (Ask-YN "Open this in your browser?") {
        Start-Process $url
        Print-Dim "Opened in browser - switch over and grab the key"
        Write-Host ""
    }

    Print-Dim "Paste the key below, or copy it and press Enter to read from clipboard."
    $keyInput = Read-Host "  $displayName key"

    if ([string]::IsNullOrEmpty($keyInput)) {
        try { $keyInput = Get-Clipboard -ErrorAction SilentlyContinue }
        catch { $keyInput = "" }
        if (-not [string]::IsNullOrEmpty($keyInput)) {
            Print-Dim "Read from clipboard"
        }
    }

    if ([string]::IsNullOrEmpty($keyInput)) {
        Print-Warn "No key entered - set $envVar later"
        Print-Dim "`$env:$envVar = `"...`""
        return $false
    }

    if ($prefix -and -not $keyInput.StartsWith($prefix)) {
        Print-Warn "Key doesn't start with '$prefix' - may not be valid, saving anyway"
    } else {
        Print-Ok "Key captured"
    }

    Save-EnvVar $envVar $keyInput "$comment - $displayName"
    return $true
}

# ── Welcome ──

Clear-Host
Write-Host ""
Write-Host "    +===================================+" -ForegroundColor White
Write-Host "    |         meepo  setup              |" -ForegroundColor White
Write-Host "    |     local ai agent for Windows    |" -ForegroundColor White
Write-Host "    +===================================+" -ForegroundColor White
Write-Host ""

# ── Step 1: Prerequisites ──

Print-Step 1 "Prerequisites"

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $rustVersion = (rustc --version 2>$null) -replace "rustc ", ""
    Print-Ok "Rust $rustVersion"
} else {
    Print-Err "Rust not found"
    Write-Host ""
    Write-Host "  Install: https://rustup.rs" -ForegroundColor White
    exit 1
}

Print-Ok "Windows $([Environment]::OSVersion.Version)"

if (Get-Command gh -ErrorAction SilentlyContinue) { Print-Ok "GitHub CLI" }
else { Print-Dim "gh not found (optional - winget install GitHub.cli)" }

# ── Step 2: Build ──

Print-Step 2 "Build"

$BinaryPath = Join-Path $RepoDir "target\release\meepo.exe"

if (Test-Path $BinaryPath) {
    Print-Ok "Binary exists at $BinaryPath"
    if (Ask-YN "Rebuild?") {
        Print-Dim "Building..."
        Push-Location $RepoDir
        cargo build --release 2>&1 | Select-Object -Last 1
        Pop-Location
        Print-Ok "Build complete"
    }
} else {
    Print-Dim "First build - this takes ~2 minutes..."
    Push-Location $RepoDir
    cargo build --release 2>&1 | Select-Object -Last 1
    Pop-Location
    Print-Ok "Built $BinaryPath"
}

# ── Step 3: Config ──

Print-Step 3 "Configuration"

if (Test-Path $ConfigFile) {
    Print-Ok "Config exists at $ConfigFile"
    if (Ask-YN "Overwrite with fresh defaults?") {
        Remove-Item $ConfigFile -Force
        & $BinaryPath init 2>$null
        Print-Ok "Config re-initialized"
    }
} else {
    & $BinaryPath init 2>$null
    Print-Ok "Created $ConfigDir\ with config.toml, SOUL.md, MEMORY.md"
}

# ── Step 4: Anthropic API Key ──

Print-Step 4 "Anthropic API Key (required)"

Write-Host "  Powers all of Meepo's thinking via Claude."
Capture-Key "Anthropic" "https://console.anthropic.com/settings/keys" "ANTHROPIC_API_KEY" "sk-ant-" "Meepo"

# ── Step 5: Tavily API Key ──

Print-Step 5 "Tavily API Key (optional - web search)"

Write-Host "  Enables the web_search tool and cleaner URL extraction."
Write-Host "  Free tier available - no credit card needed."

if (Ask-YN "Set up Tavily?") {
    Capture-Key "Tavily" "https://app.tavily.com/home" "TAVILY_API_KEY" "tvly-" "Meepo - Tavily web search"
} else {
    Print-Dim "Skipped - web_search tool won't be available"
}

# ── Step 6: Channels ──

Print-Step 6 "Channels"

Write-Host "  Meepo can listen on Discord, Slack, and/or Alexa."
Print-Dim "You can also skip all channels and use 'meepo ask' from the CLI."
Write-Host ""

# ── Discord ──
if (Ask-YN "Enable Discord?") {
    Write-Host ""
    Write-Host "  Quick setup: Create a bot, copy its token, invite it to your server." -ForegroundColor White
    Print-Url "https://discord.com/developers/applications"

    if (Ask-YN "Open Discord Developer Portal?") {
        Start-Process "https://discord.com/developers/applications"
        Print-Dim "Opened in browser"
    }

    Write-Host ""
    Print-Dim "In the portal:"
    Print-Dim "  1. New Application -> `"Meepo`" -> Bot -> Reset Token -> copy it"
    Print-Dim "  2. Turn on MESSAGE CONTENT INTENT"
    Print-Dim "  3. OAuth2 -> URL Generator -> scope: bot -> Send Messages"
    Print-Dim "  4. Open the generated URL to invite bot to your server"
    Write-Host ""

    Print-Dim "Paste the bot token below, or copy it and press Enter for clipboard."
    $discordToken = Read-Host "  Bot token"

    if ([string]::IsNullOrEmpty($discordToken)) {
        try { $discordToken = Get-Clipboard -ErrorAction SilentlyContinue } catch {}
        if (-not [string]::IsNullOrEmpty($discordToken)) { Print-Dim "Read from clipboard" }
    }

    if (-not [string]::IsNullOrEmpty($discordToken)) {
        Save-EnvVar "DISCORD_BOT_TOKEN" $discordToken "Meepo - Discord bot"
        Print-Ok "Token saved"
    } else {
        Print-Warn "No token - set DISCORD_BOT_TOKEN later"
    }

    Write-Host ""
    Print-Dim "To get your user ID: enable Developer Mode in Discord settings,"
    Print-Dim "then right-click your name -> Copy User ID."
    $discordUser = Ask-Value "Your Discord user ID (or Enter to skip)"

    # Update config
    (Get-Content $ConfigFile) -replace '^(enabled = false.*# \[channels\.discord\])', 'enabled = true' |
        Set-Content $ConfigFile -ErrorAction SilentlyContinue
    $content = Get-Content $ConfigFile -Raw
    $content = $content -replace '(?m)(\[channels\.discord\]\r?\nenabled = )false', '${1}true'
    if ($discordUser) {
        $content = $content -replace '(?m)(allowed_users = )\[\]', "`${1}[`"$discordUser`"]"
    }
    Set-Content $ConfigFile $content

    Print-Ok "Discord enabled"
}

# ── Slack ──
Write-Host ""
if (Ask-YN "Enable Slack?") {
    Write-Host ""
    Write-Host "  Quick setup: Create a Slack app, add scopes, install to workspace." -ForegroundColor White
    Print-Url "https://api.slack.com/apps"

    if (Ask-YN "Open Slack API portal?") {
        Start-Process "https://api.slack.com/apps"
        Print-Dim "Opened in browser"
    }

    Write-Host ""
    Print-Dim "In the portal:"
    Print-Dim "  1. Create New App -> From scratch -> `"Meepo`""
    Print-Dim "  2. OAuth & Permissions -> add scopes:"
    Print-Dim "     chat:write, channels:read, im:history, im:read, users:read"
    Print-Dim "  3. Install to Workspace -> copy Bot User OAuth Token"
    Write-Host ""

    Print-Dim "Paste the token below, or copy it and press Enter for clipboard."
    $slackToken = Read-Host "  Bot token (xoxb-...)"

    if ([string]::IsNullOrEmpty($slackToken)) {
        try { $slackToken = Get-Clipboard -ErrorAction SilentlyContinue } catch {}
        if (-not [string]::IsNullOrEmpty($slackToken)) { Print-Dim "Read from clipboard" }
    }

    if (-not [string]::IsNullOrEmpty($slackToken)) {
        Save-EnvVar "SLACK_BOT_TOKEN" $slackToken "Meepo - Slack bot"
        Print-Ok "Token saved"
    } else {
        Print-Warn "No token - set SLACK_BOT_TOKEN later"
    }

    $content = Get-Content $ConfigFile -Raw
    $content = $content -replace '(?m)(\[channels\.slack\]\r?\nenabled = )false', '${1}true'
    Set-Content $ConfigFile $content

    Print-Ok "Slack enabled"
}

# ── Alexa ──
Write-Host ""
if (Ask-YN "Enable Alexa? (talk to Meepo via Amazon Echo)") {
    Write-Host ""
    Write-Host "  Quick setup: Create a custom Alexa Skill and copy the Skill ID." -ForegroundColor White
    Print-Url "https://developer.amazon.com/alexa/console/ask"

    if (Ask-YN "Open Alexa Developer Console?") {
        Start-Process "https://developer.amazon.com/alexa/console/ask"
        Print-Dim "Opened in browser"
    }

    Write-Host ""
    Print-Dim "In the console:"
    Print-Dim "  1. Create Skill -> Custom -> `"Meepo`""
    Print-Dim "  2. Set endpoint to your Meepo instance URL"
    Print-Dim "     (use ngrok for local dev: ngrok http 3000)"
    Print-Dim "  3. Copy the Skill ID (starts with amzn1.ask.skill....)"
    Write-Host ""

    $alexaSkill = Ask-Value "Alexa Skill ID (or Enter to skip)"

    if (-not [string]::IsNullOrEmpty($alexaSkill)) {
        Save-EnvVar "ALEXA_SKILL_ID" $alexaSkill "Meepo - Alexa skill"
        Print-Ok "Skill ID saved"
    } else {
        Print-Warn "No skill ID - set ALEXA_SKILL_ID later"
    }

    $content = Get-Content $ConfigFile -Raw
    $content = $content -replace '(?m)(\[channels\.alexa\]\r?\nenabled = )false', '${1}true'
    Set-Content $ConfigFile $content

    Print-Ok "Alexa enabled"
}

# ── Step 7: Verify ──

Print-Step 7 "Verify"

$apiKey = [Environment]::GetEnvironmentVariable("ANTHROPIC_API_KEY", "User")
if (-not [string]::IsNullOrEmpty($apiKey)) {
    Print-Dim "Testing API connection..."
    try {
        $testOutput = & $BinaryPath ask "Say 'hello' in one word." 2>$null
        if ($testOutput) {
            Write-Host "  $($testOutput | Select-Object -First 5)"
            Print-Ok "API connection works"
        } else {
            Print-Warn "API test failed - check your key"
        }
    } catch {
        Print-Warn "API test failed - check your key"
    }
} else {
    Print-Warn "No API key set - skipping connection test"
}

# ── Summary ──

Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Blue
Write-Host ""

Write-Host "  Files" -ForegroundColor White
Print-Dim "Config   $ConfigFile"
Print-Dim "Binary   $BinaryPath"
Print-Dim "Soul     $(Join-Path $ConfigDir 'workspace\SOUL.md')"
Print-Dim "Memory   $(Join-Path $ConfigDir 'workspace\MEMORY.md')"
Write-Host ""

Write-Host "  Keys" -ForegroundColor White
foreach ($var in @("ANTHROPIC_API_KEY", "TAVILY_API_KEY", "DISCORD_BOT_TOKEN", "SLACK_BOT_TOKEN")) {
    $val = [Environment]::GetEnvironmentVariable($var, "User")
    if ($val) { Print-Ok $var }
    else { Print-Dim "$var (not set)" }
}
Write-Host ""

Write-Host "  Next steps" -ForegroundColor White
Write-Host "  > meepo start              " -ForegroundColor Cyan -NoNewline; Print-Dim "# start the daemon"
Write-Host "  > meepo ask `"Hello`"        " -ForegroundColor Cyan -NoNewline; Print-Dim "# one-shot question"
Write-Host "  > scripts\install.ps1      " -ForegroundColor Cyan -NoNewline; Print-Dim "# run on login"
Write-Host ""

Write-Host "  Note: iMessage and Mail.app tools are macOS-only." -ForegroundColor DarkGray
Write-Host "  On Windows, Meepo uses Outlook for email/calendar via its built-in tools." -ForegroundColor DarkGray
Write-Host ""
