//! Apple Reminders channel adapter using AppleScript polling

use crate::bus::MessageChannel;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use meepo_core::types::{ChannelType, IncomingMessage, MessageKind, OutgoingMessage};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Apple Reminders channel adapter that polls Reminders.app for new items
/// in a designated list and creates reminders from outgoing messages.
pub struct RemindersChannel {
    poll_interval: Duration,
    list_name: String,
    /// Tracks reminder IDs we've already processed to avoid duplicates
    seen_ids: Arc<Mutex<HashSet<String>>>,
}

impl RemindersChannel {
    pub fn new(poll_interval: Duration, list_name: String) -> Self {
        Self {
            poll_interval,
            list_name,
            seen_ids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Sanitize a string for safe use in AppleScript.
    fn escape_applescript(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .chars()
            .filter(|&c| c >= ' ' || c == '\t')
            .collect()
    }

    /// Poll Reminders.app for incomplete reminders in the configured list
    async fn poll_reminders(&self, tx: &mpsc::Sender<IncomingMessage>) -> Result<()> {
        let list = Self::escape_applescript(&self.list_name);

        let script = format!(
            r#"
tell application "Reminders"
    try
        if not (exists list "{list}") then
            return ""
        end if
        set output to ""
        set targetList to list "{list}"
        set incompleteReminders to (every reminder of targetList whose completed is false)
        repeat with r in incompleteReminders
            set rName to name of r
            set rId to id of r
            set rBody to ""
            try
                set rBody to body of r
            end try
            if rBody is missing value then set rBody to ""
            set output to output & "<<REM_START>>" & "\n"
            set output to output & "ID: " & rId & "\n"
            set output to output & "Name: " & rName & "\n"
            set output to output & "Body: " & rBody & "\n"
            set output to output & "<<REM_END>>" & "\n"
        end repeat
        return output
    on error errMsg
        return "ERROR: " & errMsg
    end try
end tell
"#
        );

        let output = tokio::time::timeout(
            Duration::from_secs(30),
            Command::new("osascript").arg("-e").arg(&script).output(),
        )
        .await
        .map_err(|_| anyhow!("Reminders.app polling timed out"))?
        .map_err(|e| anyhow!("Failed to run osascript: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Reminders.app poll failed: {}", stderr);
            return Ok(());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() || stdout.starts_with("ERROR:") {
            if stdout.starts_with("ERROR:") {
                warn!("Reminders.app error: {}", stdout);
            }
            return Ok(());
        }

        for block in stdout.split("<<REM_START>>") {
            let block = block.trim();
            if block.is_empty() || !block.contains("<<REM_END>>") {
                continue;
            }

            let block = block.replace("<<REM_END>>", "");
            let mut id = String::new();
            let mut name = String::new();
            let mut body = String::new();

            for line in block.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("ID: ") {
                    id = val.to_string();
                } else if let Some(val) = line.strip_prefix("Name: ") {
                    name = val.to_string();
                } else if let Some(val) = line.strip_prefix("Body: ") {
                    body = val.to_string();
                }
            }

            if id.is_empty() || name.is_empty() {
                continue;
            }

            // Skip already-seen reminders
            {
                let mut seen = self.seen_ids.lock().await;
                if seen.contains(&id) {
                    continue;
                }
                seen.insert(id.clone());
            }

            let content = if body.is_empty() {
                name.clone()
            } else {
                format!("{}\n\n{}", name, body)
            };

            let msg_id = format!("reminder_{}", id);

            let incoming = IncomingMessage {
                id: msg_id,
                sender: "Reminders.app".to_string(),
                content,
                channel: ChannelType::Reminders,
                timestamp: Utc::now(),
            };

            info!("New reminder from Reminders.app: {}", name);

            if let Err(e) = tx.send(incoming).await {
                error!("Failed to send reminder message to bus: {}", e);
            }

            // Mark the reminder as completed so it doesn't get picked up again
            let complete_script = format!(
                r#"
tell application "Reminders"
    try
        set targetList to list "{list}"
        set targetReminders to (every reminder of targetList whose id is "{id}")
        repeat with r in targetReminders
            set completed of r to true
        end repeat
    end try
end tell
"#,
                list = Self::escape_applescript(&self.list_name),
                id = Self::escape_applescript(&id),
            );

            if let Err(e) = Command::new("osascript")
                .arg("-e")
                .arg(&complete_script)
                .output()
                .await
            {
                warn!("Failed to mark reminder as completed: {}", e);
            }
        }

        Ok(())
    }

    /// Create a new reminder in Reminders.app
    async fn create_reminder(&self, name: &str, body: &str) -> Result<()> {
        let safe_list = Self::escape_applescript(&self.list_name);
        let safe_name = Self::escape_applescript(name);
        let safe_body = Self::escape_applescript(body);

        let script = format!(
            r#"
tell application "Reminders"
    try
        if not (exists list "{safe_list}") then
            make new list with properties {{name:"{safe_list}"}}
        end if
        tell list "{safe_list}"
            make new reminder with properties {{name:"{safe_name}", body:"{safe_body}"}}
        end tell
        return "OK"
    on error errMsg
        return "ERROR: " & errMsg
    end try
end tell
"#
        );

        let output = tokio::time::timeout(
            Duration::from_secs(30),
            Command::new("osascript").arg("-e").arg(&script).output(),
        )
        .await
        .map_err(|_| anyhow!("Reminders create timed out"))?
        .map_err(|e| anyhow!("Failed to run osascript: {}", e))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            if result.trim().starts_with("ERROR:") {
                return Err(anyhow!("Reminders.app error: {}", result.trim()));
            }
            info!("Reminder created: {}", safe_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow!("Failed to create reminder: {}", stderr))
        }
    }
}

#[async_trait]
impl MessageChannel for RemindersChannel {
    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) -> Result<()> {
        info!("Starting Reminders channel adapter");
        info!("Poll interval: {:?}", self.poll_interval);
        info!("Reminders list: {}", self.list_name);

        let poll_interval = self.poll_interval;
        let list_name = self.list_name.clone();
        let seen_ids = self.seen_ids.clone();

        let channel = RemindersChannel {
            poll_interval,
            list_name,
            seen_ids,
        };

        tokio::spawn(async move {
            info!("Reminders polling task started");
            let mut interval = tokio::time::interval(channel.poll_interval);

            loop {
                interval.tick().await;
                debug!("Polling Reminders.app for new reminders");

                if let Err(e) = channel.poll_reminders(&tx).await {
                    error!("Error polling Reminders.app: {}", e);
                }
            }
        });

        info!("Reminders channel adapter started");
        Ok(())
    }

