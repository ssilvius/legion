import type { LegionService } from "./interface";
import type {
  AgentInfo,
  DoneResult,
  EventHandler,
  FeedItem,
  Schedule,
  SignalItem,
  StatusItem,
  StatusOutput,
  Task,
  Unsubscribe,
} from "./types";

export class HttpAdapter implements LegionService {
  private baseUrl: string;

  constructor(baseUrl: string = "") {
    this.baseUrl = baseUrl;
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`);
    if (!response.ok) {
      const body = await response.json().catch(() => ({}));
      throw new Error(
        (body as Record<string, string>).error || `HTTP ${response.status}`,
      );
    }
    return response.json() as Promise<T>;
  }

  private async postJson<T>(
    path: string,
    body?: Record<string, unknown>,
  ): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: body ? { "Content-Type": "application/json" } : undefined,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!response.ok) {
      const data = await response.json().catch(() => ({}));
      throw new Error(
        (data as Record<string, string>).error || `HTTP ${response.status}`,
      );
    }
    return response.json() as Promise<T>;
  }

  // Tasks

  async getTasks(): Promise<Task[]> {
    return this.get("/api/tasks");
  }

  async acceptTask(id: string): Promise<void> {
    await this.postJson(`/api/tasks/${id}/accept`);
  }

  async completeTask(id: string, note?: string): Promise<void> {
    await this.postJson(`/api/tasks/${id}/done`, note ? { note } : undefined);
  }

  async blockTask(id: string, reason?: string): Promise<void> {
    await this.postJson(
      `/api/tasks/${id}/block`,
      reason ? { note: reason } : undefined,
    );
  }

  async unblockTask(id: string): Promise<void> {
    await this.postJson(`/api/tasks/${id}/unblock`);
  }

  async createTask(
    from: string,
    to: string,
    text: string,
    priority: string,
    context?: string,
  ): Promise<Task> {
    return this.postJson("/api/tasks/create", {
      from,
      to,
      text,
      priority,
      context: context ?? null,
    });
  }

  // Status

  async getStatus(repo: string): Promise<StatusOutput> {
    return this.get(`/api/status?repo=${encodeURIComponent(repo)}`);
  }

  async getNeeds(repo: string): Promise<StatusItem[]> {
    return this.get(`/api/needs?repo=${encodeURIComponent(repo)}`);
  }

  async done(repo: string, text: string): Promise<DoneResult> {
    return this.postJson("/api/done", { repo, text });
  }

  // Feed

  async getBullpen(offset = 0, limit = 50): Promise<FeedItem[]> {
    return this.get(`/api/feed?offset=${offset}&limit=${limit}`);
  }

  async post(repo: string, text: string): Promise<void> {
    await this.postJson("/api/post", { repo, text });
  }

  async boost(id: string): Promise<void> {
    await this.postJson(`/api/boost/${id}`);
  }

  // Agents

  async getAgents(): Promise<AgentInfo[]> {
    return this.get("/api/agents");
  }

  // Signals

  async getSignals(): Promise<SignalItem[]> {
    return this.get("/api/signals");
  }

  // Schedules

  async getSchedules(): Promise<Schedule[]> {
    return this.get("/api/schedules");
  }

  async toggleSchedule(id: string): Promise<void> {
    await this.postJson(`/api/schedules/${id}/toggle`);
  }

  // Real-time events via SSE

  subscribe(handler: EventHandler): Unsubscribe {
    let retryDelay = 1000;
    const maxRetryDelay = 30000;
    let source: EventSource | null = null;
    let stopped = false;

    const connect = () => {
      if (stopped) return;

      source = new EventSource(`${this.baseUrl}/sse`);

      source.onopen = () => {
        retryDelay = 1000;
      };

      source.addEventListener("agents", (event) => {
        try {
          const agents: AgentInfo[] = JSON.parse(event.data);
          handler.onAgents?.(agents);
        } catch {
          // ignore parse errors
        }
      });

      source.addEventListener("feed", (event) => {
        try {
          const feed: FeedItem[] = JSON.parse(event.data);
          handler.onFeed?.(feed);
        } catch {
          // ignore parse errors
        }
      });

      source.addEventListener("tasks", (event) => {
        try {
          const tasks: Task[] = JSON.parse(event.data);
          handler.onTasks?.(tasks);
        } catch {
          // ignore parse errors
        }
      });

      source.onerror = () => {
        source?.close();
        if (!stopped) {
          setTimeout(connect, retryDelay);
          retryDelay = Math.min(retryDelay * 2, maxRetryDelay);
        }
      };
    };

    connect();

    return () => {
      stopped = true;
      source?.close();
    };
  }
}
