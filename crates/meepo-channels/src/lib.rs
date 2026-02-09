//! Channel adapters and message bus for meepo
//!
//! This crate provides the message routing infrastructure and channel-specific
//! adapters for Discord, iMessage, and Slack.

pub mod bus;
pub mod discord;
#[cfg(target_os = "macos")]
pub mod email;
#[cfg(target_os = "macos")]
pub mod imessage;
pub mod rate_limit;
pub mod slack;

// Re-export main types
pub use bus::{MessageBus, MessageChannel};
pub use discord::DiscordChannel;
#[cfg(target_os = "macos")]
pub use email::EmailChannel;
#[cfg(target_os = "macos")]
pub use imessage::IMessageChannel;
pub use rate_limit::RateLimiter;
pub use slack::SlackChannel;
