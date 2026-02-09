//! Platform-abstracted email, calendar, and system tools
//!
//! These tools delegate to platform-specific implementations through the platform module.
//! On macOS: AppleScript-based implementations.
//! On Windows: PowerShell/COM-based implementations.

use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;
use tracing::debug;

use super::{ToolHandler, json_schema};
use crate::platform::{EmailProvider, CalendarProvider, ClipboardProvider, AppLauncher};

/// Read emails from the default email application
pub struct ReadEmailsTool {
    provider: Box<dyn EmailProvider>,
}

impl ReadEmailsTool {
    pub fn new() -> Self {
        Self {
            provider: crate::platform::create_email_provider(),
        }
    }
}

#[async_trait]
impl ToolHandler for ReadEmailsTool {
    fn name(&self) -> &str {
        "read_emails"
    }

    fn description(&self) -> &str {
        "Read recent emails. Returns sender, subject, date, and preview for the latest emails."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "limit": {
                    "type": "number",
                    "description": "Number of emails to retrieve (default: 10, max: 50)"
                },
                "mailbox": {
                    "type": "string",
                    "description": "Mailbox to read from (default: 'inbox'). Options: inbox, sent, drafts, trash"
                },
                "search": {
                    "type": "string",
                    "description": "Optional search term to filter by subject or sender"
                }
            }),
            vec![],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let limit = input.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .min(50);
        let mailbox = input.get("mailbox")
            .and_then(|v| v.as_str())
            .unwrap_or("inbox");
        let search = input.get("search")
            .and_then(|v| v.as_str());

        debug!("Reading {} emails from {}", limit, mailbox);
        self.provider.read_emails(limit, mailbox, search).await
    }
}

/// Read calendar events from the default calendar application
pub struct ReadCalendarTool {
    provider: Box<dyn CalendarProvider>,
}

impl ReadCalendarTool {
    pub fn new() -> Self {
        Self {
            provider: crate::platform::create_calendar_provider(),
        }
    }
}

#[async_trait]
impl ToolHandler for ReadCalendarTool {
    fn name(&self) -> &str {
        "read_calendar"
    }

    fn description(&self) -> &str {
        "Read upcoming calendar events. Returns today's and upcoming events."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "days_ahead": {
                    "type": "number",
                    "description": "Number of days ahead to look (default: 1)"
                }
            }),
            vec![],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let days_ahead = input.get("days_ahead")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        debug!("Reading calendar events for next {} days", days_ahead);
        self.provider.read_events(days_ahead).await
    }
}

/// Send email via the default email application
pub struct SendEmailTool {
    provider: Box<dyn EmailProvider>,
}

impl SendEmailTool {
    pub fn new() -> Self {
        Self {
            provider: crate::platform::create_email_provider(),
        }
    }
}

#[async_trait]
impl ToolHandler for SendEmailTool {
    fn name(&self) -> &str {
        "send_email"
    }

    fn description(&self) -> &str {
        "Send an email. Composes and sends a message to the specified recipient."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "to": {
                    "type": "string",
                    "description": "Recipient email address"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject"
                },
                "body": {
                    "type": "string",
                    "description": "Email body content"
                },
                "cc": {
                    "type": "string",
                    "description": "Optional CC recipient email address"
                },
                "in_reply_to": {
                    "type": "string",
                    "description": "Optional subject line of email to reply to (enables threading)"
                }
            }),
            vec!["to", "subject", "body"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let to = input.get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'to' parameter"))?;
        let subject = input.get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'subject' parameter"))?;
        let body = input.get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'body' parameter"))?;
        let cc = input.get("cc").and_then(|v| v.as_str());
        let in_reply_to = input.get("in_reply_to").and_then(|v| v.as_str());

        // Input validation: body length limit
        if body.len() > 50_000 {
            return Err(anyhow::anyhow!("Email body too long ({} chars, max 50,000)", body.len()));
        }

        debug!("Sending email to: {}", to);
        self.provider.send_email(to, subject, body, cc, in_reply_to).await
    }
}

/// Create a calendar event in the default calendar application
pub struct CreateEventTool {
    provider: Box<dyn CalendarProvider>,
}

impl CreateEventTool {
    pub fn new() -> Self {
        Self {
            provider: crate::platform::create_calendar_provider(),
        }
    }
}

#[async_trait]
impl ToolHandler for CreateEventTool {
    fn name(&self) -> &str {
        "create_calendar_event"
    }

