//! Context loading and system prompt building

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{debug, warn};

/// Load SOUL.md content from file
pub fn load_soul<P: AsRef<Path>>(path: P) -> Result<String> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to load SOUL from {:?}", path.as_ref()))?;

    debug!("Loaded SOUL.md ({} bytes)", content.len());
    Ok(content)
}

/// Load MEMORY.md content from file
pub fn load_memory<P: AsRef<Path>>(path: P) -> Result<String> {
    match std::fs::read_to_string(path.as_ref()) {
        Ok(content) => {
            debug!("Loaded MEMORY.md ({} bytes)", content.len());
            Ok(content)
        }
        Err(e) => {
            warn!("Failed to load MEMORY.md: {}. Using empty memory.", e);
            Ok(String::new())
        }
    }
}

/// Build complete system prompt from components
pub fn build_system_prompt(soul: &str, memory: &str, extra_context: &str) -> String {
    let mut prompt = String::new();

    // Add SOUL first - this is the core identity
    if !soul.is_empty() {
        prompt.push_str("# IDENTITY\n\n");
        prompt.push_str(soul);
        prompt.push_str("\n\n");
    }

    // Add MEMORY - accumulated knowledge
    if !memory.is_empty() {
        prompt.push_str("# MEMORY\n\n");
        prompt.push_str(memory);
        prompt.push_str("\n\n");
    }

    // Add extra context - conversation history, relevant entities, etc.
    if !extra_context.is_empty() {
        prompt.push_str("# CONTEXT\n\n");
        prompt.push_str(extra_context);
        prompt.push_str("\n\n");
    }

    // Add current timestamp
    prompt.push_str("# CURRENT TIME\n\n");
    prompt.push_str(&chrono::Utc::now().to_rfc3339());
    prompt.push_str("\n\n");

    // Add instructions
    prompt.push_str("# INSTRUCTIONS\n\n");
    prompt.push_str("You are an autonomous agent with access to powerful tools. ");
    prompt.push_str("Use your tools proactively to help the user. ");
    prompt.push_str("When you learn something important, use the Remember tool to store it. ");
    prompt.push_str("Be concise but thorough. ");
    prompt.push_str("Always think step-by-step about complex tasks.\n");

    debug!("Built system prompt ({} chars)", prompt.len());

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_soul() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "I am a helpful assistant")?;

        let content = load_soul(file.path())?;
        assert!(content.contains("helpful assistant"));

        Ok(())
    }

    #[test]
    fn test_build_system_prompt() {
        let soul = "I am meepo";
        let memory = "The user likes Rust";
        let context = "Recent conversation about async programming";

        let prompt = build_system_prompt(soul, memory, context);

        assert!(prompt.contains("IDENTITY"));
        assert!(prompt.contains("MEMORY"));
        assert!(prompt.contains("CONTEXT"));
        assert!(prompt.contains("meepo"));
        assert!(prompt.contains("Rust"));
    }

    #[test]
    fn test_build_system_prompt_empty() {
        let prompt = build_system_prompt("", "", "");
        assert!(prompt.contains("INSTRUCTIONS"));
        assert!(prompt.contains("CURRENT TIME"));
    }
}
