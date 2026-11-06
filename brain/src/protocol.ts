// Wire protocol between the Inkling app (Rust) and this sidecar.
// One JSON object per line on stdin (requests) and stdout (events).
// Logs go to stderr so stdout stays machine-readable.

export type Voice = "plain" | "warm" | "blunt" | "riddle";
export type Frequency = "rarely" | "when_it_matters" | "often";

export interface Mind {
  name: string;
  description: string;
  howItThinks: string;
  voice: Voice;
  frequency: Frequency;
}

export interface ThreadMessage {
  role: "mind" | "you";
  text: string;
}

export interface ThreadSummary {
  quote: string;
  messages: ThreadMessage[];
  resolved: boolean;
}

// A point the writer let stand: every mind is told it, so none raises it again.
export interface LetStandPoint {
  quote: string; // the words, as they were when let stand
  point: string; // the mind's first message: what it asked or corrected
  kind: "question" | "correction";
  mind: string; // the mind that raised it
  reason: string | null; // the writer's one line, if they gave one
}

export interface Context {
  noteTitle: string;
  noteText: string;
  brief: string | null;
  mind: Mind;
  aboutMe: string | null;
  threads: ThreadSummary[];
  now: string;
  letStand: LetStandPoint[];
}

// Both are optional on the wire: a missing or unknown value leaves the runtime's default.
export interface RunSettings {
  model?: string;
  effort?: "low" | "medium" | "high" | "xhigh" | "max";
}

export type Task =
  | { task: "read"; ctx: Context; target: string }
  | { task: "probe"; ctx: Context; target: string }
  | { task: "ask"; ctx: Context; target: string; question: string }
  | { task: "answer"; ctx: Context; target: string }
  | { task: "sketch"; ctx: Context; target: string }
  | { task: "draw"; ctx: Context; quote: string; thought: string; request?: string }
  | { task: "reply"; ctx: Context; quote: string; messages: ThreadMessage[]; message: string }
  | { task: "chat"; ctx: Context; history: ThreadMessage[]; message: string; near: string | null }
  | { task: "learn"; aboutMe: string | null; message: string };

export type Request =
  | { op: "run"; id: string; settings: RunSettings; input: Task }
  | { op: "cancel"; id: string }
  | { op: "status"; id: string }
  | { op: "shutdown" };

export type Event =
  | { ev: "ready"; provider: string; version: string }
  | { ev: "delta"; id: string; text: string }
  | { ev: "done"; id: string; data: unknown; usage?: Usage }
  | { ev: "error"; id: string; kind: ErrorKind; message: string }
  | { ev: "status"; id: string; data: unknown }
  | { ev: "limit"; status: string; resetsAt?: number; utilization?: number; kind?: string };

export type ErrorKind = "auth" | "rate_limit" | "overloaded" | "cancelled" | "bad_output" | "other";

export interface Usage {
  ms: number;
  model?: string;
  costUsd?: number;
}
