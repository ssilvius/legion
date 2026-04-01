import type { AgentInfo, Task } from "@/src/services/types";

export type AgentState = "working" | "idle" | "blocked" | "pending";

export interface AgentStatus {
  repo: string;
  state: AgentState;
  activeCount: number;
  pendingCount: number;
  blockedCount: number;
}

function isPathLike(name: string): boolean {
  return name.includes("/") || name.startsWith("agent-");
}

export function deriveAgentStatus(
  agents: AgentInfo[],
  tasks: Task[],
): AgentStatus[] {
  return agents
    .filter((a) => !isPathLike(a.repo))
    .map((agent) => {
      const agentTasks = tasks.filter((t) => t.to_repo === agent.repo);
      const accepted = agentTasks.filter(
        (t) => t.status === "accepted",
      ).length;
      const pending = agentTasks.filter((t) => t.status === "pending").length;
      const blocked = agentTasks.filter((t) => t.status === "blocked").length;

      let state: AgentState;
      if (accepted > 0) {
        state = "working";
      } else if (blocked > 0 && accepted === 0) {
        state = "blocked";
      } else if (pending > 0) {
        state = "pending";
      } else {
        state = "idle";
      }

      return {
        repo: agent.repo,
        state,
        activeCount: accepted,
        pendingCount: pending,
        blockedCount: blocked,
      };
    })
    .sort((a, b) => a.repo.localeCompare(b.repo));
}

export function summarizeAgents(statuses: AgentStatus[]): {
  working: number;
  idle: number;
  blocked: number;
  pending: number;
  needsAttention: boolean;
} {
  let working = 0;
  let idle = 0;
  let blocked = 0;
  let pending = 0;

  for (const s of statuses) {
    switch (s.state) {
      case "working":
        working++;
        break;
      case "idle":
        idle++;
        break;
      case "blocked":
        blocked++;
        break;
      case "pending":
        pending++;
        break;
    }
  }

  return {
    working,
    idle,
    blocked,
    pending,
    needsAttention: blocked > 0 || idle > 0,
  };
}
