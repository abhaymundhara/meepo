## What

Brief description of the change.

## Why

Why is this change needed?

## How

How was this implemented? Any notable design decisions?

## Testing

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` passes
- [ ] Tested manually (describe below)

## Checklist

- [ ] Code follows existing style and patterns
- [ ] No hardcoded secrets or personal information
- [ ] New tools implement `ToolHandler` trait and are registered at startup
- [ ] Platform-specific code uses the `platform` abstraction layer
- [ ] Config changes are reflected in `config/default.toml` with comments
