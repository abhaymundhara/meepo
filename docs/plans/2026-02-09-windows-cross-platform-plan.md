# Windows Cross-Platform Support — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make Meepo compile and run on Windows with full feature parity — Windows-native equivalents for all macOS tools, conditional compilation, cross-platform scripts, and Windows service installation.

**Architecture:** Introduce a platform abstraction layer (`platform/` module in meepo-core) with traits for each OS-specific capability. macOS implementations use existing AppleScript code; Windows implementations use PowerShell + Outlook COM, Windows UI Automation, and native APIs. Tools hold `Box<dyn PlatformTrait>` and get the correct impl injected at startup via `#[cfg(target_os)]`. iMessage and Email channels use `#[cfg(target_os = "macos")]` to compile only on macOS.

**Tech Stack:** `arboard` (cross-platform clipboard), `open` crate (cross-platform app launcher), PowerShell COM scripting for Outlook email/calendar on Windows, `powershell_script` or raw `Command::new("powershell")` for Windows scripting bridge, `#[cfg]` attributes for conditional compilation, PowerShell scripts replacing bash for Windows setup/install.

---

### Task 1: Add cross-platform dependencies to workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml` (workspace root, lines 16-39)
- Modify: `crates/meepo-core/Cargo.toml`

**Step 1: Add new dependencies to workspace Cargo.toml**

Add after line 39 (`glob = "0.3"`):

```toml
arboard = "3"
open = "5"
```

**Step 2: Add platform-conditional dependencies to meepo-core/Cargo.toml**

Add after `glob = { workspace = true }`:

```toml
arboard = { workspace = true }
open = { workspace = true }
```

**Step 3: Verify it compiles**

Run: `cargo check -p meepo-core`
Expected: Compiles with warnings but no errors.

**Step 4: Commit**

```bash
git add Cargo.toml crates/meepo-core/Cargo.toml
git commit -m "deps: add arboard and open crates for cross-platform support"
```

---

### Task 2: Create platform abstraction traits in `platform/mod.rs`

**Files:**
- Create: `crates/meepo-core/src/platform/mod.rs`
- Create: `crates/meepo-core/src/platform/macos.rs`
- Create: `crates/meepo-core/src/platform/windows.rs`
- Modify: `crates/meepo-core/src/tools/mod.rs` (add `pub mod platform;` to lib, not tools)

Actually, the platform module should be at the crate root level:
- Modify: `crates/meepo-core/src/lib.rs` or whatever the crate root is

**Step 1: Check crate root**

Run: `cat crates/meepo-core/src/lib.rs`
We need to add `pub mod platform;` to it.

**Step 2: Create `platform/mod.rs` with trait definitions**

Create: `crates/meepo-core/src/platform/mod.rs`

```rust
//! Platform abstraction layer for OS-specific functionality
//!
//! Provides trait definitions and platform-specific implementations.
//! On macOS: AppleScript-based implementations.
//! On Windows: PowerShell/COM-based implementations.

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use anyhow::Result;
use async_trait::async_trait;

/// Email provider for reading and sending emails
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Read recent emails from the system email client
    async fn read_emails(&self, limit: u64, mailbox: &str, search: Option<&str>) -> Result<String>;
    /// Send an email, optionally as a reply
    async fn send_email(&self, to: &str, subject: &str, body: &str, cc: Option<&str>, in_reply_to: Option<&str>) -> Result<String>;
}

/// Calendar provider for reading and creating events
#[async_trait]
pub trait CalendarProvider: Send + Sync {
    /// Read calendar events for the given number of days ahead
    async fn read_events(&self, days_ahead: u64) -> Result<String>;
    /// Create a new calendar event
    async fn create_event(&self, summary: &str, start_time: &str, duration_minutes: u64) -> Result<String>;
}

/// Clipboard provider for reading clipboard contents
#[async_trait]
pub trait ClipboardProvider: Send + Sync {
    /// Get the current clipboard text content
    async fn get_clipboard(&self) -> Result<String>;
}

/// Application launcher
#[async_trait]
pub trait AppLauncher: Send + Sync {
    /// Open an application by name
    async fn open_app(&self, app_name: &str) -> Result<String>;
}

/// UI automation for accessibility
#[async_trait]
pub trait UiAutomation: Send + Sync {
    /// Read information about the focused application and window
    async fn read_screen(&self) -> Result<String>;
    /// Click a UI element by name and type
    async fn click_element(&self, element_name: &str, element_type: &str) -> Result<String>;
    /// Type text using keyboard simulation
    async fn type_text(&self, text: &str) -> Result<String>;
}

/// Create platform providers for the current OS
pub fn create_email_provider() -> Box<dyn EmailProvider> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacOsEmailProvider) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsEmailProvider) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { panic!("Email provider not available on this platform") }
}

pub fn create_calendar_provider() -> Box<dyn CalendarProvider> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacOsCalendarProvider) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsCalendarProvider) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { panic!("Calendar provider not available on this platform") }
}

pub fn create_clipboard_provider() -> Box<dyn ClipboardProvider> {
    Box::new(CrossPlatformClipboard)
}

pub fn create_app_launcher() -> Box<dyn AppLauncher> {
    Box::new(CrossPlatformAppLauncher)
}

pub fn create_ui_automation() -> Box<dyn UiAutomation> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacOsUiAutomation) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsUiAutomation) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { panic!("UI automation not available on this platform") }
}

/// Cross-platform clipboard using `arboard` crate
pub struct CrossPlatformClipboard;

#[async_trait]
impl ClipboardProvider for CrossPlatformClipboard {
    async fn get_clipboard(&self) -> Result<String> {
        // arboard is not async, run in blocking task
        tokio::task::spawn_blocking(|| {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
            clipboard.get_text()
                .map_err(|e| anyhow::anyhow!("Failed to read clipboard: {}", e))
        })
        .await?
    }
}

/// Cross-platform app launcher using `open` crate
pub struct CrossPlatformAppLauncher;

#[async_trait]
impl AppLauncher for CrossPlatformAppLauncher {
    async fn open_app(&self, app_name: &str) -> Result<String> {
        open::that(app_name)
            .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", app_name, e))?;
        Ok(format!("Successfully opened {}", app_name))
    }
}
```

