# CLAUDE.md

## What This Is

Legion is a local Rust binary that stores and retrieves agent reflections. It's the memory layer for Claude Code agents that work on specific codebases.

## Commands

```bash
legion reflect --repo <name> --text "reflection text"
legion reflect --repo <name> --transcript /path/to/transcript.jsonl
legion recall --repo <name> --context "what I'm working on"
legion consult --context "problem outside your domain" --limit <n>
legion post --repo <name> --text "share with the team"
legion board --repo <name>
legion board --count --repo <name>
legion stats --repo <name>
```

### Cross-Agent Consultation

When you encounter a problem outside your domain, use `legion consult` to search
reflections from ALL repos/agents:

```bash
legion consult --context "discriminated unions in composite rules" --limit 3
```

This searches across all indexed agent reflections regardless of repo (e.g., kelex, rafters, platform)
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

### Water Cooler (Push-Based Communication)

Push something to the team instead of keeping it to yourself:

```bash
legion post --repo rafters --text "OKLCH bet paid off"
legion board --repo kelex              # read all posts, mark as read
legion board --count --repo kelex      # unread count only (for hooks)
```

Posts are reflections with `audience = 'team'`. Discoverable via `consult` for free.

## Phase Plan

1. **Phase 1** (complete): SQLite + Tantivy BM25. Store reflections, recall by text similarity.
2. **Phase 1.5** (complete): Cross-agent consultation via `legion consult`. BM25 search across all repos.
3. **Phase 1.75** (issues #33-#35): Water cooler. `legion post` and `legion board` for push-based agent communication.
4. **Phase 2** (when BM25 hits semantic wall): Add model2vec-rs, hybrid BM25 + cosine scoring. Synapse agent for quality gating.
5. **Phase 3** (if needed): fastembed-rs with bge-small-en-v1.5 for higher quality.

## Hook Integration

Legion is called by Claude Code hooks:
- `SessionStart` hook calls `legion recall` and injects context via additionalContext. Also shows unread board post count.
- `Stop` hook prompts the agent to reflect before closing
- `consult` is agent-initiated (called via Bash mid-session), not hook-driven
- `post` is agent-initiated (when agent has something worth sharing with the team)
- `board` is agent-initiated (when agent wants to read what others posted)

## Project Layout

```
src/
  main.rs          -- CLI entry point (clap)
  db.rs            -- SQLite init, migrations, CRUD
  search.rs        -- Tantivy index management
  reflect.rs       -- Reflection creation (from text or transcript)
  recall.rs        -- Query and rank reflections
  board.rs         -- Water cooler: post and board commands
  stats.rs         -- Reflection statistics reporting
  error.rs         -- Error types
  testutil.rs      -- Shared test helpers (#[cfg(test)] only)
tests/
  integration.rs   -- End-to-end binary tests
docs/
  plans/           -- Design documents
```
