// Claude provider: every call goes through the Claude Agent SDK, which drives the
// local Claude Code runtime and so uses the machine's Claude Code login
// (Pro/Max subscription) instead of an API key.

import { query, type Options, type SDKMessage } from "@anthropic-ai/claude-agent-sdk";
import { accessSync, constants, mkdirSync, statSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { delimiter, isAbsolute, join } from "node:path";
import type { ErrorKind, RunSettings, Usage } from "./protocol.ts";
import { foundIn, type Found } from "./web.ts";

export class ProviderError extends Error {
  constructor(public kind: ErrorKind, message: string) {
    super(message);
  }
}

/**
 * A call that could look things up ran out of turns (`WEB_TURNS`) before answering. The
 * brain answers again without the web, so it ends as an answer (a question, for a
 * correction it could not back), never as an error. `spentUsd` is what it cost.
 */
export class WebSpent extends ProviderError {
  constructor(message: string, public spentUsd: number) {
    super("other", message);
  }
}

export interface LimitInfo {
  status: string;
  resetsAt?: number;
  utilization?: number;
  kind?: string;
}

// Finder launches without the shell's PATH. Keep the native install's symlink
// so new sessions pick up the executable Claude Code updated it to.
export function installedClaude(env = process.env, home = homedir()): string {
  if (env.INKLING_CLAUDE_PATH) return env.INKLING_CLAUDE_PATH;
  const candidates = [
    join(home, ".local/bin/claude"),
    ...(env.PATH ?? "").split(delimiter).filter(isAbsolute).map(dir => join(dir, "claude")),
    "/opt/homebrew/bin/claude",
    "/usr/local/bin/claude",
  ];
  for (const path of candidates) {
    try {
      if (!statSync(path).isFile()) continue;
      accessSync(path, constants.X_OK);
      return path;
    } catch {
      // Missing, a stale symlink, or not executable. Try the next location.
    }
  }
  throw new ProviderError("other", "Claude Code was not found. Install Claude Code or set INKLING_CLAUDE_PATH to its executable.");
}

// Tests swap in a fake SDK without needing an installed Claude Code.
type QueryFn = typeof query;
let queryFn: QueryFn = args => query({
  ...args,
  options: { ...args.options, pathToClaudeCodeExecutable: installedClaude() },
});
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
/**
 * With the web: a search or two, a page read, the answer, and its retries. The only limit
 * on a call that searches: it stops a call going round in circles, not one that costs.
 * There is no cap on what it spends, as it runs on the owner's Claude plan.
 */
const WEB_TURNS = 8;
/** The SDK's own tools for looking things up; nothing else is ever turned on. */
export const WEB_TOOLS = ["WebSearch", "WebFetch"];
function baseOptions(system: string, settings: RunSettings, signal: AbortController, web = false): Options {
  const opts: Options = {
    systemPrompt: system,
    tools: web ? WEB_TOOLS : [],
    // Allowed without asking: there is nobody to ask, and these two only read the web.
    allowedTools: web ? WEB_TOOLS : [],
    settingSources: [],
    maxTurns: web ? WEB_TURNS : 1,
    persistSession: false,
    model: settings.model,
    effort: settings.effort,
    cwd: workDir(),
    env: childEnv(),
    abortController: signal,
  };
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
  /** What the call's searches returned: the only URLs a source may cite. */
  found: Found[];
}

async function drain(
  q: AsyncIterable<SDKMessage>,
  started: number,
  structured: boolean,
  onDelta?: (t: string) => void,
  onLimit?: (l: LimitInfo) => void,
  web = false,
): Promise<Drained> {
  let model: string | undefined;
  let streamed = "";
  let assistantError: string | undefined;
  let limitRejected = false;
  const found: Found[] = [];
  let searches = 0;
  for await (const m of q) {
    switch (m.type) {
      case "user": {
        // a tool's result: a search's hits are what a source may cite
        const hits = foundIn((m as any).tool_use_result);
        if (Array.isArray((m as any).tool_use_result?.results)) searches += 1;
        found.push(...hits);
        break;
      }
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
        const usage: Usage = { ms: Date.now() - started, model, costUsd: r.total_cost_usd, ...tokens(r.usage), ...(searches ? { searches } : {}) };
        // Where a call that looked things up spent its tokens, per model, for the log.
        if (searches && r.modelUsage) {
          const per = Object.entries(r.modelUsage as Record<string, any>).map(
            ([name, u]) => `${name}: in ${u.inputTokens}, out ${u.outputTokens}, cache read ${u.cacheReadInputTokens}, cache write ${u.cacheCreationInputTokens}, $${Number(u.costUSD ?? 0).toFixed(4)}`,
          );
          process.stderr.write(`[brain] web call: ${searches} search(es); ${per.join("; ")}` + "\n");
        }
        if (r.subtype !== "success" || r.is_error) {
          const errors = Array.isArray(r.errors) ? r.errors.filter(Boolean).join("; ") : "";
          const msg = String(errors || r.result || r.subtype || "the model returned an error");
          // looking things up ran out of turns: answer again without the web
          if (web && r.subtype === "error_max_turns") {
            throw new WebSpent(`looking it up stopped (${r.subtype}, $${Number(r.total_cost_usd ?? 0).toFixed(2)})`, Number(r.total_cost_usd ?? 0));
          }
          throw new ProviderError(resultKind(r.subtype, assistantError, limitRejected, structured, msg), msg);
        }
        return { text: r.result ?? streamed, structured: r.structured_output, usage, found };
      }
    }
  }
  throw new ProviderError("other", "the model stopped without a result");
}

/** The token counts on a result, as numbers, when the runtime reports them. */
function tokens(u: any): Partial<Usage> {
  if (typeof u !== "object" || u === null) return {};
  const n = (v: unknown) => (typeof v === "number" && Number.isFinite(v) ? v : undefined);
  const out: Partial<Usage> = {
    inputTokens: n(u.input_tokens),
    outputTokens: n(u.output_tokens),
    cacheReadTokens: n(u.cache_read_input_tokens),
    cacheWriteTokens: n(u.cache_creation_input_tokens),
  };
  for (const k of Object.keys(out) as (keyof Usage)[]) if (out[k] === undefined) delete out[k];
  return out;
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
  web = false,
): Promise<{ text: string; usage: Usage; found: Found[] }> {
  const started = Date.now();
  try {
    const opts = baseOptions(system, settings, signal, web);
    opts.includePartialMessages = Boolean(onDelta);
    const out = await drain(queryFn({ prompt, options: opts }), started, false, onDelta, onLimit, web);
    return { text: out.text.trim(), usage: out.usage, found: out.found };
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
  web = false,
): Promise<{ data: T; usage: Usage; found: Found[] }> {
  const started = Date.now();
  try {
    const opts = baseOptions(system, settings, signal, web);
    opts.outputFormat = { type: "json_schema", schema: schema as Record<string, unknown> };
    // The answer comes back as a StructuredOutput tool call, and when it is missing
    // or fails the schema the runtime asks again in another turn. With one turn that
    // retry is impossible and the call fails with "Reached maximum number of turns".
    // Without the web there are no other tools, so extra turns can only be those
    // retries; with it, searches and page reads take turns too.
    opts.maxTurns = web ? WEB_TURNS : STRUCTURED_TURNS;
    const out = await drain(queryFn({ prompt, options: opts }), started, true, undefined, onLimit, web);
    let data = out.structured as T | undefined;
    if (data === undefined || data === null) {
      // Fall back to parsing the text, in case the runtime returned it there.
      data = parseLooseJson<T>(out.text);
    }
    return { data, usage: out.usage, found: out.found };
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
