// Claude provider: every call goes through the Claude Agent SDK, which drives the
// local Claude Code runtime and so uses the machine's Claude Code login
// (Pro/Max subscription) instead of an API key.

import { query, type Options, type SDKMessage } from "@anthropic-ai/claude-agent-sdk";
import { mkdirSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import type { ErrorKind, RunSettings, Usage } from "./protocol.ts";

export class ProviderError extends Error {
  constructor(public kind: ErrorKind, message: string) {
    super(message);
  }
}

export interface LimitInfo {
  status: string;
  resetsAt?: number;
  utilization?: number;
  kind?: string;
}

// Tests swap in a fake SDK; everything else uses the real one.
type QueryFn = typeof query;
let queryFn: QueryFn = query;
export function setQueryForTests(fn: QueryFn) {
  queryFn = fn;
}

// A neutral working directory so the runtime never picks up a project's files.
// It lives in the app's data folder, which INKLING_DATA_DIR moves (as it does
// for the app), so a test run never writes into the real notebook folder.
let workdir: string | undefined;
function workDir(): string {
  if (workdir) return workdir;
  const base = process.env.INKLING_DATA_DIR || join(homedir(), "Library", "Application Support", "Inkling");
  for (const dir of [join(base, "brain-cwd"), join(tmpdir(), "inkling-brain-cwd")]) {
    try {
      mkdirSync(dir, { recursive: true });
      return (workdir = dir);
    } catch {
      // Unwritable: try the next place rather than failing every request.
    }
  }
  return (workdir = tmpdir());
}

export function childEnv(source: Record<string, string | undefined> = process.env): Record<string, string | undefined> {
  const env: Record<string, string | undefined> = { ...source };
  // Use the subscription login even if an API key or token happens to be exported.
  if (source.INKLING_USE_API_KEY !== "1") {
    delete env.ANTHROPIC_API_KEY;
    delete env.ANTHROPIC_AUTH_TOKEN;
  }
  env.CLAUDE_AGENT_SDK_CLIENT_APP = "inkling/0.1.0";
  return env;
}

const STRUCTURED_TURNS = 3;

function baseOptions(system: string, settings: RunSettings, signal: AbortController): Options {
  const opts: Options = {
    systemPrompt: system,
    tools: [],
    settingSources: [],
    maxTurns: 1,
    persistSession: false,
    model: settings.model,
    effort: settings.effort,
    cwd: workDir(),
    env: childEnv(),
    abortController: signal,
  };
  if (process.env.INKLING_CLAUDE_PATH) opts.pathToClaudeCodeExecutable = process.env.INKLING_CLAUDE_PATH;
  return opts;
}

/** Best guess at the kind of a failure from its message alone. */
export function classify(message: string): ErrorKind {
  const m = message.toLowerCase();
  // Limits first: their messages often also say "log in" or "account".
  if (/\b429\b|rate.?limit|usage limit|limit reached|hit your limit|reached your limit|quota|too many requests/.test(m)) {
    return "rate_limit";
  }
  if (/\b529\b|overloaded|over capacity/.test(m)) return "overloaded";
  if (/\b40[13]\b|authenticat|unauthori[sz]ed|not logged in|\blog ?in\b|\/login|oauth|credential|api key|forbidden/.test(m)) {
    return "auth";
  }
  return "other";
}

// The runtime tags a failed model call on the assistant message it synthesises.
const ASSISTANT_ERROR_KIND: Record<string, ErrorKind> = {
  authentication_failed: "auth",
  oauth_org_not_allowed: "auth",
  account_on_hold: "auth",
  verification_required: "auth",
  cloud_credential_error: "auth",
  billing_error: "rate_limit",
  rate_limit: "rate_limit",
  overloaded: "overloaded",
  max_output_tokens: "bad_output",
};

interface Drained {
  text: string;
  structured: unknown;
  usage: Usage;
}

async function drain(
  q: AsyncIterable<SDKMessage>,
  started: number,
  structured: boolean,
  onDelta?: (t: string) => void,
  onLimit?: (l: LimitInfo) => void,
): Promise<Drained> {
  let model: string | undefined;
  let streamed = "";
  let assistantError: string | undefined;
  let limitRejected = false;
  for await (const m of q) {
    switch (m.type) {
      case "system":
        if ((m as any).subtype === "init") model = (m as any).model;
        break;
      case "assistant": {
        const err = (m as any).error;
        if (typeof err === "string") assistantError = err;
        break;
      }
      case "stream_event": {
        const ev = (m as any).event;
        if (ev?.type === "content_block_delta" && ev.delta?.type === "text_delta" && onDelta) {
          streamed += ev.delta.text;
          onDelta(ev.delta.text);
        }
        break;
      }
      case "rate_limit_event": {
        const info = (m as any).rate_limit_info ?? {};
        if (info.status === "rejected") limitRejected = true;
        onLimit?.({
          status: info.status,
          resetsAt: info.resetsAt,
          utilization: info.utilization,
          kind: info.rateLimitType,
        });
        break;
      }
      case "result": {
        const r = m as any;
        const usage: Usage = { ms: Date.now() - started, model, costUsd: r.total_cost_usd };
        if (r.subtype !== "success" || r.is_error) {
          const errors = Array.isArray(r.errors) ? r.errors.filter(Boolean).join("; ") : "";
          const msg = String(errors || r.result || r.subtype || "the model returned an error");
          throw new ProviderError(resultKind(r.subtype, assistantError, limitRejected, structured, msg), msg);
        }
        return { text: r.result ?? streamed, structured: r.structured_output, usage };
      }
    }
  }
  throw new ProviderError("other", "the model stopped without a result");
}

function resultKind(
  subtype: string | undefined,
  assistantError: string | undefined,
  limitRejected: boolean,
  structured: boolean,
  message: string,
): ErrorKind {
  if (subtype === "error_max_structured_output_retries") return "bad_output";
  // With one turn and no tools, running out of turns means the structured answer never came.
  if (subtype === "error_max_turns" && structured) return "bad_output";
  if (assistantError && ASSISTANT_ERROR_KIND[assistantError]) return ASSISTANT_ERROR_KIND[assistantError];
  if (limitRejected) return "rate_limit";
  return classify(message);
}

/** Free-form text, streamed to `onDelta` as it arrives. */
export async function runText(
  system: string,
  prompt: string,
  settings: RunSettings,
  signal: AbortController,
  onDelta?: (t: string) => void,
  onLimit?: (l: LimitInfo) => void,
): Promise<{ text: string; usage: Usage }> {
  const started = Date.now();
  try {
    const opts = baseOptions(system, settings, signal);
    opts.includePartialMessages = Boolean(onDelta);
    const out = await drain(queryFn({ prompt, options: opts }), started, false, onDelta, onLimit);
    return { text: out.text.trim(), usage: out.usage };
  } catch (e) {
    throw asProviderError(e, signal);
  }
}

/** A JSON object validated against `schema` by the runtime. */
export async function runStructured<T>(
  system: string,
  prompt: string,
  schema: object,
  settings: RunSettings,
  signal: AbortController,
  onLimit?: (l: LimitInfo) => void,
): Promise<{ data: T; usage: Usage }> {
  const started = Date.now();
  try {
    const opts = baseOptions(system, settings, signal);
    opts.outputFormat = { type: "json_schema", schema: schema as Record<string, unknown> };
    // The answer comes back as a StructuredOutput tool call, and when it is missing
    // or fails the schema the runtime asks again in another turn. With one turn that
    // retry is impossible and the call fails with "Reached maximum number of turns".
    // There are no other tools, so extra turns can only be those retries.
    opts.maxTurns = STRUCTURED_TURNS;
    const out = await drain(queryFn({ prompt, options: opts }), started, true, undefined, onLimit);
    let data = out.structured as T | undefined;
    if (data === undefined || data === null) {
      // Fall back to parsing the text, in case the runtime returned it there.
      data = parseLooseJson<T>(out.text);
    }
    return { data, usage: out.usage };
  } catch (e) {
    throw asProviderError(e, signal);
  }
}

function asProviderError(e: unknown, signal: AbortController): ProviderError {
  if (signal.signal.aborted) return new ProviderError("cancelled", "cancelled");
  if (e instanceof ProviderError) return e;
  const msg = e instanceof Error ? e.message : String(e);
  return new ProviderError(classify(msg), msg);
}

export function parseLooseJson<T>(text: string): T {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start < 0 || end <= start) throw new ProviderError("bad_output", "expected a JSON object");
  try {
    return JSON.parse(text.slice(start, end + 1)) as T;
  } catch {
    throw new ProviderError("bad_output", "the model's JSON did not parse");
  }
}