    async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        // Acknowledgments are silently ignored for Reminders
        if msg.kind == MessageKind::Acknowledgment {
            debug!("Skipping Reminders acknowledgment");
            return Ok(());
        }

        // Extract a title from the first line of content, rest becomes body
        let (title, body) = match msg.content.split_once('\n') {
            Some((first, rest)) => (first.trim().to_string(), rest.trim().to_string()),
            None => (msg.content.clone(), String::new()),
        };

        self.create_reminder(&title, &body).await
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Reminders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminders_channel_creation() {
        let channel = RemindersChannel::new(Duration::from_secs(10), "Meepo".to_string());
        assert_eq!(channel.channel_type(), ChannelType::Reminders);
    }

    #[test]
    fn test_escape_applescript() {
        assert_eq!(
            RemindersChannel::escape_applescript("Hello \"world\""),
            "Hello \\\"world\\\""
        );
        assert_eq!(
            RemindersChannel::escape_applescript("line1\nline2"),
            "line1\\nline2"
        );
    }

    #[tokio::test]
    async fn test_seen_ids_dedup() {
        let channel = RemindersChannel::new(Duration::from_secs(10), "Meepo".to_string());

        {
            let mut seen = channel.seen_ids.lock().await;
            seen.insert("reminder_1".to_string());
        }

        {
            let seen = channel.seen_ids.lock().await;
            assert!(seen.contains("reminder_1"));
            assert!(!seen.contains("reminder_2"));
        }
    }
}
