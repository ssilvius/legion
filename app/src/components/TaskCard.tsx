import { Badge } from "@/src/components/ui/badge";
import { Button } from "@/src/components/ui/button";
import { Card } from "@/src/components/ui/card";
import type { Task } from "@/src/services/types";
import { relativeTime } from "@/src/lib/time";

interface TaskCardProps {
  task: Task;
  onAccept: (id: string) => void;
  onDone: (id: string) => void;
  onBlock: (id: string) => void;
  onUnblock: (id: string) => void;
}

const priorityVariant: Record<string, "default" | "secondary" | "destructive"> = {
  high: "destructive",
  med: "secondary",
  low: "default",
};

export function TaskCard({
  task,
  onAccept,
  onDone,
  onBlock,
  onUnblock,
}: TaskCardProps) {
  return (
    <Card>
      <Card.Header>
        <Card.Title>{task.to_repo}</Card.Title>
        <Card.Description>
          {task.from_repo} &rarr; {task.to_repo}
        </Card.Description>
      </Card.Header>
      <Card.Content>
        <p>{task.text}</p>
        {task.context ? (
          <p className="text-muted-foreground text-sm">{task.context}</p>
        ) : null}
        {task.status === "blocked" && task.note ? (
          <Badge variant="destructive">BLOCKED: {task.note}</Badge>
        ) : null}
      </Card.Content>
      <Card.Footer>
        <Badge variant={priorityVariant[task.priority] ?? "default"}>
          {task.priority}
        </Badge>
        <span className="text-muted-foreground text-sm">
          {relativeTime(task.created_at)}
        </span>
        {task.status === "pending" ? (
          <Button size="sm" onClick={() => onAccept(task.id)}>
            Accept
          </Button>
        ) : null}
        {task.status === "accepted" ? (
          <>
            <Button size="sm" variant="default" onClick={() => onDone(task.id)}>
              Done
            </Button>
            <Button
              size="sm"
              variant="destructive"
              onClick={() => onBlock(task.id)}
            >
              Block
            </Button>
          </>
        ) : null}
        {task.status === "blocked" ? (
          <Button size="sm" variant="secondary" onClick={() => onUnblock(task.id)}>
            Unblock
          </Button>
        ) : null}
      </Card.Footer>
    </Card>
  );
}
