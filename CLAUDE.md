# CLAUDE.md

## What This Is

Legion is a local Rust binary that stores and retrieves agent reflections. It's the memory layer for Claude Code agents that work on specific codebases.

## Commands

```bash
legion reflect --repo <name> --text "reflection text"
legion reflect --repo <name> --transcript /path/to/transcript.jsonl
legion reflect --repo <name> --text "..." --domain color-tokens --tags "semantic,consumer"
legion reflect --repo <name> --text "..." --follows <parent-id>
legion recall --repo <name> --context "what I'm working on"
legion consult --context "problem outside your domain" --limit <n>
legion post --repo <name> --text "share with the team"
legion board --repo <name>
legion board --count --repo <name>
legion boost --id <reflection-id>
legion chain --id <reflection-id>
legion surface --repo <name>
legion stats --repo <name>
legion reindex
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
    id TEXT PRIMARY KEY,           -- UUIDv7
    repo TEXT NOT NULL,            -- repository name
    text TEXT NOT NULL,            -- the reflection
    created_at TEXT NOT NULL,      -- ISO 8601
    audience TEXT NOT NULL DEFAULT 'self',  -- 'self' or 'team'
    domain TEXT,                   -- classification tag (e.g., "color-tokens")
    tags TEXT,                     -- comma-separated tags
    recall_count INTEGER NOT NULL DEFAULT 0,  -- boost counter
    last_recalled_at TEXT,         -- for decay calculation
    parent_id TEXT,                -- learning chain link
    embedding BLOB                 -- nullable, Phase 2.5
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
3. **Phase 1.75** (complete): Water cooler. `legion post` and `legion board` for push-based agent communication.
4. **Phase 2.0** (complete): Synapse metadata. Domain/tags, learning chains, boost/decay ranking, `legion surface`.
5. **Phase 2.5** (next): Add model2vec-rs embeddings, hybrid BM25 + cosine scoring, transfer detection.
6. **Phase 3** (if needed): LLM classification via Synapse agent for quality gating.

## Hook Integration

Legion is called by Claude Code hooks:
- `SessionStart` hook calls `legion recall` + `legion surface` and injects context via additionalContext
- `Stop` hook prompts the agent to reflect before closing
- `consult` is agent-initiated (called via Bash mid-session), not hook-driven
- `post` is agent-initiated (when agent has something worth sharing with the team)
- `board` is agent-initiated (when agent wants to read what others posted)
- `boost` is agent-initiated (after recalling and successfully applying a reflection)
- `chain` is agent-initiated (to trace a learning chain)

## Project Layout

```
src/
  main.rs          -- CLI entry point (clap)
  db.rs            -- SQLite init, migrations, CRUD
  search.rs        -- Tantivy index management
  reflect.rs       -- Reflection creation (from text or transcript)
  recall.rs        -- Query and rank reflections (weighted by boost/decay)
  board.rs         -- Water cooler: post and board commands
  surface.rs       -- Cross-repo highlight surfacing
  stats.rs         -- Reflection statistics reporting
  init.rs          -- Hook script generation and settings.json management
  error.rs         -- Error types
  testutil.rs      -- Shared test helpers (#[cfg(test)] only)
tests/
  integration.rs   -- End-to-end binary tests
docs/
  plans/           -- Design documents
```
