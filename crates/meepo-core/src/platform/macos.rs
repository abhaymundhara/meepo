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

/// Run an AppleScript with 30 second timeout
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
        warn!("AppleScript failed: {}", error);
        Err(anyhow::anyhow!("AppleScript failed: {}", error))
    }
}

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
        debug!("Reading {} emails from Mail.app ({})", limit, mailbox);
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

/// Allowlist of valid UI element types for macOS accessibility
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_applescript_string() {
        assert_eq!(sanitize_applescript_string("test\\path"), "test\\\\path");
        assert_eq!(sanitize_applescript_string("test\"quote"), "test\\\"quote");
        assert_eq!(sanitize_applescript_string("test\nline"), "test line");
        assert_eq!(sanitize_applescript_string("test\rline"), "test line");
        let with_control = "test\x01\x02\x03text";
        assert_eq!(sanitize_applescript_string(with_control), "testtext");
    }

    #[test]
    fn test_sanitize_prevents_injection() {
        let attack = "test\"; do shell script \"rm -rf /\" --\"";
        let safe = sanitize_applescript_string(attack);
        assert!(!safe.contains('\n'));
        assert!(safe.contains("\\\""));
    }
}
