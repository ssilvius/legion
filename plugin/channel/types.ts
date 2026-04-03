// Events from Legion's existing /sse endpoint
export interface LegacyFeedItem {
  id: string;
  repo: string;
  text: string;
  created_at: string;
  is_signal: boolean;
}

export interface LegacyTask {
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

// Future per-repo SSE events (when /events/:repo is built)
export interface PostEvent {
  type: "post";
  id: string;
  from: string;
  text: string;
  timestamp: string;
  is_signal: boolean;
}

export interface SignalEvent {
  type: "signal";
  id: string;
  from: string;
  to: string;
  verb: string;
  status: string | null;
  note: string | null;
  timestamp: string;
}

export interface TaskEvent {
  type: "task";
  id: string;
  from: string;
  to: string;
  text: string;
  priority: string;
  status: string;
  context: string | null;
  timestamp: string;
}

export interface DiscordEvent {
  type: "discord";
  channel: string;
  author: string;
  text: string;
  timestamp: string;
}

export type LegionEvent = PostEvent | SignalEvent | TaskEvent | DiscordEvent;
