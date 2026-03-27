import { useCallback } from "react";
import { Container } from "@/src/components/ui/container";
import { Grid } from "@/src/components/ui/grid";
import { Badge } from "@/src/components/ui/badge";
import { TaskCard } from "@/src/components/TaskCard";
import { useLegion } from "@/src/services";
import type { Task } from "@/src/services/types";

interface KanbanBoardProps {
  tasks: Task[];
  agentFilter: string | null;
  onTasksChanged: () => void;
}

const columns: { status: string; label: string }[] = [
  { status: "pending", label: "Pending" },
  { status: "accepted", label: "Accepted" },
  { status: "blocked", label: "Blocked" },
  { status: "done", label: "Done" },
];

export function KanbanBoard({
  tasks,
  agentFilter,
  onTasksChanged,
}: KanbanBoardProps) {
  const legion = useLegion();

  const filteredTasks = agentFilter
    ? tasks.filter((t) => t.to_repo === agentFilter)
    : tasks;

  const handleAccept = useCallback(
    (id: string) => {
      legion.acceptTask(id).then(onTasksChanged).catch(console.error);
    },
    [legion, onTasksChanged],
  );

  const handleDone = useCallback(
    (id: string) => {
      legion.completeTask(id).then(onTasksChanged).catch(console.error);
    },
    [legion, onTasksChanged],
  );

  const handleBlock = useCallback(
    (id: string) => {
      legion.blockTask(id).then(onTasksChanged).catch(console.error);
    },
    [legion, onTasksChanged],
  );

  const handleUnblock = useCallback(
    (id: string) => {
      legion.unblockTask(id).then(onTasksChanged).catch(console.error);
    },
    [legion, onTasksChanged],
  );

  return (
    <Grid preset="split">
      {columns.map((col) => {
        const colTasks = filteredTasks.filter((t) => t.status === col.status);
        return (
          <Container key={col.status} as="section" gap="3">
            <Container as="div">
              <Badge
                variant={
                  col.status === "blocked" && colTasks.length > 0
                    ? "destructive"
                    : "secondary"
                }
              >
                {col.label} ({colTasks.length})
              </Badge>
            </Container>
            {colTasks.length === 0 ? (
              <p className="text-muted-foreground text-sm">none</p>
            ) : (
              colTasks.map((task) => (
                <TaskCard
                  key={task.id}
                  task={task}
                  onAccept={handleAccept}
                  onDone={handleDone}
                  onBlock={handleBlock}
                  onUnblock={handleUnblock}
                />
              ))
            )}
          </Container>
        );
      })}
    </Grid>
  );
}
