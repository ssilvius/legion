# Legion

Agent specialization through deliberate practice.

Legion is a local Rust CLI that stores and retrieves agent reflections. It's the memory layer for Claude Code agents -- every session ends with a reflection, every session starts with relevant context from past work.

The idea: agents that develop real expertise through accumulated experience, not static prompts. Each repo builds its own corpus of learned heuristics over time. The shape of the corpus IS the expertise.

## Install

```bash
cargo install --path .
```

Requires Rust stable toolchain. The binary installs to `~/.cargo/bin/legion`.

## Usage

```bash
# Store a reflection
legion reflect --repo kelex --text "Zod discriminated unions need the discriminator field identified before mapping child schemas"

# Store from a session transcript (uses last assistant message)
legion reflect --repo kelex --transcript ~/.claude/projects/.../transcript.jsonl

# Recall by relevance (BM25 full-text search)
legion recall --repo kelex --context "Zod schema mapping"

# Recall most recent (bypasses search, useful for hooks)
legion recall --repo kelex --latest --limit 3

# Consult across all repos (cross-agent knowledge sharing)
legion consult --context "discriminated unions in composite rules" --limit 3

# Statistics
legion stats
legion stats --repo kelex
```

## How It Works

**Reflect**: After each session, the agent answers: "What would you tell another agent who hits this same problem tomorrow?" This framing produces actionable knowledge, not vague journaling. The reflection is stored in SQLite and indexed in Tantivy for BM25 full-text search.

**Recall**: At session start, relevant reflections are retrieved and injected into the agent's context. On feature branches, BM25 searches using the branch name as context. Falls back to most recent reflections when no keyword match exists.

**Isolation**: Reflections are scoped by repo name. Kelex reflections never leak into rafters queries. Each codebase builds its own expertise corpus.

## Claude Code Hooks

Legion integrates via two hooks in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/legion-recall.sh",
            "timeout": 10
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/legion-reflect.sh",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

**SessionStart** (`legion-recall.sh`): Retrieves reflections for the current repo. Tries BM25 search with the git branch name first, falls back to `--latest` for broad recall. Returns results as `additionalContext`.

**Stop** (`legion-reflect.sh`): Blocks the agent from stopping and prompts it to reflect. Uses `stop_hook_active` to prevent loops -- the agent reflects, tries to stop again, and passes through cleanly.

## Architecture

- **Storage**: SQLite via rusqlite with WAL mode. XDG data dir (`~/Library/Application Support/legion/` on macOS, `~/.local/share/legion/` on Linux). Override with `LEGION_DATA_DIR` env var.
- **Search**: Tantivy BM25 with English stemming. Queries are filtered by repo (exact match) and ranked by text relevance.
- **IDs**: UUIDv7 (time-ordered, non-predictable).

### Data Model

```sql
CREATE TABLE reflections (
    id TEXT PRIMARY KEY,        -- UUIDv7
    repo TEXT NOT NULL,         -- repository name
    text TEXT NOT NULL,         -- the reflection
    created_at TEXT NOT NULL,   -- ISO 8601
    embedding BLOB              -- nullable, reserved for Phase 2
);
```

## Phase Plan

1. **Phase 1** (complete): SQLite + Tantivy BM25. Store reflections, recall by text similarity.
2. **Phase 1.5** (complete): Cross-agent consultation via `legion consult`. BM25 search across all repos.
3. **Phase 2**: Add model2vec-rs (potion-retrieval-32M) for hybrid BM25 + cosine scoring when keyword matching hits the semantic wall.
4. **Phase 3**: fastembed-rs with bge-small-en-v1.5 if higher quality embeddings are needed.

## Development

```bash
cargo test          # 53 tests (47 unit + 6 integration)
cargo clippy -- -D warnings
cargo fmt -- --check
```

## License

Private.
