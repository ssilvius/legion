// Types matching the Rust API response shapes exactly

export interface Task {
  id: string;
  from_repo: string;
  to_repo: string;
  text: string;
  context: string | null;
  priority: string;
  status: string;
  note: string | null;
  created_at: string;
  updated_at: string;
}

export interface StatusItem {
  category: string;
  text: string;
  from: string;
  age: string;
}

export interface StatusOutput {
  repo: string;
  your_work: StatusItem[];
  team_needs: StatusItem[];
  what_changed: StatusItem[];
}

export interface FeedItem {
  id: string;
  repo: string;
  text: string;
  created_at: string;
  is_signal: boolean;
}

export interface AgentInfo {
  repo: string;
  unread: number;
  reflection_count: number;
  boost_sum: number;
  team_post_count: number;
  last_activity: string;
}

export interface Schedule {
  id: string;
  name: string;
  cron: string;
  command: string;
  repo: string;
  enabled: boolean;
  last_run: string | null;
  next_run: string;
  created_at: string;
  active_start: string | null;
  active_end: string | null;
}

export interface SignalItem {
  id: string;
  from_repo: string;
  to: string;
  verb: string;
  status: string | null;
  text: string;
  created_at: string;
}

export interface DoneResult {
  announcement: string;
  notified: string[];
}

export type EventType = "agents" | "feed" | "tasks" | "ping";

export interface EventHandler {
  onAgents?: (agents: AgentInfo[]) => void;
  onFeed?: (feed: FeedItem[]) => void;
  onTasks?: (tasks: Task[]) => void;
}

export type Unsubscribe = () => void;