**Step 3: Register platform module in crate root**

Find crate root (likely `crates/meepo-core/src/lib.rs`) and add:
```rust
pub mod platform;
```

**Step 4: Verify it compiles**

Run: `cargo check -p meepo-core`
Expected: Compiles (macOS impls don't exist yet, but cfg gates mean they won't be required until macos.rs is created).

**Step 5: Commit**

```bash
git add crates/meepo-core/src/platform/
git commit -m "feat: add platform abstraction traits for cross-platform support"
```

---

### Task 3: Create macOS platform implementations

**Files:**
- Create: `crates/meepo-core/src/platform/macos.rs`

This file extracts the AppleScript logic from `tools/macos.rs` and `tools/accessibility.rs` into trait implementations. The existing tool files will later be refactored to use these impls.

**Step 1: Create `platform/macos.rs`**

```rust
//! macOS platform implementations using AppleScript

use async_trait::async_trait;
use anyhow::{Result, Context};
use tokio::process::Command;
use tracing::{debug, warn};

use super::{EmailProvider, CalendarProvider, UiAutomation};

/// Sanitize a string for safe use in AppleScript
pub fn sanitize_applescript_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', " ")
        .chars()
        .filter(|&c| c >= ' ' || c == '\t')
        .collect()
}

/// Helper to run an AppleScript with timeout
async fn run_applescript(script: &str) -> Result<String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
    )
    .await
    .map_err(|_| anyhow::anyhow!("AppleScript execution timed out after 30 seconds"))?
    .context("Failed to execute osascript")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!("AppleScript failed: {}", error))
    }
}

// ── Email ──

pub struct MacOsEmailProvider;

#[async_trait]
impl EmailProvider for MacOsEmailProvider {
    async fn read_emails(&self, limit: u64, mailbox: &str, search: Option<&str>) -> Result<String> {
        let safe_mailbox = match mailbox.to_lowercase().as_str() {
            "inbox" => "inbox",
            "sent" => "sent mailbox",
            "drafts" => "drafts",
            "trash" => "trash",
            _ => "inbox",
        };

        let filter_clause = if let Some(term) = search {
            let safe_term = sanitize_applescript_string(term);
            format!(r#" whose (subject contains "{}" or sender contains "{}")"#, safe_term, safe_term)
        } else {
            String::new()
        };

        let script = format!(r#"
tell application "Mail"
    try
        set msgs to (messages 1 thru {} of {}{})
        set output to ""
        repeat with m in msgs
            set msgBody to content of m
            if length of msgBody > 500 then
                set msgBody to text 1 thru 500 of msgBody
            end if
            set output to output & "From: " & (sender of m) & "\n"
            set output to output & "Subject: " & (subject of m) & "\n"
            set output to output & "Date: " & (date received of m as string) & "\n"
            set output to output & "Preview: " & msgBody & "\n"
            set output to output & "---\n"
        end repeat
        return output
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, limit, safe_mailbox, filter_clause);

        debug!("Reading {} emails from Mail.app ({})", limit, mailbox);
        run_applescript(&script).await
    }

    async fn send_email(&self, to: &str, subject: &str, body: &str, cc: Option<&str>, in_reply_to: Option<&str>) -> Result<String> {
        let safe_to = sanitize_applescript_string(to);
        let safe_subject = sanitize_applescript_string(subject);
        let safe_body = sanitize_applescript_string(body);

        let script = if let Some(reply_subject) = in_reply_to {
            let safe_reply_subject = sanitize_applescript_string(reply_subject);
            debug!("Replying to email with subject: {}", reply_subject);
            format!(r#"
tell application "Mail"
    try
        set targetMsgs to (every message of inbox whose subject contains "{}")
        if (count of targetMsgs) > 0 then
            set originalMsg to item 1 of targetMsgs
            set replyMsg to reply originalMsg with opening window
            set content of replyMsg to "{}"
            send replyMsg
            return "Reply sent (threaded)"
        else
            set newMessage to make new outgoing message with properties {{subject:"{}", content:"{}", visible:true}}
            tell newMessage
                make new to recipient at end of to recipients with properties {{address:"{}"}}
                send
            end tell
            return "Email sent (no original found for threading)"
        end if
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, safe_reply_subject, safe_body, safe_subject, safe_body, safe_to)
        } else {
            debug!("Sending new email to: {}", to);
            let cc_block = if let Some(cc_addr) = cc {
                let safe_cc = sanitize_applescript_string(cc_addr);
                format!(r#"
                make new cc recipient at end of cc recipients with properties {{address:"{}"}}"#, safe_cc)
            } else {
                String::new()
            };

            format!(r#"
tell application "Mail"
    try
        set newMessage to make new outgoing message with properties {{subject:"{}", content:"{}", visible:true}}
        tell newMessage
            make new to recipient at end of to recipients with properties {{address:"{}"}}{}
            send
        end tell
        return "Email sent successfully"
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, safe_subject, safe_body, safe_to, cc_block)
        };

        run_applescript(&script).await
    }
}

// ── Calendar ──

pub struct MacOsCalendarProvider;

#[async_trait]
impl CalendarProvider for MacOsCalendarProvider {
    async fn read_events(&self, days_ahead: u64) -> Result<String> {
        debug!("Reading calendar events for next {} days", days_ahead);

        let script = format!(r#"
tell application "Calendar"
    try
        set startDate to current date
        set endDate to (current date) + ({} * days)
        set theEvents to (every event of calendar "Calendar" whose start date is greater than or equal to startDate and start date is less than or equal to endDate)
        set output to ""
        repeat with evt in theEvents
            set output to output & "Event: " & (summary of evt) & "\n"
            set output to output & "Start: " & (start date of evt as string) & "\n"
            set output to output & "End: " & (end date of evt as string) & "\n"
            set output to output & "---\n"
        end repeat
        return output
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, days_ahead);

        run_applescript(&script).await
    }

    async fn create_event(&self, summary: &str, start_time: &str, duration_minutes: u64) -> Result<String> {
        debug!("Creating calendar event: {}", summary);

        let safe_summary = sanitize_applescript_string(summary);
        let safe_start_time = sanitize_applescript_string(start_time);

        let script = format!(r#"
tell application "Calendar"
    try
        set startDate to date "{}"
        set endDate to startDate + ({} * minutes)
        tell calendar "Calendar"
            make new event with properties {{summary:"{}", start date:startDate, end date:endDate}}
        end tell
        return "Event created successfully"
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, safe_start_time, duration_minutes, safe_summary);

        run_applescript(&script).await
    }
}

// ── UI Automation ──

/// Allowlist of valid AppleScript UI element types
const VALID_ELEMENT_TYPES: &[&str] = &[
    "button", "checkbox", "radio button", "text field", "text area",
    "pop up button", "menu item", "menu button", "slider", "tab group",
    "table", "outline", "list", "scroll area", "group", "window",
    "sheet", "toolbar", "static text", "image", "link", "cell", "row",
    "column", "combo box", "incrementor", "relevance indicator",
];

pub struct MacOsUiAutomation;

#[async_trait]
impl UiAutomation for MacOsUiAutomation {
    async fn read_screen(&self) -> Result<String> {
        debug!("Reading screen information");
        let script = r#"
tell application "System Events"
    try
        set frontApp to first application process whose frontmost is true
        set appName to name of frontApp
        try
            set windowTitle to name of front window of frontApp
            return "App: " & appName & "\nWindow: " & windowTitle
        on error
            return "App: " & appName & "\nWindow: (no window)"
        end try
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#;
        run_applescript(script).await
    }

    async fn click_element(&self, element_name: &str, element_type: &str) -> Result<String> {
        if !VALID_ELEMENT_TYPES.iter().any(|&valid| valid.eq_ignore_ascii_case(element_type)) {
            return Err(anyhow::anyhow!("Invalid element type: {}", element_type));
        }

        debug!("Clicking {} element: {}", element_type, element_name);
        let safe_element_name = sanitize_applescript_string(element_name);

        let script = format!(r#"
tell application "System Events"
    try
        set frontApp to first application process whose frontmost is true
        tell frontApp
            click {} "{}"
        end tell
        return "Clicked successfully"
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, element_type, safe_element_name);

        run_applescript(&script).await
    }

    async fn type_text(&self, text: &str) -> Result<String> {
        debug!("Typing text ({} chars)", text.len());
        let safe_text = sanitize_applescript_string(text);

        let script = format!(r#"
tell application "System Events"
    try
        keystroke "{}"
        return "Text typed successfully"
    on error errMsg
        return "Error: " & errMsg
    end try
end tell
"#, safe_text.replace('\n', "\" & return & \""));

        run_applescript(&script).await
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo check -p meepo-core`

