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
  /** A thread between notes: the other note and the words it quotes there. */
  elsewhere?: { noteTitle: string; quote: string } | null;
}

/** Words from another note that share the subject of the paragraph just read (relate). */
export interface Passage {
  /** The other note's id: echoed back, never put in the prompt. */
  note: string;
  noteTitle: string;
  /** That note's created_at as a date: "Thursday 3 September 2026". */
  written: string;
  /** Its updated_at, the same way. */
  edited: string;
  /** The nearest heading above the passage. */
  heading: string | null;
  /** The passage, at most 600 characters. */
  text: string;
}

export type Relation = "contradicts" | "depends";

/** The other end of a thread between notes, as it is now (reply). */
export interface ElsewhereEnd {
  noteTitle: string;
  /** As Passage.edited; empty when the note is gone. */
  edited: string;
  /** The other end's words now. */
  quote: string;
  /** Its paragraph now (at most 600 characters); null when the note is gone. */
  text: string | null;
  relation: Relation;
  /** The other note was deleted. */
  gone?: boolean;
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
  /** Look things up on the web (Settings › Model): the tasks that can use a source search. */
  web?: boolean;
  model?: string;
  effort?: "low" | "medium" | "high" | "xhigh" | "max";
}

export type Task =
  // `through`: a read made by Read it through, which reads a finished draft once.
  | { task: "read"; ctx: Context; target: string; through?: boolean }
  | { task: "probe"; ctx: Context; target: string }
  | { task: "ask"; ctx: Context; target: string; question: string }
  | { task: "answer"; ctx: Context; target: string }
  | { task: "sketch"; ctx: Context; target: string }
  | { task: "draw"; ctx: Context; quote: string; thought: string; request?: string }
  | { task: "reply"; ctx: Context; quote: string; messages: ThreadMessage[]; message: string; elsewhere?: ElsewhereEnd }
  | { task: "chat"; ctx: Context; history: ThreadMessage[]; message: string; near: string | null }
  | { task: "learn"; aboutMe: string | null; message: string }
  | { task: "relate"; ctx: Context; target: string; before: string[]; passages: Passage[] };

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
  /** Token counts the runtime reports, to measure what a task costs. */
  inputTokens?: number;
  outputTokens?: number;
  cacheReadTokens?: number;
  cacheWriteTokens?: number;
  /** Web searches the call made. */
  searches?: number;
}