/**
 * Who is signed in and which models Claude Code offers (the SDK's `supportedModels()`,
 * passed on as it comes: value, displayName, description, resolvedModel, effort levels),
 * without spending a model turn. Always settles within `timeoutMs`.
 */
export async function accountStatus(timeoutMs = Number(process.env.INKLING_STATUS_TIMEOUT_MS) || 20000): Promise<unknown> {
  const signal = new AbortController();
  // A prompt stream that never yields keeps the session open without sending a turn.
  let release: () => void = () => {};
  const idle = new Promise<void>((r) => (release = r));
  async function* never() {
    await idle;
  }
  let timer: ReturnType<typeof setTimeout> | undefined;
  // Aborting the session does not always settle a pending control request, so
  // the timeout rejects on its own instead of relying on the abort.
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new ProviderError("other", "Claude Code did not say who is signed in (timed out)")), timeoutMs);
  });
  let q: ReturnType<QueryFn> | undefined;
  try {
    // No model or effort: the list should not depend on one, and nothing is sent.
    q = queryFn({
      prompt: never() as any,
      options: baseOptions("", {}, signal),
    });
    const info = await Promise.race([q.accountInfo(), timeout]);
    let models: unknown = null;
    try {
      models = await Promise.race([q.supportedModels(), timeout]);
    } catch {
      models = null;
    }
    return { account: info, models };
  } catch (e) {
    if (e instanceof ProviderError) throw e;
    const msg = e instanceof Error ? e.message : String(e);
    throw new ProviderError(classify(msg), msg);
  } finally {
    clearTimeout(timer);
    release();
    signal.abort();
    try {
      q?.close();
    } catch {
      // Already closed.
    }
  }
}
