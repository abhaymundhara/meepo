//! RAG-enhanced tools: smart_recall and ingest_document
//!
//! These tools leverage the new RAG features (hybrid search, GraphRAG,
//! document chunking) to provide more powerful knowledge retrieval
//! and document ingestion capabilities.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info};

use super::{ToolHandler, json_schema};
use meepo_knowledge::chunking::{
    ChunkingConfig, DocumentMetadata, chunk_text, detect_content_type,
};
use meepo_knowledge::graph_rag::{GraphRagConfig, format_graph_context, graph_expand};
use meepo_knowledge::{KnowledgeDb, KnowledgeGraph};

/// Smart recall tool that uses GraphRAG for relationship-aware retrieval.
///
/// Unlike the basic `recall` tool, this traverses entity relationships
/// to pull in contextually connected knowledge.
pub struct SmartRecallTool {
    graph: Arc<KnowledgeGraph>,
    db: Arc<KnowledgeDb>,
    config: GraphRagConfig,
}

impl SmartRecallTool {
    pub fn new(graph: Arc<KnowledgeGraph>, db: Arc<KnowledgeDb>) -> Self {
        Self {
            graph,
            db,
            config: GraphRagConfig::default(),
        }
    }

    pub fn with_config(mut self, config: GraphRagConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl ToolHandler for SmartRecallTool {
    fn name(&self) -> &str {
        "smart_recall"
    }

    fn description(&self) -> &str {
        "Search the knowledge graph with relationship-aware retrieval (GraphRAG). \
         Finds directly matching entities AND related knowledge by traversing \
         entity relationships. Returns richer context than basic recall."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "query": {
                    "type": "string",
                    "description": "Search query for knowledge retrieval"
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of direct results (default: 5)"
                },
                "max_hops": {
                    "type": "number",
                    "description": "Maximum relationship hops to traverse (default: 2)"
                }
            }),
            vec!["query"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let max_hops = input.get("max_hops").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

        debug!(
            "Smart recall for: {} (limit={}, hops={})",
            query, limit, max_hops
        );

        // Step 1: Search using Tantivy full-text search
        let search_results = self
            .graph
            .search(query, limit)
            .context("Failed to search knowledge graph")?;

        if search_results.is_empty() {
            return Ok("No matching knowledge found.".to_string());
        }

        // Step 2: Expand via GraphRAG
        let seeds: Vec<(String, f32)> = search_results
            .iter()
            .map(|r| (r.id.clone(), r.score))
            .collect();

        let config = GraphRagConfig {
            max_hops,
            max_expanded_results: limit * 3,
            ..self.config.clone()
        };

        let expanded = graph_expand(&self.db, &seeds, &config)
            .await
            .context("Failed to expand via GraphRAG")?;

        // Step 3: Format results
        let context = format_graph_context(&expanded, &config);

        if context.is_empty() {
            return Ok("No matching knowledge found.".to_string());
        }

        let mut output = format!(
            "Found {} result(s) ({} direct, {} via relationships):\n\n",
            expanded.len(),
            search_results.len(),
            expanded.len().saturating_sub(search_results.len())
        );
        output.push_str(&context);

        Ok(output)
    }
}

/// Ingest a document into the knowledge graph by chunking and indexing it.
pub struct IngestDocumentTool {
    graph: Arc<KnowledgeGraph>,
    chunking_config: ChunkingConfig,
}

impl IngestDocumentTool {
    pub fn new(graph: Arc<KnowledgeGraph>) -> Self {
        Self {
            graph,
            chunking_config: ChunkingConfig::default(),
        }
    }

    pub fn with_chunking_config(mut self, config: ChunkingConfig) -> Self {
        self.chunking_config = config;
        self
    }
}

#[async_trait]
impl ToolHandler for IngestDocumentTool {
    fn name(&self) -> &str {
        "ingest_document"
    }

    fn description(&self) -> &str {
        "Ingest a document into the knowledge graph. The document is split into \
         chunks and each chunk is indexed for later retrieval. Supports text files, \
         markdown, code, and other text formats. Use this to build up the knowledge \
         base from files."
    }

