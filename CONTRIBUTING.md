# Contributing to Meepo

## Prerequisites

- **macOS** (required for AppleScript integrations and iMessage)
- **Rust toolchain** — Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Optional CLIs** (for specific tools):
  - `gh` — GitHub CLI (`brew install gh`)
  - `claude` — Claude Code CLI (`npm install -g @anthropic-ai/claude-code`)

## Getting Started

```bash
git clone https://github.com/kavymi/meepo.git
cd meepo

# Build the workspace
cargo build

# Run all tests (167 tests across 5 crates)
cargo test --workspace

# Run tests for a single crate
cargo test -p meepo-core

# Build release binary
cargo build --release
```

## Workspace Layout

Meepo is a Cargo workspace with 5 crates. Dependencies flow downward — `meepo-cli` depends on everything, leaf crates have no internal dependencies.

```
crates/
├── meepo-cli/          # Binary entry point
│   └── src/
│       ├── main.rs     # CLI commands, daemon startup, event loop
│       └── config.rs   # Config loading, env var expansion
│
├── meepo-core/         # Core agent logic (largest crate)
│   └── src/
│       ├── lib.rs      # Public API exports
│       ├── agent.rs    # Agent struct, message handling, conversation history
│       ├── api.rs      # Anthropic API client, tool loop
│       ├── tavily.rs   # Tavily Search/Extract client
│       ├── orchestrator.rs  # Sub-agent task orchestrator
│       └── tools/      # All 25 tool implementations
│           ├── mod.rs          # ToolHandler trait, ToolRegistry
│           ├── macos.rs        # Mail, Calendar, Clipboard, Apps (6 tools)
│           ├── accessibility.rs # Screen reader, click, type (3 tools)
│           ├── code.rs         # write_code, make_pr, review_pr (3 tools)
│           ├── search.rs       # web_search via Tavily (1 tool)
│           ├── memory.rs       # Knowledge graph tools (4 tools)
│           ├── system.rs       # Commands, files, browse_url (4 tools)
│           ├── watchers.rs     # Watcher management (3 tools)
│           └── delegate.rs     # Sub-agent delegation (1 tool)
│
├── meepo-channels/     # Messaging adapters
│   └── src/
│       ├── lib.rs      # MessageBus, BusSender, MessageChannel trait
│       ├── discord.rs  # Discord via Serenity WebSocket
│       ├── slack.rs    # Slack via HTTP polling
│       └── imessage.rs # iMessage via SQLite + AppleScript
│
├── meepo-knowledge/    # Persistence layer
│   └── src/
│       ├── lib.rs      # KnowledgeGraph (combines SQLite + Tantivy)
│       ├── db.rs       # KnowledgeDb (SQLite operations)
│       └── search.rs   # TantivyIndex (full-text search)
│
└── meepo-scheduler/    # Background watchers
    └── src/
        ├── lib.rs      # WatcherRunner, task management
        ├── watchers.rs # 7 watcher types (email, calendar, file, etc.)
        └── persistence.rs # Watcher state in SQLite
```

## Key Patterns

**Tool system:** All tools implement the `ToolHandler` trait (`name()`, `description()`, `input_schema()`, `execute()`). They're registered in a `ToolRegistry` (HashMap-backed) at daemon startup. The API client runs a tool loop until Claude returns a final text response or hits the 10-iteration limit.

**Channel adapters:** Channels implement `MessageChannel` trait (`start()`, `send()`, `channel_type()`). The `MessageBus` splits into a receiver and an `Arc<BusSender>` for concurrent send/receive.

**Secrets in config:** API keys use `${ENV_VAR}` syntax in TOML, expanded at load time. Never hardcode secrets. Structs holding secrets get custom `Debug` impls (not `#[derive(Debug)]`).

**Optional providers:** Use `Option<Config>` with `#[serde(default)]` for optional features like Tavily. Construct the client only if the key is non-empty. Conditionally register tools.

**Concurrency:** Use `tokio::sync::Semaphore` for concurrency limits. Use CAS loops (`compare_exchange`) for atomic counters under contention, not load-then-increment.

## Running Locally

```bash
# Initialize config (creates ~/.meepo/)
cargo run -- init

# Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# Start daemon in debug mode
cargo run -- --debug start

# One-shot test (doesn't need daemon running)
cargo run -- ask "Hello, what tools do you have?"
```

## Adding a New Tool

1. Create a struct in the appropriate file under `crates/meepo-core/src/tools/`
2. Implement `ToolHandler` trait:
   ```rust
   #[async_trait]
   impl ToolHandler for MyTool {
       fn name(&self) -> &str { "my_tool" }
       fn description(&self) -> &str { "Does something useful" }
       fn input_schema(&self) -> serde_json::Value {
           serde_json::json!({
               "type": "object",
               "properties": { ... },
               "required": [...]
           })
       }
       async fn execute(&self, input: serde_json::Value) -> anyhow::Result<String> {
           // Implementation
       }
   }
   ```
3. Register it in `crates/meepo-cli/src/main.rs` during daemon startup
4. Add tests

## Pull Request Workflow

1. Create a feature branch: `git checkout -b feature/my-feature`
2. Make changes
3. Run tests: `cargo test --workspace`
4. Run clippy: `cargo clippy --workspace`
5. Open a PR against `main`

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
