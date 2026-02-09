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
    async fn read_emails(&self, limit: u64, mailbox: &str, search: Option<&str>) -> Result<String>;
    async fn send_email(&self, to: &str, subject: &str, body: &str, cc: Option<&str>, in_reply_to: Option<&str>) -> Result<String>;
}

/// Calendar provider for reading and creating events
#[async_trait]
pub trait CalendarProvider: Send + Sync {
    async fn read_events(&self, days_ahead: u64) -> Result<String>;
    async fn create_event(&self, summary: &str, start_time: &str, duration_minutes: u64) -> Result<String>;
}

/// Clipboard provider for reading clipboard contents
#[async_trait]
pub trait ClipboardProvider: Send + Sync {
    async fn get_clipboard(&self) -> Result<String>;
}

/// Application launcher
#[async_trait]
pub trait AppLauncher: Send + Sync {
    async fn open_app(&self, app_name: &str) -> Result<String>;
}

/// UI automation for accessibility
#[async_trait]
pub trait UiAutomation: Send + Sync {
    async fn read_screen(&self) -> Result<String>;
    async fn click_element(&self, element_name: &str, element_type: &str) -> Result<String>;
    async fn type_text(&self, text: &str) -> Result<String>;
}

/// Create platform email provider
pub fn create_email_provider() -> Box<dyn EmailProvider> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacOsEmailProvider) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsEmailProvider) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { panic!("Email provider not available on this platform") }
}

/// Create platform calendar provider
pub fn create_calendar_provider() -> Box<dyn CalendarProvider> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacOsCalendarProvider) }
    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsCalendarProvider) }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { panic!("Calendar provider not available on this platform") }
}

/// Create cross-platform clipboard provider
pub fn create_clipboard_provider() -> Box<dyn ClipboardProvider> {
    Box::new(CrossPlatformClipboard)
}

/// Create cross-platform app launcher
pub fn create_app_launcher() -> Box<dyn AppLauncher> {
    Box::new(CrossPlatformAppLauncher)
}

/// Create platform UI automation provider
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
        let name = app_name.to_string();
        tokio::task::spawn_blocking(move || {
            open::that(&name)
                .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", name, e))?;
            Ok(format!("Successfully opened {}", name))
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_provider_creates() {
        let _provider = create_clipboard_provider();
    }

    #[test]
    fn test_app_launcher_creates() {
        let _launcher = create_app_launcher();
    }

    #[test]
    fn test_platform_providers_create() {
        let _email = create_email_provider();
        let _calendar = create_calendar_provider();
        let _ui = create_ui_automation();
    }
}
