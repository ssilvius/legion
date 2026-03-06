# CLAUDE.md

## What This Is

Legion is a local Rust binary that stores and retrieves agent reflections. It's the memory layer for Claude Code agents that work on specific codebases.

## Commands

```bash
legion reflect --repo <name> --text "reflection text"
legion reflect --repo <name> --transcript /path/to/transcript.jsonl
legion recall --repo <name> --context "what I'm working on"
legion consult --context "problem outside your domain"
legion stats --repo <name>
```

### Cross-Agent Consultation

When you encounter a problem outside your domain, use `legion consult` to search
reflections from ALL repos/agents:

```bash
legion consult --context "discriminated unions in composite rules" --limit 3
```

This searches across kelex, rafters, platform, and all other agent reflections
using BM25. Results include source repo attribution so you know which domain
the knowledge came from. Use this when you hit something unfamiliar -- another
agent may have already solved it.

## Architecture

- **Storage**: SQLite via rusqlite (XDG data dir: ~/.local/share/legion/)
- **Search**: Tantivy BM25 full-text index (Phase 1)
- **Future**: model2vec-rs embeddings in nullable BLOB column (Phase 2)

## Rules

- No emoji in code, comments, or documentation
- No `unwrap()` in production code
- No `unsafe` code
- All types explicit
- `cargo clippy -- -D warnings` must pass
- `cargo fmt -- --check` must pass
- Errors use thiserror derive macros
- UUIDv7 for all IDs

## Data Model

```sql
CREATE TABLE reflections (
    id TEXT PRIMARY KEY,        -- UUIDv7
    repo TEXT NOT NULL,         -- repository name
    text TEXT NOT NULL,         -- the reflection
    created_at TEXT NOT NULL,   -- ISO 8601
    embedding BLOB              -- nullable, Phase 2
);

CREATE INDEX idx_reflections_repo ON reflections(repo);
CREATE INDEX idx_reflections_created ON reflections(created_at);
```

## Phase Plan

1. **Phase 1** (complete): SQLite + Tantivy BM25. Store reflections, recall by text similarity.
2. **Phase 1.5** (complete): Cross-agent consultation via `legion consult`. BM25 search across all repos.
3. **Phase 2** (when BM25 hits semantic wall): Add model2vec-rs, hybrid BM25 + cosine scoring. Synapse agent for quality gating.
4. **Phase 3** (if needed): fastembed-rs with bge-small-en-v1.5 for higher quality.

## Hook Integration

Legion is called by Claude Code hooks:
- `SessionStart` hook calls `legion recall` and injects context via additionalContext
- `Stop` hook prompts the agent to reflect before closing
- `consult` is agent-initiated (called via Bash mid-session), not hook-driven

## Project Layout

```
src/
  main.rs          -- CLI entry point (clap)
  db.rs            -- SQLite init, migrations, CRUD
  search.rs        -- Tantivy index management
  reflect.rs       -- Reflection creation (from text or transcript)
  recall.rs        -- Query and rank reflections
  stats.rs         -- Reflection statistics reporting
  error.rs         -- Error types
  testutil.rs      -- Shared test helpers (#[cfg(test)] only)
tests/
  integration.rs   -- End-to-end binary tests
```
