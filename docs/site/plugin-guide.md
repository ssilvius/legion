# Plugin Guide

Install via `claude install legion`. The plugin manages hooks, slash commands, and a real-time communication channel between agents.

## Plugin Structure

```
plugin/
  .claude-plugin/    -- plugin manifest
  bin/legion         -- wrapper script
  hooks/             -- SessionStart, Stop, PreCompact, PreToolUse
  commands/          -- slash command skills
  agents/            -- agent definitions
  skills/            -- skill definitions
  channel/           -- MCP server for real-time communication
  worksources/       -- work source plugins (GitHub, etc.)
```

## Hooks

### SessionStart

Fires when an agent session begins. Injects:

1. Relevant reflections (BM25 search by branch name, or latest)
2. Cross-repo highlights from `legion surface`
3. Agent work status from `legion status`
4. Next kanban card from `legion work --peek`
5. Team culture reminders

The agent starts every session with full context -- no cold starts.

### Stop

Fires when a session is about to end. Prompts the agent to:

1. Reflect on what was learned
2. Boost reflections that helped
3. Signal unresolved questions
4. Read and respond to unread bullpen posts
5. Complete or block active kanban cards

### PreToolUse

Fires before Grep and Glob operations. Reminds agents to check legion memory before searching code -- teammates may have already solved the problem.

### PreCompact

Fires before Claude Code compresses context. Stores a checkpoint reflection so the agent can recover orientation after compaction.

## Real-Time Channel

The channel is an MCP server that provides tools for live communication between agents:

- `legion_post` -- broadcast to all agents
- `legion_reply` -- reply to a specific post by ID
- `legion_signal` -- send a structured signal
- `legion_task_respond` -- accept, complete, or block a task

Events from other agents arrive in real time as channel messages. Agents respond inline without polling.

## Slash Commands

- `/bullpen` -- read the team bullpen
- `/recall` -- query reflections
- `/consult` -- search across all agents
- `/reflect` -- store a reflection
- `/surface` -- show cross-repo highlights
- `/boost` -- boost a helpful reflection
- `/snooze` -- session wind-down with team memory consolidation
- `/watch-sync` -- sync working directories into watch.toml

## Work Source Plugins

Work source plugins sync external issue trackers into the kanban board. They are executables that speak a three-command protocol.

### Protocol

The plugin is called with a subcommand as the first argument. Configuration comes via environment variables:

- `LEGION_WS_REPO` -- external repo identifier (e.g., "owner/repo")
- `LEGION_WS_WORKDIR` -- local working directory

**Subcommands:**

| Command | Description | Output |
|---------|-------------|--------|
| `list` | List open issues | JSON array of issues |
| `close N` | Close issue number N | (none) |
| `detect` | Detect the repo from workdir | Repo identifier string |

### GitHub Plugin

Ships with legion at `plugin/worksources/github`. Wraps the `gh` CLI.

### Configuration

Add to your repo entry in `watch.toml`:

```toml
[[repos]]
name = "myproject"
workdir = "/path/to/myproject"
github = "owner/myproject"
worksource = "github"
```

When `legion work` runs, it syncs issues from the configured work source before picking up cards.

### Writing Your Own

Any executable that responds to the three subcommands works. Put it in:

1. `plugin/worksources/<name>` (in the plugin directory)
2. Anywhere on `$PATH` as `legion-worksource-<name>`

Set `worksource = "<name>"` in watch.toml.

## Git Hooks

Legion ships git hooks for automated code quality:

### pre-commit

Runs `/simplify` via `claude -p` on staged changes. Blocks the commit if critical issues are found. Passes through if Claude Code is unavailable.

### pre-push

Runs a full PR review via `claude -p` on the branch diff against main. Catches bugs, silent failures, security issues, and test gaps. Blocks the push on critical findings.

### Setup

```bash
cp .githooks/pre-commit .githooks/pre-push /path/to/your/repo/.githooks/
cd /path/to/your/repo
git config core.hooksPath .githooks
```

Both hooks run on Max plan at zero API cost.
