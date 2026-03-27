import { useCallback, useEffect, useState } from "react";
import { Container } from "@/src/components/ui/container";
import { Separator } from "@/src/components/ui/separator";
import { Tabs } from "@/src/components/ui/tabs";
import { StatusBar } from "@/src/components/StatusBar";
import { AgentStatusRow } from "@/src/components/AgentStatusRow";
import { KanbanBoard } from "@/src/components/KanbanBoard";
import {
  deriveAgentStatus,
  type AgentStatus,
} from "@/src/hooks/useAgentStatus";
import { useLegion } from "@/src/services";
import type { AgentInfo, Task } from "@/src/services/types";

export function App() {
  const legion = useLegion();
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [agentFilter, setAgentFilter] = useState<string | null>(null);

  const agentStatuses: AgentStatus[] = deriveAgentStatus(agents, tasks);

  const fetchTasks = useCallback(() => {
    legion.getTasks().then(setTasks).catch(console.error);
  }, [legion]);

  useEffect(() => {
    legion.getAgents().then(setAgents).catch(console.error);
    fetchTasks();
  }, [legion, fetchTasks]);

  useEffect(() => {
    return legion.subscribe({
      onAgents: setAgents,
      onTasks: setTasks,
    });
  }, [legion]);

  const handleFilterAgent = useCallback((repo: string | null) => {
    setAgentFilter(repo);
  }, []);

  return (
    <Container as="main" size="6xl" padding="6" gap="4">
      <span className="text-lg font-semibold tracking-tight">legion</span>

      <StatusBar agents={agentStatuses} />
      <AgentStatusRow
        agents={agentStatuses}
        activeFilter={agentFilter}
        onFilterAgent={handleFilterAgent}
      />

      <Separator />

      <Tabs defaultValue="tasks">
        <Tabs.List>
          <Tabs.Trigger value="tasks">Tasks</Tabs.Trigger>
          <Tabs.Trigger value="feed">Feed</Tabs.Trigger>
          <Tabs.Trigger value="signals">Signals</Tabs.Trigger>
          <Tabs.Trigger value="stats">Stats</Tabs.Trigger>
          <Tabs.Trigger value="chat">Chat</Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="tasks">
          <KanbanBoard
            tasks={tasks}
            agentFilter={agentFilter}
            onTasksChanged={fetchTasks}
          />
        </Tabs.Content>

        <Tabs.Content value="feed">
          Feed view -- bullpen, broadcast bar coming in #77
        </Tabs.Content>

        <Tabs.Content value="signals">
          Signals view -- coming soon
        </Tabs.Content>

        <Tabs.Content value="stats">
          Stats view -- coming soon
        </Tabs.Content>

        <Tabs.Content value="chat">
          Chat view -- coming soon
        </Tabs.Content>
      </Tabs>
    </Container>
  );
}