**Step 3: Commit**

```bash
git add crates/meepo-core/src/platform/macos.rs
git commit -m "feat: extract macOS AppleScript logic into platform trait impls"
```

---

### Task 4: Create Windows platform implementations

**Files:**
- Create: `crates/meepo-core/src/platform/windows.rs`

**Step 1: Create `platform/windows.rs`**

This file implements the platform traits using PowerShell for Outlook COM automation and Windows UI Automation concepts. It will only compile on Windows (`#[cfg(target_os = "windows")]` in `mod.rs`).

```rust
//! Windows platform implementations using PowerShell and COM automation

use async_trait::async_trait;
use anyhow::{Result, Context};
use tokio::process::Command;
use tracing::{debug, warn};

use super::{EmailProvider, CalendarProvider, UiAutomation};

/// Sanitize a string for safe use in PowerShell (prevent injection)
pub fn sanitize_powershell_string(input: &str) -> String {
    input
        .replace('`', "``")        // Escape backtick (PS escape char)
        .replace('$', "`$")        // Escape variable expansion
        .replace('"', "`\"")       // Escape double quotes
        .replace('\n', "`n")       // Newlines
        .replace('\r', "`r")       // Carriage returns
        .chars()
        .filter(|&c| c >= ' ' || c == '\t')
        .collect()
}

