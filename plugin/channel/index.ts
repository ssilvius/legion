#!/usr/bin/env bun
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { connectSSE } from "./sse-client.js";
import { bridgeEvent } from "./event-bridge.js";
import { registerTools } from "./tools.js";
import { INSTRUCTIONS } from "./instructions.js";

const repo =
  process.env.LEGION_REPO ||
  process.env.CLAUDE_CWD?.split("/").pop() ||
  process.cwd().split("/").pop() ||
  "unknown";
const port = parseInt(process.env.LEGION_PORT || "3131", 10);
const fakechat = process.env.LEGION_FAKECHAT === "1";

const server = new Server(
  { name: "legion", version: "0.1.0" },
  {
    capabilities: {
      experimental: { "claude/channel": {} },
      tools: {},
    },
    instructions: INSTRUCTIONS,
  }
);

registerTools(server, { repo, port });

const transport = new StdioServerTransport();
await server.connect(transport);

// Write channel marker for hook coordination
const markerPath = `/tmp/legion-channel-${repo}`;
await Bun.write(markerPath, `${process.pid}`);

console.error(`[legion-channel] connected for repo: ${repo}, port: ${port}`);

if (fakechat) {
  const { startFakechat } = await import("./fakechat.js");
  startFakechat((event) => bridgeEvent(server, event, repo));
} else {
  connectSSE({
    port,
    repo,
    onEvent: (event) => bridgeEvent(server, event, repo),
    onConnect: () =>
      console.error("[legion-channel] SSE stream connected"),
  });
}