    fn input_schema(&self) -> Value {
        json_schema(
            serde_json::json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to ingest"
                },
                "title": {
                    "type": "string",
                    "description": "Optional title for the document (defaults to filename)"
                },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional tags to associate with this document"
                }
            }),
            vec!["path"],
        )
    }

    async fn execute(&self, input: Value) -> Result<String> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
        let title = input.get("title").and_then(|v| v.as_str());
        let tags: Vec<String> = input
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Expand ~ in path
        let expanded_path = if let Some(rest) = path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(rest)
            } else {
                std::path::PathBuf::from(path)
            }
        } else {
            std::path::PathBuf::from(path)
        };

        // Read the file
        let content = tokio::fs::read_to_string(&expanded_path)
            .await
            .context(format!("Failed to read file: {}", expanded_path.display()))?;

        if content.is_empty() {
            return Ok("File is empty, nothing to ingest.".to_string());
        }

        let filename = expanded_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let doc_title = title.unwrap_or(&filename);
        let content_type = detect_content_type(path);

        info!(
            "Ingesting document: {} ({} chars, {})",
            doc_title,
            content.len(),
            content_type
        );

        // Chunk the document
        let chunks = chunk_text(&content, &self.chunking_config);

        // Create a parent document entity
        let doc_metadata = serde_json::json!({
            "source_path": path,
            "content_type": content_type,
            "total_chars": content.len(),
            "chunk_count": chunks.len(),
            "tags": tags,
        });

        let doc_id = self
            .graph
            .add_entity(doc_title, "document", Some(doc_metadata))
            .await
            .context("Failed to create document entity")?;

        // Index each chunk as a child entity linked to the document
        let mut chunk_ids = Vec::new();
        for chunk in &chunks {
            let chunk_name = format!(
                "{} [chunk {}/{}]",
                doc_title,
                chunk.chunk_index + 1,
                chunk.total_chunks
            );

            let chunk_metadata = serde_json::json!({
                "full_content": chunk.content,
                "chunk_index": chunk.chunk_index,
                "start_offset": chunk.start_offset,
                "end_offset": chunk.end_offset,
                "total_chunks": chunk.total_chunks,
                "parent_document": doc_id,
            });

            let chunk_id = self
                .graph
                .add_entity(&chunk_name, "document_chunk", Some(chunk_metadata))
                .await
                .context("Failed to create chunk entity")?;

            // Link chunk to parent document
            self.graph
                .link_entities(&doc_id, &chunk_id, "contains_chunk", None)
                .await
                .context("Failed to link chunk to document")?;

            chunk_ids.push(chunk_id);
        }

        // Link consecutive chunks
        for window in chunk_ids.windows(2) {
            let _ = self
                .graph
                .link_entities(&window[0], &window[1], "next_chunk", None)
                .await;
        }

        let metadata = DocumentMetadata {
            source_path: Some(path.to_string()),
            title: Some(doc_title.to_string()),
            content_type: content_type.to_string(),
            total_chars: content.len(),
            chunk_count: chunks.len(),
        };

        Ok(format!(
            "Ingested '{}': {} chunks created from {} chars ({})\nDocument ID: {}",
            metadata.title.as_deref().unwrap_or("unknown"),
            metadata.chunk_count,
            metadata.total_chars,
            metadata.content_type,
            doc_id
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_recall_schema() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let index_path = temp.path().join("test_index");
        let graph = Arc::new(KnowledgeGraph::new(&db_path, &index_path).unwrap());
        let db = graph.db();

        let tool = SmartRecallTool::new(graph, db);
        assert_eq!(tool.name(), "smart_recall");
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("query").is_some());
    }

    #[test]
    fn test_ingest_document_schema() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let index_path = temp.path().join("test_index");
        let graph = Arc::new(KnowledgeGraph::new(&db_path, &index_path).unwrap());

        let tool = IngestDocumentTool::new(graph);
        assert_eq!(tool.name(), "ingest_document");
        let schema = tool.input_schema();
        assert!(schema.get("properties").is_some());
        assert!(schema["properties"].get("path").is_some());
    }

    #[tokio::test]
    async fn test_smart_recall_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let index_path = temp.path().join("test_index");
        let graph = Arc::new(KnowledgeGraph::new(&db_path, &index_path).unwrap());
        let db = graph.db();

        let tool = SmartRecallTool::new(graph, db);
        let result = tool
            .execute(serde_json::json!({"query": "nonexistent_xyz"}))
            .await
            .unwrap();
        assert!(result.contains("No matching"));
    }

    #[tokio::test]
    async fn test_ingest_nonexistent_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let index_path = temp.path().join("test_index");
        let graph = Arc::new(KnowledgeGraph::new(&db_path, &index_path).unwrap());

        let tool = IngestDocumentTool::new(graph);
        let result = tool
            .execute(serde_json::json!({"path": "/nonexistent/file.txt"}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ingest_and_recall() {
        let temp = tempfile::TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        let index_path = temp.path().join("test_index");
        let graph = Arc::new(KnowledgeGraph::new(&db_path, &index_path).unwrap());
        let db = graph.db();

        // Create a test file
        let test_file = temp.path().join("test_doc.md");
        tokio::fs::write(
            &test_file,
            "# Rust Programming\n\nRust is a systems programming language.\n\n\
             It focuses on safety and performance.",
        )
        .await
        .unwrap();

        // Ingest it
        let ingest = IngestDocumentTool::new(graph.clone());
        let result = ingest
            .execute(serde_json::json!({
                "path": test_file.to_str().unwrap(),
                "title": "Rust Guide"
            }))
            .await
            .unwrap();
        assert!(result.contains("Ingested"));
        assert!(result.contains("Rust Guide"));

        // Recall it
        let recall = SmartRecallTool::new(graph, db);
        let result = recall
            .execute(serde_json::json!({"query": "Rust programming"}))
            .await
            .unwrap();
        assert!(result.contains("Found"));
    }
}
