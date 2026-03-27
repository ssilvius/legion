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

export interface LegionService {
  // Tasks
  getTasks(): Promise<Task[]>;
  acceptTask(id: string): Promise<void>;
  completeTask(id: string, note?: string): Promise<void>;
  blockTask(id: string, reason?: string): Promise<void>;
  unblockTask(id: string): Promise<void>;
  createTask(
    from: string,
    to: string,
    text: string,
    priority: string,
    context?: string,
  ): Promise<Task>;

  // Status
  getStatus(repo: string): Promise<StatusOutput>;
  getNeeds(repo: string): Promise<StatusItem[]>;
  done(repo: string, text: string): Promise<DoneResult>;

  // Feed
  getBullpen(offset?: number, limit?: number): Promise<FeedItem[]>;
  post(repo: string, text: string): Promise<void>;
  boost(id: string): Promise<void>;

  // Agents
  getAgents(): Promise<AgentInfo[]>;

  // Signals
  getSignals(): Promise<SignalItem[]>;

  // Schedules
  getSchedules(): Promise<Schedule[]>;
  toggleSchedule(id: string): Promise<void>;

  // Real-time events
  subscribe(handler: EventHandler): Unsubscribe;
}
