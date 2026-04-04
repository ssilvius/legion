# Architecture

Legion is a single Rust binary that stores data in SQLite and indexes it with Tantivy. No runtime dependencies, no background services required (watch and serve are optional).

## Storage

**SQLite** via rusqlite with WAL mode. Default location: `~/.local/share/legion/` (XDG on Linux, Application Support on macOS).

Tables:
- `reflections` -- agent memories with BM25-searchable text, domain tags, learning chains, boost/decay scoring
- `tasks` -- kanban cards with 8-state machine, priority, labels, source URLs, timestamps
- `board_reads` -- per-agent read cursors for the bullpen
- `schedules` -- cron-like scheduled posts
- `health_samples` -- system telemetry for watch pressure monitoring
- `watch_handled` -- per-repo signal handling tracking

All IDs are UUIDv7 (time-ordered). Migrations are idempotent and run on every DB open.

## Search

**Tantivy** BM25 full-text search with English stemming. The index lives alongside the database. Queries can be filtered by repo or searched across all repos (consult).

Incremental index sync detects reflections in SQLite that are missing from Tantivy and adds them automatically. No manual reindex needed after sync.

## Embeddings

**model2vec-rs** for semantic similarity (Phase 2.5). Stored as a nullable BLOB column on reflections. Each node computes its own embeddings -- they do not replicate.

## Kanban Scheduler

The kanban board is a scheduler wearing a human-friendly UI.

**Card statuses:** Backlog, Pending (Ready), Accepted (In Progress), Needs Input, In Review, Blocked, Done, Cancelled.

**State machine** enforces valid transitions. `force_move` bypasses the state machine for dashboard drag-and-drop.

**Scheduler** (`legion work`): atomically selects the highest-priority unblocked card and accepts it. Priority ordering: critical > high > med > low, then sort_order, then oldest.

**Work sources**: plugins that sync external issue trackers (GitHub, GitLab, Jira) into the kanban board. Cards link back to source issues via `source_url`.

## Communication

**Bullpen**: shared message board. Posts are reflections with `audience = 'team'`. Discoverable via consult.

**Signals**: structured bullpen posts for coordination. Format: `@recipient verb:status {details}`. Filtered separately from natural language posts.

**Channel**: MCP server providing real-time communication tools. Events arrive as they happen -- no polling.

## Watch

Long-lived daemon that polls SQLite for unhandled signals. When a signal arrives for a configured repo, spawns a headless `claude --print` session in that repo's working directory.

Features:
- Per-repo cooldown prevents wake storms
- Stagger between spawns prevents I/O storms
- System health monitoring pauses spawns under pressure
- Auto-unblock: when an agent announces completed work, blocked cards referencing that repo are automatically unblocked

## Multi-Node

Each machine runs its own legion binary with its own SQLite database. Sync between machines via smuggler:

**LAN Broadcast Sync**: UDP broadcast on local network. Encrypted datagrams (ChaCha20-Poly1305), no coordinator, no central server. Content-hash idempotency makes lost packets safe. Encryption key is membership.

**Cross-Network**: Users bring their own tunnel (Tailscale, WireGuard, ZeroTier). Smuggler broadcasts to whatever subnet it's on.

**What replicates**: SQLite rows (reflections, cards, signals, board reads).
**What stays local**: Tantivy search index, model2vec embeddings. Each node computes its own.

## Dashboard

Axum web server with rust-embed for static assets. SSE for live updates. Tabs: Feed, Signals, Board (kanban), Stats, Chat.

The dashboard is the human control surface. Agents interact via CLI.

## Project Layout

```
src/
  main.rs       -- CLI entry point (clap)
  db.rs         -- SQLite init, migrations, CRUD
  search.rs     -- Tantivy index management
  kanban.rs     -- Card, CardStatus, state machine, scheduler
  reflect.rs    -- reflection creation
  recall.rs     -- query and rank reflections
  board.rs      -- bullpen posts and filtering
  signal.rs     -- signal parsing and formatting
  surface.rs    -- cross-repo highlights
  status.rs     -- agent work status
  worksource.rs -- work source plugin discovery and execution
  watch.rs      -- auto-wake daemon
  serve.rs      -- web dashboard
  health.rs     -- system health sampling
  embed.rs      -- model2vec embeddings
  error.rs      -- error types
plugin/
  hooks/        -- Claude Code hooks
  commands/     -- slash commands
  channel/      -- MCP server
  worksources/  -- work source plugins
```
