//! meepo-scheduler - Reactive watchers and task scheduling
//!
//! This crate provides functionality for:
//! - Defining various types of watchers (email, calendar, GitHub, file, etc.)
//! - Persisting watchers to SQLite
//! - Running watchers as tokio tasks with event emission
//! - Scheduling one-shot and recurring tasks

pub mod watcher;
pub mod persistence;
pub mod runner;

pub use watcher::{Watcher, WatcherKind, WatcherEvent};
pub use persistence::{
    init_watcher_tables, save_watcher, get_active_watchers,
    deactivate_watcher, delete_watcher, get_watcher_by_id
};
pub use runner::{WatcherRunner, WatcherConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_serialization() {
        use chrono::Utc;

        let watcher = Watcher {
            id: "test-123".to_string(),
            kind: WatcherKind::EmailWatch {
                from: Some("test@example.com".to_string()),
                subject_contains: Some("invoice".to_string()),
                interval_secs: 300,
            },
            action: "Process incoming invoices".to_string(),
            reply_channel: "slack-finance".to_string(),
            active: true,
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&watcher).unwrap();
        let deserialized: Watcher = serde_json::from_str(&json).unwrap();

        assert_eq!(watcher.id, deserialized.id);
        assert_eq!(watcher.action, deserialized.action);
    }
}
