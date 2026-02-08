//! MEMORY.md and SOUL.md synchronization

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, info, warn};

/// Load MEMORY.md contents
pub fn load_memory<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    debug!("Loading memory from {:?}", path);

    if !path.exists() {
        warn!("Memory file does not exist at {:?}, returning empty string", path);
        return Ok(String::new());
    }

    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read memory file at {:?}", path))?;

    info!("Loaded {} bytes from memory file", content.len());
    Ok(content)
}

/// Save MEMORY.md contents
pub fn save_memory<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Saving memory to {:?}", path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context(format!("Failed to create directory {:?}", parent))?;
    }

    std::fs::write(path, content)
        .context(format!("Failed to write memory file at {:?}", path))?;

    info!("Saved {} bytes to memory file", content.len());
    Ok(())
}

/// Load SOUL.md contents (meepo's core identity and purpose)
pub fn load_soul<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    debug!("Loading soul from {:?}", path);

    if !path.exists() {
        warn!("Soul file does not exist at {:?}, returning empty string", path);
        return Ok(String::new());
    }

    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read soul file at {:?}", path))?;

    info!("Loaded {} bytes from soul file", content.len());
    Ok(content)
}

/// Append to MEMORY.md
pub fn append_memory<P: AsRef<Path>>(path: P, content: &str) -> Result<()> {
    let path = path.as_ref();
    debug!("Appending to memory at {:?}", path);

    // Load existing content
    let mut existing = load_memory(path)?;

    // Append new content with newline separator
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(content);

    // Save back
    save_memory(path, &existing)?;

    info!("Appended {} bytes to memory file", content.len());
    Ok(())
}

/// Parse MEMORY.md into structured entries
///
/// Expects format like:
/// ```markdown
/// ## [Timestamp]
/// Content here...
///
/// ## [Timestamp]
/// More content...
/// ```
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub timestamp: String,
    pub content: String,
}

pub fn parse_memory(content: &str) -> Vec<MemoryEntry> {
    let mut entries = Vec::new();
    let mut current_timestamp: Option<String> = None;
    let mut current_content = String::new();

    for line in content.lines() {
        if line.starts_with("## ") {
            // Save previous entry if exists
            if let Some(timestamp) = current_timestamp.take() {
                if !current_content.trim().is_empty() {
                    entries.push(MemoryEntry {
                        timestamp,
                        content: current_content.trim().to_string(),
                    });
                }
                current_content.clear();
            }

            // Start new entry
            current_timestamp = Some(line[3..].trim().to_string());
        } else if current_timestamp.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last entry
    if let Some(timestamp) = current_timestamp {
        if !current_content.trim().is_empty() {
            entries.push(MemoryEntry {
                timestamp,
                content: current_content.trim().to_string(),
            });
        }
    }

    entries
}

/// Format memory entries back to markdown
pub fn format_memory(entries: &[MemoryEntry]) -> String {
    let mut output = String::new();

    for entry in entries {
        output.push_str(&format!("## {}\n\n{}\n\n", entry.timestamp, entry.content));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_save_and_load_memory() -> Result<()> {
        let temp_path = env::temp_dir().join("test_memory.md");
        let _ = std::fs::remove_file(&temp_path);

        let content = "# Test Memory\n\nSome content here";
        save_memory(&temp_path, content)?;

        let loaded = load_memory(&temp_path)?;
        assert_eq!(loaded, content);

        let _ = std::fs::remove_file(&temp_path);
        Ok(())
    }

    #[test]
    fn test_load_nonexistent() -> Result<()> {
        let temp_path = env::temp_dir().join("nonexistent_memory.md");
        let _ = std::fs::remove_file(&temp_path);

        let content = load_memory(&temp_path)?;
        assert_eq!(content, "");

        Ok(())
    }

    #[test]
    fn test_append_memory() -> Result<()> {
        let temp_path = env::temp_dir().join("test_append_memory.md");
        let _ = std::fs::remove_file(&temp_path);

        save_memory(&temp_path, "First line")?;
        append_memory(&temp_path, "Second line")?;

        let content = load_memory(&temp_path)?;
        assert!(content.contains("First line"));
        assert!(content.contains("Second line"));

        let _ = std::fs::remove_file(&temp_path);
        Ok(())
    }

    #[test]
    fn test_parse_memory() {
        let content = r#"
## 2026-01-01 10:00:00

This is the first memory entry.
It has multiple lines.

## 2026-01-02 15:30:00

This is the second entry.

"#;

        let entries = parse_memory(content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].timestamp, "2026-01-01 10:00:00");
        assert!(entries[0].content.contains("first memory entry"));
        assert_eq!(entries[1].timestamp, "2026-01-02 15:30:00");
        assert!(entries[1].content.contains("second entry"));
    }

    #[test]
    fn test_format_memory() {
        let entries = vec![
            MemoryEntry {
                timestamp: "2026-01-01".to_string(),
                content: "First entry".to_string(),
            },
            MemoryEntry {
                timestamp: "2026-01-02".to_string(),
                content: "Second entry".to_string(),
            },
        ];

        let formatted = format_memory(&entries);
        assert!(formatted.contains("## 2026-01-01"));
        assert!(formatted.contains("First entry"));
        assert!(formatted.contains("## 2026-01-02"));
        assert!(formatted.contains("Second entry"));
    }

    #[test]
    fn test_load_soul() -> Result<()> {
        let temp_path = env::temp_dir().join("test_soul.md");
        let _ = std::fs::remove_file(&temp_path);

        let soul_content = "# SOUL\n\nI am a helpful meepo.";
        save_memory(&temp_path, soul_content)?;

        let loaded = load_soul(&temp_path)?;
        assert_eq!(loaded, soul_content);

        let _ = std::fs::remove_file(&temp_path);
        Ok(())
    }
}
