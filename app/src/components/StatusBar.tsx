import { Badge } from "@/src/components/ui/badge";
import { summarizeAgents, type AgentStatus } from "@/src/hooks/useAgentStatus";

interface StatusBarProps {
  agents: AgentStatus[];
}

export function StatusBar({ agents }: StatusBarProps) {
  const summary = summarizeAgents(agents);

  if (agents.length === 0) {
    return (
      <Badge variant="secondary">
        No agents connected
      </Badge>
    );
  }

  if (!summary.needsAttention) {
    return (
      <Badge variant="default">
        All clear -- {summary.working} working
        {summary.pending > 0 ? `, ${summary.pending} pending` : ""}
      </Badge>
    );
  }

  return (
    <Badge variant="destructive">
      Attention needed -- {summary.blocked} blocked, {summary.idle} idle
      {summary.working > 0 ? `, ${summary.working} working` : ""}
    </Badge>
  );
}
