import type { LegionEvent } from "./types.js";

// Fakechat mode: HTTP server that accepts POST /inject to simulate events.
// Usage: curl -X POST localhost:$FAKECHAT_PORT/inject -d '{"type":"signal","from":"kelex",...}'
export async function startFakechat(
  onEvent: (event: LegionEvent) => void
): Promise<void> {
  const port = parseInt(process.env.FAKECHAT_PORT || "3132", 10);

  Bun.serve({
    port,
    async fetch(req) {
      const url = new URL(req.url);

      if (req.method === "POST" && url.pathname === "/inject") {
        try {
          const event = (await req.json()) as LegionEvent;
          onEvent(event);
          return new Response("injected", { status: 200 });
        } catch {
          return new Response("invalid json", { status: 400 });
        }
      }

      return new Response("POST /inject to simulate events", {
        status: 200,
      });
    },
  });

  console.error(`[legion-channel] fakechat listening on port ${port}`);
}
