import { useCallback, useEffect, useState } from "react";
import { Container } from "@/src/components/ui/container";
import { Badge } from "@/src/components/ui/badge";
import { Button } from "@/src/components/ui/button";
import { Card } from "@/src/components/ui/card";
import { Separator } from "@/src/components/ui/separator";
import { BroadcastBar } from "@/src/components/BroadcastBar";
import { useLegion } from "@/src/services";
import type { FeedItem } from "@/src/services/types";
import { relativeTime } from "@/src/lib/time";

type FeedFilter = "all" | "signals" | "musings";

export function FeedView() {
  const legion = useLegion();
  const [items, setItems] = useState<FeedItem[]>([]);
  const [filter, setFilter] = useState<FeedFilter>("all");

  useEffect(() => {
    legion.getBullpen().then(setItems).catch(console.error);
  }, [legion]);

  useEffect(() => {
    return legion.subscribe({
      onFeed: setItems,
    });
  }, [legion]);

  const handleBoost = useCallback(
    (id: string) => {
      legion.boost(id).catch(console.error);
    },
    [legion],
  );

  const filtered = items.filter((item) => {
    if (filter === "signals") return item.is_signal;
    if (filter === "musings") return !item.is_signal;
    return true;
  });

  return (
    <Container as="section" gap="4">
      <BroadcastBar />

      <Separator />

      <Container as="div" gap="2">
        <Button
          size="sm"
          variant={filter === "all" ? "default" : "secondary"}
          onClick={() => setFilter("all")}
        >
          all
        </Button>
        <Button
          size="sm"
          variant={filter === "signals" ? "default" : "secondary"}
          onClick={() => setFilter("signals")}
        >
          signals
        </Button>
        <Button
          size="sm"
          variant={filter === "musings" ? "default" : "secondary"}
          onClick={() => setFilter("musings")}
        >
          musings
        </Button>
      </Container>

      {filtered.length === 0 ? (
        <p className="text-muted-foreground text-sm">No posts</p>
      ) : (
        filtered.map((item) => (
          <Card key={item.id}>
            <Card.Header>
              <Card.Title>
                <Badge variant="default">{item.repo}</Badge>
                {item.is_signal ? (
                  <Badge variant="secondary">signal</Badge>
                ) : null}
              </Card.Title>
              <Card.Description>{relativeTime(item.created_at)}</Card.Description>
            </Card.Header>
            <Card.Content>
              <p>{item.text}</p>
            </Card.Content>
            <Card.Footer>
              <Button size="sm" variant="outline" onClick={() => handleBoost(item.id)}>
                +boost
              </Button>
            </Card.Footer>
          </Card>
        ))
      )}
    </Container>
  );
}
