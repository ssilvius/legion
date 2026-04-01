import { Badge } from "@/src/components/ui/badge";
import type { AgentState, AgentStatus } from "@/src/hooks/useAgentStatus";

interface AgentStatusRowProps {
  agents: AgentStatus[];
  activeFilter: string | null;
  onFilterAgent: (repo: string | null) => void;
}

const stateVariant: Record<AgentState, "default" | "secondary" | "destructive" | "outline"> = {
  working: "default",
  idle: "secondary",
  blocked: "destructive",
  pending: "outline",
};

const stateLabel: Record<AgentState, string> = {
  working: "working",
  idle: "idle",
  blocked: "blocked",
  pending: "pending",
};

export function AgentStatusRow({
  agents,
  activeFilter,
  onFilterAgent,
}: AgentStatusRowProps) {
  if (agents.length === 0) return null;

  return (
    <div role="group" aria-label="Agent status">
      {agents.map((agent) => (
        <Badge
          key={agent.repo}
          variant={
            activeFilter === agent.repo ? "default" : stateVariant[agent.state]
          }
          onClick={() =>
            onFilterAgent(activeFilter === agent.repo ? null : agent.repo)
          }
          style={{ cursor: "pointer" }}
        >
          {agent.repo}: {stateLabel[agent.state]}
          {agent.activeCount > 0 ? ` (${agent.activeCount})` : ""}
        </Badge>
      ))}
    </div>
  );
}
