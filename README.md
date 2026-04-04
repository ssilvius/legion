# Legion

Multi-agent orchestration for Claude Code. Memory, coordination, and kanban -- all inside the walled garden.

## What It Does

Legion gives Claude Code agents persistent memory across sessions, a shared communication channel, and a kanban board that humans manage while agents execute. No API keys, no external services, no terms violations. Runs on your Max plan.

**Memory**: Agents reflect at session end, recall at session start. Knowledge compounds.

**Team Communication**: Bullpen for broadcast, signals for structured coordination, auto-wake for sleeping agents.

**Kanban Scheduler**: Cards, columns, priorities. You manage the board, agents pick up work. External issues (GitHub, GitLab, Jira) sync in via plugins.

**Multi-Node**: SQLite-based. Sync between machines with [smugglr](https://github.com/rafters-studio/smuggler). No central server.

## Install

```bash
claude install legion
```

Installs the binary, hooks, slash commands, and real-time channel.

## Quick Start

```bash
# Agents remember
legion reflect --repo myproject --text "arrays are tricky in codegen"
legion recall --repo myproject --context "codegen arrays"

# Agents talk
legion post --repo myproject --text "found a pattern worth sharing"
legion signal --repo myproject --to teammate --verb review --status ready

# You manage work
legion kanban create --from sean --to myproject --text "implement search" --priority high
legion work --repo myproject          # agent picks up next card
legion done --repo myproject --text "shipped" --id <card-id>

# Agents wake each other
legion watch
```

## Architecture

Local SQLite + Tantivy search. No cloud dependency. Agents are native Claude Code sessions communicating through the shared database. The kanban board is a scheduler underneath -- priority queue with dependency resolution wearing a UI humans understand.

Work source plugins sync external issue trackers. GitHub ships first. The interface is an executable that speaks a three-command protocol (list, close, detect).

## Development

```bash
cargo test                        # 300+ tests
cargo clippy -- -D warnings
cargo fmt -- --check
```

Git hooks enforce quality: pre-commit runs simplify, pre-push runs full review. Both via `claude -p` on Max plan.

## License

MIT
