import { Container } from "@/src/components/ui/container";
import { Separator } from "@/src/components/ui/separator";
import { Tabs } from "@/src/components/ui/tabs";

export function App() {
  return (
    <Container as="main" size="6xl" padding="6" gap="6">
      <span className="text-lg font-semibold tracking-tight">legion</span>

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
          Tasks view -- status bar, agent row, triage, kanban coming in #74-#76
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
