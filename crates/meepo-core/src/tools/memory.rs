//! Knowledge graph and memory tools

use async_trait::async_trait;
use serde_json::Value;
use anyhow::{Result, Context};
use std::sync::Arc;
use tracing::debug;

use meepo_knowledge::KnowledgeDb;
use super::{ToolHandler, json_schema};

/// Remember information by adding to knowledge graph
pub struct RememberTool {
    db: Arc<KnowledgeDb>,
}

impl RememberTool {
    pub fn new(db: Arc<KnowledgeDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolHandler for RememberTool {
    fn name(&self) -> &str {
        "remember"
    }

    fn description(&self) -> &str {
        "Remember important information by storing it in the knowledge graph. \
         Creates an entity with a name, type, and optional metadata."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "name": {
                    "type": "string",
                    "description": "Name or identifier for this piece of knowledge"
                },
                "entity_type": {
                    "type": "string",
                    "description": "Type of entity (e.g., 'person', 'concept', 'fact', 'preference')"
                },
                "metadata": {
                    "type": "object",
                    "description": "Additional structured information about this entity"
                }
            }),
            vec!["name", "entity_type"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let name = input.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' parameter"))?;
        let entity_type = input.get("entity_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'entity_type' parameter"))?;
        let metadata = input.get("metadata").cloned();

        debug!("Remembering: {} (type: {})", name, entity_type);

        let entity_id = self.db.insert_entity(name, entity_type, metadata)
            .context("Failed to insert entity")?;

        Ok(format!("Remembered '{}' with ID: {}", name, entity_id))
    }
}

/// Recall information from knowledge graph
pub struct RecallTool {
    db: Arc<KnowledgeDb>,
}

impl RecallTool {
    pub fn new(db: Arc<KnowledgeDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolHandler for RecallTool {
    fn name(&self) -> &str {
        "recall"
    }

    fn description(&self) -> &str {
        "Search the knowledge graph for previously stored information. \
         Returns matching entities based on name or type."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "query": {
                    "type": "string",
                    "description": "Search query (searches in name and type)"
                },
                "entity_type": {
                    "type": "string",
                    "description": "Optional: filter by entity type"
                }
            }),
            vec!["query"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let query = input.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
        let entity_type = input.get("entity_type").and_then(|v| v.as_str());

        debug!("Searching knowledge graph for: {}", query);

        let results = self.db.search_entities(query, entity_type)
            .context("Failed to search entities")?;

        if results.is_empty() {
            return Ok("No matching information found.".to_string());
        }

        let mut output = format!("Found {} result(s):\n\n", results.len());
        for entity in results.iter().take(10) {
            output.push_str(&format!("- {} ({})", entity.name, entity.entity_type));
            if let Some(metadata) = &entity.metadata {
                output.push_str(&format!("\n  Metadata: {}", metadata));
            }
            output.push('\n');
        }

        Ok(output)
    }
}

/// Link entities together in knowledge graph
pub struct LinkEntitiesTool {
    db: Arc<KnowledgeDb>,
}

impl LinkEntitiesTool {
    pub fn new(db: Arc<KnowledgeDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolHandler for LinkEntitiesTool {
    fn name(&self) -> &str {
        "link_entities"
    }

    fn description(&self) -> &str {
        "Create a relationship between two entities in the knowledge graph. \
         Useful for building connections between concepts, people, facts, etc."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "source_id": {
                    "type": "string",
                    "description": "ID of the source entity"
                },
                "target_id": {
                    "type": "string",
                    "description": "ID of the target entity"
                },
                "relation_type": {
                    "type": "string",
                    "description": "Type of relationship (e.g., 'related_to', 'works_with', 'part_of')"
                },
                "metadata": {
                    "type": "object",
                    "description": "Optional metadata about the relationship"
                }
            }),
            vec!["source_id", "target_id", "relation_type"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let source_id = input.get("source_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'source_id' parameter"))?;
        let target_id = input.get("target_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'target_id' parameter"))?;
        let relation_type = input.get("relation_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'relation_type' parameter"))?;
        let metadata = input.get("metadata").cloned();

        debug!("Linking {} -> {} ({})", source_id, target_id, relation_type);

        let rel_id = self.db.insert_relationship(source_id, target_id, relation_type, metadata)
            .context("Failed to create relationship")?;

        Ok(format!("Created relationship with ID: {}", rel_id))
    }
}

/// Search knowledge graph using full-text search
pub struct SearchKnowledgeTool {
    db: Arc<KnowledgeDb>,
}

impl SearchKnowledgeTool {
    pub fn new(db: Arc<KnowledgeDb>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ToolHandler for SearchKnowledgeTool {
    fn name(&self) -> &str {
        "search_knowledge"
    }

    fn description(&self) -> &str {
        "Perform a full-text search across all stored knowledge. \
         More powerful than recall for finding relevant information."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of results (default: 10)"
                }
            }),
            vec!["query"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let query = input.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
        let limit = input.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        debug!("Full-text search for: {}", query);

        // Use the basic search for now (Tantivy integration would go here)
        let results = self.db.search_entities(query, None)
            .context("Failed to search knowledge")?;

        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        let mut output = format!("Found {} result(s):\n\n", results.len().min(limit));
        for entity in results.iter().take(limit) {
            output.push_str(&format!("- {} ({})\n", entity.name, entity.entity_type));
            if let Some(metadata) = &entity.metadata {
                output.push_str(&format!("  {}\n", metadata));
            }
        }

        Ok(output)
    }
}
