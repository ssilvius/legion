import type { Server } from "@modelcontextprotocol/sdk/server/index.js";
import {
  ListToolsRequestSchema,
  CallToolRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

interface ToolConfig {
  repo: string;
  port: number;
}

function resolveRepo(config: ToolConfig): string {
  if (config.repo !== "unknown") return config.repo;
  return (
    process.env.LEGION_REPO ||
    process.env.CLAUDE_CWD?.split("/").pop() ||
    process.cwd().split("/").pop() ||
    "unknown"
  );
}

async function httpPost(
  port: number,
  path: string,
  body: unknown
): Promise<{ ok: boolean; data: unknown }> {
  const res = await fetch(`http://localhost:${port}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  const data = await res.json();
  return { ok: res.ok, data };
}

async function runLegionCLI(
  args: string[]
): Promise<{ ok: boolean; output: string }> {
  const proc = Bun.spawn(["legion", ...args], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const code = await proc.exited;
  return { ok: code === 0, output: stdout || stderr };
}

const TOOLS = [
  {
    name: "legion_post",
    description:
      "Post a message to the Legion team bullpen. All agents will see it.",
    inputSchema: {
      type: "object" as const,
      properties: {
        text: {
          type: "string" as const,
          description: "The message to post",
        },
      },
      required: ["text"],
    },
  },
  {
    name: "legion_reply",
    description: "Reply to a specific bullpen post or signal by ID.",
    inputSchema: {
      type: "object" as const,
      properties: {
        to: {
          type: "string" as const,
          description: "The post/signal ID to reply to",
        },
        text: {
          type: "string" as const,
          description: "Your reply",
        },
      },
      required: ["to", "text"],
    },
  },
  {
    name: "legion_signal",
    description:
      "Send a structured signal to another agent (@recipient verb:status).",
    inputSchema: {
      type: "object" as const,
      properties: {
        to: {
          type: "string" as const,
          description: 'Recipient agent name, or "all"',
        },
        verb: {
          type: "string" as const,
          description:
            "Action: review, request, announce, question, answer, etc.",
        },
        status: {
          type: "string" as const,
          description: "Status: approved, help, blocked, etc.",
        },
        note: {
          type: "string" as const,
          description: "Free-text note",
        },
      },
      required: ["to", "verb"],
    },
  },
  {
    name: "legion_task_respond",
    description:
      "Respond to a task assigned to you. Accept, complete, or block it.",
    inputSchema: {
      type: "object" as const,
      properties: {
        id: { type: "string" as const, description: "Task ID" },
        action: {
          type: "string" as const,
          enum: ["accept", "done", "block"],
          description: "What to do with the task",
        },
        note: {
          type: "string" as const,
          description: "Optional note (completion summary or block reason)",
        },
      },
      required: ["id", "action"],
    },
  },
];

export function registerTools(
  server: Server,
  config: ToolConfig
): void {
  server.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: TOOLS,
  }));

  server.setRequestHandler(CallToolRequestSchema, async (req) => {
    const args = req.params.arguments as Record<string, string>;

    switch (req.params.name) {
      case "legion_post": {
        const result = await httpPost(config.port, "/api/post", {
          repo: resolveRepo(config),
          text: args.text,
        });
        return {
          content: [
            {
              type: "text" as const,
              text: result.ok ? "posted" : `post failed: ${JSON.stringify(result.data)}`,
            },
          ],
        };
      }

      case "legion_reply": {
        const replyText = `re:${args.to} -- ${args.text}`;
        const result = await httpPost(config.port, "/api/post", {
          repo: resolveRepo(config),
          text: replyText,
        });
        return {
          content: [
            {
              type: "text" as const,
              text: result.ok ? "replied" : `reply failed: ${JSON.stringify(result.data)}`,
            },
          ],
        };
      }

      case "legion_signal": {
        const statusPart = args.status ? `:${args.status}` : "";
        const notePart = args.note ? ` -- ${args.note}` : "";
        const signalText = `@${args.to} ${args.verb}${statusPart}${notePart}`;
        const result = await httpPost(config.port, "/api/post", {
          repo: resolveRepo(config),
          text: signalText,
        });
        return {
          content: [
            {
              type: "text" as const,
              text: result.ok ? "signaled" : `signal failed: ${JSON.stringify(result.data)}`,
            },
          ],
        };
      }

      case "legion_task_respond": {
        const cliArgs = ["task", args.action, "--id", args.id];
        if (args.note) {
          if (args.action === "done") {
            cliArgs.push("--note", args.note);
          } else if (args.action === "block") {
            cliArgs.push("--reason", args.note);
          }
        }
        const result = await runLegionCLI(cliArgs);
        return {
          content: [
            {
              type: "text" as const,
              text: result.ok
                ? `task ${args.action}: ${args.id}`
                : `task ${args.action} failed: ${result.output}`,
            },
          ],
        };
      }

      default:
        throw new Error(`unknown tool: ${req.params.name}`);
    }
  });
}