/// Helper to run a PowerShell script with timeout
async fn run_powershell(script: &str) -> Result<String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
    )
    .await
    .map_err(|_| anyhow::anyhow!("PowerShell execution timed out after 30 seconds"))?
    .context("Failed to execute PowerShell")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!("PowerShell failed: {}", error))
    }
}

// ── Email (Outlook COM) ──

pub struct WindowsEmailProvider;

#[async_trait]
impl EmailProvider for WindowsEmailProvider {
    async fn read_emails(&self, limit: u64, mailbox: &str, search: Option<&str>) -> Result<String> {
        debug!("Reading {} emails from Outlook ({})", limit, mailbox);

        let folder = match mailbox.to_lowercase().as_str() {
            "inbox" => "6",       // olFolderInbox
            "sent" => "5",        // olFolderSentMail
            "drafts" => "16",     // olFolderDrafts
            "trash" => "3",       // olFolderDeletedItems
            _ => "6",
        };

        let filter_clause = if let Some(term) = search {
            let safe_term = sanitize_powershell_string(term);
            format!(r#"$items = $items | Where-Object {{ $_.Subject -like "*{}*" -or $_.SenderName -like "*{}*" }}"#, safe_term, safe_term)
        } else {
            String::new()
        };

        let script = format!(r#"
try {{
    $outlook = New-Object -ComObject Outlook.Application
    $namespace = $outlook.GetNamespace("MAPI")
    $folder = $namespace.GetDefaultFolder({folder})
    $items = $folder.Items
    $items.Sort("[ReceivedTime]", $true)
    {filter_clause}
    $count = [Math]::Min($items.Count, {limit})
    $output = ""
    for ($i = 1; $i -le $count; $i++) {{
        $msg = $items.Item($i)
        $body = $msg.Body
        if ($body.Length -gt 500) {{ $body = $body.Substring(0, 500) }}
        $output += "From: $($msg.SenderName) <$($msg.SenderEmailAddress)>`n"
        $output += "Subject: $($msg.Subject)`n"
        $output += "Date: $($msg.ReceivedTime)`n"
        $output += "Preview: $body`n"
        $output += "---`n"
    }}
    Write-Output $output
}} catch {{
    Write-Error "Error reading emails: $_"
}}
"#);

        run_powershell(&script).await
    }

    async fn send_email(&self, to: &str, subject: &str, body: &str, cc: Option<&str>, in_reply_to: Option<&str>) -> Result<String> {
        let safe_to = sanitize_powershell_string(to);
        let safe_subject = sanitize_powershell_string(subject);
        let safe_body = sanitize_powershell_string(body);

        let script = if let Some(reply_subject) = in_reply_to {
            let safe_reply = sanitize_powershell_string(reply_subject);
            debug!("Replying to email with subject: {}", reply_subject);
            format!(r#"
try {{
    $outlook = New-Object -ComObject Outlook.Application
    $namespace = $outlook.GetNamespace("MAPI")
    $inbox = $namespace.GetDefaultFolder(6)
    $items = $inbox.Items
    $found = $items.Find("[Subject] = '{safe_reply}'")
    if ($found -ne $null) {{
        $reply = $found.Reply()
        $reply.Body = "{safe_body}" + "`n`n" + $reply.Body
        $reply.Send()
        Write-Output "Reply sent (threaded)"
    }} else {{
        $mail = $outlook.CreateItem(0)
        $mail.To = "{safe_to}"
        $mail.Subject = "{safe_subject}"
        $mail.Body = "{safe_body}"
        $mail.Send()
        Write-Output "Email sent (no original found for threading)"
    }}
}} catch {{
    Write-Error "Error sending email: $_"
}}
"#)
        } else {
            debug!("Sending new email to: {}", to);
            let cc_line = if let Some(cc_addr) = cc {
                let safe_cc = sanitize_powershell_string(cc_addr);
                format!(r#"    $mail.CC = "{safe_cc}""#)
            } else {
                String::new()
            };

            format!(r#"
try {{
    $outlook = New-Object -ComObject Outlook.Application
    $mail = $outlook.CreateItem(0)
    $mail.To = "{safe_to}"
    $mail.Subject = "{safe_subject}"
    $mail.Body = "{safe_body}"
{cc_line}
    $mail.Send()
    Write-Output "Email sent successfully"
}} catch {{
    Write-Error "Error sending email: $_"
}}
"#)
        };

        run_powershell(&script).await
    }
}

// ── Calendar (Outlook COM) ──

pub struct WindowsCalendarProvider;

#[async_trait]
impl CalendarProvider for WindowsCalendarProvider {
    async fn read_events(&self, days_ahead: u64) -> Result<String> {
        debug!("Reading calendar events for next {} days from Outlook", days_ahead);

        let script = format!(r#"
try {{
    $outlook = New-Object -ComObject Outlook.Application
    $namespace = $outlook.GetNamespace("MAPI")
    $calendar = $namespace.GetDefaultFolder(9)
    $items = $calendar.Items
    $items.IncludeRecurrences = $true
    $items.Sort("[Start]")
    $start = (Get-Date).ToString("g")
    $end = (Get-Date).AddDays({days_ahead}).ToString("g")
    $restrict = "[Start] >= '$start' AND [Start] <= '$end'"
    $filtered = $items.Restrict($restrict)
    $output = ""
    foreach ($evt in $filtered) {{
        $output += "Event: $($evt.Subject)`n"
        $output += "Start: $($evt.Start)`n"
        $output += "End: $($evt.End)`n"
        $output += "---`n"
    }}
    Write-Output $output
}} catch {{
    Write-Error "Error reading calendar: $_"
}}
"#);

        run_powershell(&script).await
    }

    async fn create_event(&self, summary: &str, start_time: &str, duration_minutes: u64) -> Result<String> {
        debug!("Creating calendar event: {}", summary);
        let safe_summary = sanitize_powershell_string(summary);
        let safe_start = sanitize_powershell_string(start_time);

        let script = format!(r#"
try {{
    $outlook = New-Object -ComObject Outlook.Application
    $appt = $outlook.CreateItem(1)
    $appt.Subject = "{safe_summary}"
    $appt.Start = [DateTime]::Parse("{safe_start}")
    $appt.Duration = {duration_minutes}
    $appt.Save()
    Write-Output "Event created successfully"
}} catch {{
    Write-Error "Error creating event: $_"
}}
"#);

        run_powershell(&script).await
    }
}

// ── UI Automation (PowerShell + System.Windows.Automation) ──

pub struct WindowsUiAutomation;

#[async_trait]
impl UiAutomation for WindowsUiAutomation {
    async fn read_screen(&self) -> Result<String> {
        debug!("Reading screen information via UI Automation");

        let script = r#"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
try {
    $root = [System.Windows.Automation.AutomationElement]::FocusedElement
    $process = Get-Process -Id $root.Current.ProcessId -ErrorAction SilentlyContinue
    $appName = if ($process) { $process.MainWindowTitle } else { $root.Current.Name }
    $processName = if ($process) { $process.ProcessName } else { "unknown" }
    Write-Output "App: $processName`nWindow: $appName"
} catch {
    Write-Error "Error reading screen: $_"
}
"#;

        run_powershell(script).await
    }

    async fn click_element(&self, element_name: &str, element_type: &str) -> Result<String> {
        debug!("Clicking {} element: {}", element_type, element_name);
        let safe_name = sanitize_powershell_string(element_name);

        let script = format!(r#"
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
try {{
    $root = [System.Windows.Automation.AutomationElement]::FocusedElement
    $walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
    # Search for element by name in the focused window
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty, "{safe_name}")
    $element = $root.FindFirst([System.Windows.Automation.TreeScope]::Subtree, $condition)
    if ($element -ne $null) {{
        $invokePattern = $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invokePattern.Invoke()
        Write-Output "Clicked successfully"
    }} else {{
        Write-Error "Element '{safe_name}' not found"
    }}
}} catch {{
    Write-Error "Error clicking element: $_"
}}
"#);

        run_powershell(&script).await
    }

    async fn type_text(&self, text: &str) -> Result<String> {
        debug!("Typing text ({} chars)", text.len());
        let safe_text = sanitize_powershell_string(text);

        let script = format!(r#"
Add-Type -AssemblyName System.Windows.Forms
try {{
    [System.Windows.Forms.SendKeys]::SendWait("{safe_text}")
    Write-Output "Text typed successfully"
}} catch {{
    Write-Error "Error typing text: $_"
}}
"#);

        run_powershell(&script).await
    }
}
```

**Step 2: Verify it compiles on macOS (should be ignored due to cfg)**

Run: `cargo check -p meepo-core`
Expected: Compiles — windows.rs is gated behind `#[cfg(target_os = "windows")]`.

**Step 3: Commit**

```bash
git add crates/meepo-core/src/platform/windows.rs
git commit -m "feat: add Windows platform implementations using PowerShell/COM"
```

---

### Task 5: Refactor tool structs to use platform abstractions

**Files:**
- Modify: `crates/meepo-core/src/tools/macos.rs` — refactor to delegate to platform traits
- Modify: `crates/meepo-core/src/tools/accessibility.rs` — refactor to delegate to platform traits

**Step 1: Rewrite `tools/macos.rs` to use platform providers**

The tools now hold a `Box<dyn Trait>` and delegate to it. The `sanitize_applescript_string` function stays exported for backward compat but delegates to the platform module.

```rust
//! Platform-aware email, calendar, clipboard, and app tools

use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use tracing::debug;

use super::{ToolHandler, json_schema};
use crate::platform;

// Re-export for backward compatibility (accessibility module uses this)
#[cfg(target_os = "macos")]
pub use crate::platform::macos::sanitize_applescript_string;
#[cfg(not(target_os = "macos"))]
pub(crate) fn sanitize_applescript_string(_input: &str) -> String {
    // Not applicable on non-macOS platforms
    _input.to_string()
}

/// Read emails from system email client
pub struct ReadEmailsTool {
    provider: Box<dyn platform::EmailProvider>,
}

impl ReadEmailsTool {
    pub fn new() -> Self {
        Self { provider: platform::create_email_provider() }
    }
}

impl Default for ReadEmailsTool {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ToolHandler for ReadEmailsTool {
    fn name(&self) -> &str { "read_emails" }
    fn description(&self) -> &str {
        "Read recent emails from the system email client. Returns sender, subject, date, and preview."
    }
    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "limit": { "type": "number", "description": "Number of emails to retrieve (default: 10, max: 50)" },
                "mailbox": { "type": "string", "description": "Mailbox to read from (default: 'inbox'). Options: inbox, sent, drafts, trash" },
                "search": { "type": "string", "description": "Optional search term to filter by subject or sender" }
            }),
            vec![],
        )
    }
    async fn execute(&self, input: Value) -> Result<String> {
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(10).min(50);
        let mailbox = input.get("mailbox").and_then(|v| v.as_str()).unwrap_or("inbox");
        let search = input.get("search").and_then(|v| v.as_str());
        self.provider.read_emails(limit, mailbox, search).await
    }
}

// [Similar pattern for ReadCalendarTool, SendEmailTool, CreateEventTool, OpenAppTool, GetClipboardTool]
// Each holds a Box<dyn Trait> and delegates execute() to the provider.
```

The full file should contain all 6 tool structs refactored to this pattern. See the implementation for the exact code.

**Step 2: Rewrite `tools/accessibility.rs` similarly**

```rust
//! Platform-aware UI automation tools

use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;

use super::{ToolHandler, json_schema};
use crate::platform;

pub struct ReadScreenTool {
    automation: Box<dyn platform::UiAutomation>,
}

impl ReadScreenTool {
    pub fn new() -> Self {
        Self { automation: platform::create_ui_automation() }
    }
}

impl Default for ReadScreenTool {
    fn default() -> Self { Self::new() }
}

// ... [ClickElementTool, TypeTextTool similarly]
```

**Step 3: Update main.rs tool registration to use `::new()` constructors**

Change:
```rust
registry.register(Arc::new(meepo_core::tools::macos::ReadEmailsTool));
```
To:
```rust
registry.register(Arc::new(meepo_core::tools::macos::ReadEmailsTool::new()));
```

For all 9 platform tools.

**Step 4: Verify it compiles**

Run: `cargo check`

**Step 5: Run tests**

Run: `cargo test`
Expected: All existing tests pass (schema tests, sanitization tests, missing params tests).

**Step 6: Commit**

```bash
git add crates/meepo-core/src/tools/macos.rs crates/meepo-core/src/tools/accessibility.rs crates/meepo-cli/src/main.rs
git commit -m "refactor: tools delegate to platform abstraction layer"
```

---

### Task 6: Add `#[cfg]` guards to macOS-only channels

**Files:**
- Modify: `crates/meepo-channels/src/lib.rs` — gate iMessage and Email behind `#[cfg(target_os = "macos")]`
- Modify: `crates/meepo-cli/src/main.rs` — gate channel registration behind `#[cfg]`

**Step 1: Gate iMessage and Email in channels lib.rs**

```rust
pub mod bus;
pub mod discord;
#[cfg(target_os = "macos")]
pub mod email;
#[cfg(target_os = "macos")]
pub mod imessage;
pub mod slack;

pub use bus::{MessageBus, MessageChannel};
pub use discord::DiscordChannel;
#[cfg(target_os = "macos")]
pub use email::EmailChannel;
#[cfg(target_os = "macos")]
pub use imessage::IMessageChannel;
pub use slack::SlackChannel;
```

**Step 2: Gate channel registration in main.rs**

Wrap the iMessage and Email channel registration blocks:

```rust
#[cfg(target_os = "macos")]
{
    if cfg.channels.imessage.enabled {
        // ... existing iMessage registration code ...
    }
}

#[cfg(target_os = "macos")]
{
    if cfg.channels.email.enabled {
        // ... existing Email registration code ...
    }
}
```

**Step 3: Gate the config structs for iMessage/Email**

In `crates/meepo-cli/src/config.rs`, make the iMessage and Email config fields optional or gated.
Actually, the config should always parse (cross-platform config file), but the channels just won't be registered on Windows. So the config structs stay, but we add a warning log on Windows if they're enabled.

**Step 4: Verify it compiles**

Run: `cargo check`

**Step 5: Commit**

```bash
git add crates/meepo-channels/src/lib.rs crates/meepo-cli/src/main.rs
git commit -m "feat: gate iMessage and Email channels behind macOS cfg"
```

---

### Task 7: Gate email/calendar watchers behind macOS cfg in scheduler

**Files:**
- Modify: `crates/meepo-scheduler/src/runner.rs` — gate AppleScript watcher polling behind `#[cfg(target_os = "macos")]`

**Step 1: Add cfg guards to poll_watcher for EmailWatch and CalendarWatch**

The `WatcherKind::EmailWatch` and `WatcherKind::CalendarWatch` match arms in `poll_watcher()` use `osascript`. On non-macOS, these should return an error explaining the watcher type isn't available.

```rust
WatcherKind::EmailWatch { .. } => {
    #[cfg(not(target_os = "macos"))]
    {
        warn!("Email watcher is only available on macOS");
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        // ... existing AppleScript email polling code ...
    }
}
```

Same for `CalendarWatch`.

**Step 2: Verify it compiles**

Run: `cargo check`

**Step 3: Commit**

```bash
git add crates/meepo-scheduler/src/runner.rs
git commit -m "feat: gate AppleScript watchers behind macOS cfg"
```

---

### Task 8: Update CLI about string and add platform detection

**Files:**
- Modify: `crates/meepo-cli/src/main.rs` — change "local AI agent for macOS" to "local AI agent"
- Modify: `crates/meepo-cli/src/main.rs` — add platform info to startup logs

**Step 1: Update CLI about string**

Line 17: Change `"Meepo — a local AI agent for macOS"` to `"Meepo — a local AI agent"`

**Step 2: Add platform info at startup**

After `info!("Starting Meepo daemon...");` add:
```rust
info!("Platform: {} ({})", std::env::consts::OS, std::env::consts::ARCH);
```

**Step 3: Verify, commit**

```bash
cargo check
git add crates/meepo-cli/src/main.rs
git commit -m "chore: update CLI branding for cross-platform support"
```

---

### Task 9: Create cross-platform setup script (PowerShell for Windows)

**Files:**
- Create: `scripts/setup.ps1` — Windows equivalent of `setup.sh`

**Step 1: Create `setup.ps1`**

PowerShell equivalent of setup.sh. Same 7-step flow but using:
- PowerShell color output instead of ANSI escapes
- `Start-Process` instead of `open`
- `Get-Clipboard` instead of `pbpaste`
- No iMessage section (not available on Windows)
- Windows-specific Outlook setup guidance instead

The script should detect Rust, build the binary, initialize config, walk through API keys, enable Discord/Slack, and verify.

**Step 2: Create `scripts/install.ps1`** — Windows equivalent of install.sh

Uses Windows Task Scheduler (`schtasks.exe`) to register Meepo as a startup task:
```powershell
schtasks /Create /SC ONLOGON /TN "Meepo" /TR "$binaryPath start" /RL HIGHEST
```

**Step 3: Create `scripts/uninstall.ps1`** — Windows equivalent

```powershell
schtasks /Delete /TN "Meepo" /F
```

**Step 4: Commit**

```bash
git add scripts/setup.ps1 scripts/install.ps1 scripts/uninstall.ps1
git commit -m "feat: add Windows PowerShell setup/install/uninstall scripts"
```

---

### Task 10: Update default.toml config comments for cross-platform

**Files:**
- Modify: `config/default.toml` — update comments to mention Windows support

**Step 1: Update iMessage section comment**

Add note that iMessage is macOS-only.

**Step 2: Update Email section comment**

Note that Email channel uses Mail.app (macOS) or Outlook (Windows).

**Step 3: Add platform detection note at top**

```toml
# Platform support: macOS and Windows.
# Some channels and tools are platform-specific — see comments below.
```

**Step 4: Commit**

```bash
git add config/default.toml
git commit -m "docs: update config comments for cross-platform support"
```

---

### Task 11: Update run.sh for cross-platform and create run.ps1

**Files:**
- Modify: `scripts/run.sh` — add uname check, skip macOS-specific parts on Linux
- Create: `scripts/run.ps1` — Windows equivalent

**Step 1: run.ps1 for Windows**

```powershell
$ProjectDir = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Binary = Join-Path $ProjectDir "target\release\meepo.exe"
# ... build if needed, check API key, exec binary ...
```

**Step 2: Commit**

```bash
git add scripts/run.sh scripts/run.ps1
git commit -m "feat: add Windows run.ps1 and improve run.sh portability"
```

---

### Task 12: Add tests for platform abstraction

**Files:**
- Add tests to: `crates/meepo-core/src/platform/mod.rs`

**Step 1: Add cross-platform tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clipboard_provider_creates() {
        // Just verify creation doesn't panic
        let _provider = create_clipboard_provider();
    }

    #[tokio::test]
    async fn test_app_launcher_creates() {
        let _launcher = create_app_launcher();
    }

    #[test]
    fn test_platform_detection() {
        // Verify that platform providers can be created on current OS
        let _email = create_email_provider();
        let _calendar = create_calendar_provider();
        let _ui = create_ui_automation();
    }
}
```

**Step 2: Verify all tests pass**

Run: `cargo test`

**Step 3: Commit**

```bash
git add crates/meepo-core/src/platform/mod.rs
git commit -m "test: add platform abstraction tests"
```

---

### Task 13: Final integration test and README update

**Files:**
- Verify full build: `cargo build --release`
- Verify all tests: `cargo test`
- Modify: `README.md` — add Windows support section

**Step 1: Full build**

Run: `cargo build --release`

**Step 2: Full test suite**

Run: `cargo test`

**Step 3: Update README**

Add a "Platform Support" section:
- macOS: Full support (all channels and tools)
- Windows: Full support except iMessage (uses Outlook for email/calendar, PowerShell for system automation)
- Update setup instructions to mention both `setup.sh` (macOS/Linux) and `setup.ps1` (Windows)

**Step 4: Commit**

```bash
git add README.md
git commit -m "docs: add Windows platform support to README"
```

---

## Summary

| Task | Description | Key Files |
|------|-------------|-----------|
| 1 | Add cross-platform deps | `Cargo.toml`, `meepo-core/Cargo.toml` |
| 2 | Platform abstraction traits | `platform/mod.rs` |
| 3 | macOS implementations | `platform/macos.rs` |
| 4 | Windows implementations | `platform/windows.rs` |
| 5 | Refactor tools to use platform | `tools/macos.rs`, `tools/accessibility.rs`, `main.rs` |
| 6 | Gate macOS-only channels | `channels/lib.rs`, `main.rs` |
| 7 | Gate macOS-only watchers | `scheduler/runner.rs` |
| 8 | Update CLI branding | `main.rs` |
| 9 | Windows scripts | `setup.ps1`, `install.ps1`, `uninstall.ps1` |
| 10 | Update config docs | `default.toml` |
| 11 | Cross-platform run script | `run.ps1`, `run.sh` |
| 12 | Platform tests | `platform/mod.rs` |
| 13 | Integration test + README | `README.md` |