    fn description(&self) -> &str {
        "Create a new calendar event."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "summary": {
                    "type": "string",
                    "description": "Event title/summary"
                },
                "start_time": {
                    "type": "string",
                    "description": "Start time in ISO8601 format or natural language"
                },
                "duration_minutes": {
                    "type": "number",
                    "description": "Duration in minutes (default: 60)"
                }
            }),
            vec!["summary", "start_time"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let summary = input.get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'summary' parameter"))?;
        let start_time = input.get("start_time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'start_time' parameter"))?;
        let duration = input.get("duration_minutes")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);

        debug!("Creating calendar event: {}", summary);
        self.provider.create_event(summary, start_time, duration).await
    }
}

/// Open an application by name
pub struct OpenAppTool {
    launcher: Box<dyn AppLauncher>,
}

impl OpenAppTool {
    pub fn new() -> Self {
        Self {
            launcher: crate::platform::create_app_launcher(),
        }
    }
}

#[async_trait]
impl ToolHandler for OpenAppTool {
    fn name(&self) -> &str {
        "open_app"
    }

    fn description(&self) -> &str {
        "Open an application by name."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "app_name": {
                    "type": "string",
                    "description": "Name of the application to open (e.g., 'Safari', 'Terminal')"
                }
            }),
            vec!["app_name"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let app_name = input.get("app_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'app_name' parameter"))?;

        // Input validation: prevent path traversal — only allow app names, not paths
        if app_name.contains('/') || app_name.contains('\\') {
            return Err(anyhow::anyhow!("App name cannot contain path separators"));
        }
        if app_name.len() > 100 {
            return Err(anyhow::anyhow!("App name too long (max 100 characters)"));
        }

        debug!("Opening application: {}", app_name);
        self.launcher.open_app(app_name).await
    }
}

/// Get clipboard content
pub struct GetClipboardTool {
    provider: Box<dyn ClipboardProvider>,
}

impl GetClipboardTool {
    pub fn new() -> Self {
        Self {
            provider: crate::platform::create_clipboard_provider(),
        }
    }
}

#[async_trait]
impl ToolHandler for GetClipboardTool {
    fn name(&self) -> &str {
        "get_clipboard"
    }

    fn description(&self) -> &str {
        "Get the current content of the system clipboard."
    }

    fn input_schema(&self) -> Value {
        json_schema(serde_json::json!({}), vec![])
    }

    async fn execute(&self, _input: Value) -> Result<String> {
        debug!("Reading clipboard content");
        self.provider.get_clipboard().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolHandler;

    #[test]
    fn test_read_emails_schema() {
        let tool = ReadEmailsTool::new();
        assert_eq!(tool.name(), "read_emails");
        assert!(!tool.description().is_empty());
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_read_calendar_schema() {
        let tool = ReadCalendarTool::new();
        assert_eq!(tool.name(), "read_calendar");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_send_email_schema() {
        let tool = SendEmailTool::new();
        assert_eq!(tool.name(), "send_email");
        let schema = tool.input_schema();
        let required: Vec<String> = serde_json::from_value(
            schema.get("required").cloned().unwrap_or(serde_json::json!([]))
        ).unwrap_or_default();
        assert!(required.contains(&"to".to_string()));
        assert!(required.contains(&"subject".to_string()));
        assert!(required.contains(&"body".to_string()));
    }

    #[test]
    fn test_create_event_schema() {
        let tool = CreateEventTool::new();
        assert_eq!(tool.name(), "create_calendar_event");
        let schema = tool.input_schema();
        let required: Vec<String> = serde_json::from_value(
            schema.get("required").cloned().unwrap_or(serde_json::json!([]))
        ).unwrap_or_default();
        assert!(required.contains(&"summary".to_string()));
        assert!(required.contains(&"start_time".to_string()));
    }

    #[test]
    fn test_open_app_schema() {
        let tool = OpenAppTool::new();
        assert_eq!(tool.name(), "open_app");
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
    }

    #[test]
    fn test_get_clipboard_schema() {
        let tool = GetClipboardTool::new();
        assert_eq!(tool.name(), "get_clipboard");
    }

    #[tokio::test]
    async fn test_send_email_missing_params() {
        let tool = SendEmailTool::new();
        let result = tool.execute(serde_json::json!({
            "to": "test@test.com"
        })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_event_missing_params() {
        let tool = CreateEventTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_open_app_missing_params() {
        let tool = OpenAppTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
