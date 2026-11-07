// A stand-in for the Agent SDK's query(). It reads a directive from the prompt,
// `[[fake {...json...}]]`, and plays back the messages the real runtime sends,
// honouring the abort signal the way the SDK does (the iterator throws).

export interface Directive {
  deltas?: string[]; // streamed text chunks (only when includePartialMessages is on)
  delayMs?: number; // wait before each message
  result?: string; // final result text (defaults to the deltas joined)
  structured?: unknown; // structured_output on the result
  hang?: boolean; // never finish until aborted
  ignoreAbort?: boolean; // finish normally even after an abort (a result racing a cancel)
  resultError?: { subtype: string; errors?: string[]; result?: string; isError?: boolean };
  assistantError?: string; // the error tag on a synthesised assistant message
  throwMessage?: string; // the iterator throws this
  endEarly?: boolean; // the iterator ends without a result
  limit?: { status: string; resetsAt?: number; utilization?: number; rateLimitType?: string };
  echo?: "options" | "env"; // result text describes what the brain passed in
  // web searches the model ran before answering: each yields a WebSearch tool result
  searches?: { query: string; results: { title: string; url: string }[] }[];
  // pages the brain's own page fetch finds (url -> page text); any other URL cannot be opened
  pages?: Record<string, string>;
  // resultError only for a call that can look things up (it has tools); others succeed
  webOnce?: boolean;
}

/** Pages the fake web serves, filled from the directives played so far (test/fake-brain.ts). */
export const fakePages = new Map<string, string>();

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function directiveOf(prompt: unknown): Directive {
  const m = typeof prompt === "string" ? /\[\[fake (\{.*?\})\]\]/s.exec(prompt) : null;
  return m ? (JSON.parse(m[1]) as Directive) : { result: "ok" };
}

export function fakeQuery({ prompt, options }: { prompt: unknown; options?: any }) {
  const d = directiveOf(prompt);
  for (const [url, text] of Object.entries(d.pages ?? {})) fakePages.set(url, text);
  const signal: AbortSignal | undefined = options?.abortController?.signal;
  const aborted = () => Boolean(signal?.aborted) && !d.ignoreAbort;
  const abortError = () => new Error("Claude Code process aborted by user");

  async function step() {
    if (d.delayMs) await sleep(d.delayMs);
    if (aborted()) throw abortError();
  }

  async function* messages(): AsyncGenerator<any, void> {
    await step();
    yield { type: "system", subtype: "init", model: options?.model ?? "fake-model" };
    if (d.limit) {
      await step();
      yield { type: "rate_limit_event", rate_limit_info: d.limit };
    }
    if (d.hang) {
      await new Promise<void>((resolve) => {
        if (signal?.aborted) resolve();
        signal?.addEventListener("abort", () => resolve(), { once: true });
      });
      throw abortError();
    }
    for (const [i, s] of (d.searches ?? []).entries()) {
      await step();
      yield {
        type: "user",
        parent_tool_use_id: null,
        message: { role: "user", content: [{ type: "tool_result", tool_use_id: `search-${i}`, content: "results" }] },
        tool_use_result: { query: s.query, results: [{ tool_use_id: `search-${i}`, content: s.results }], durationSeconds: 0.1 },
      };
    }
    let streamed = "";
    for (const t of d.deltas ?? []) {
      await step();
      streamed += t;
      if (options?.includePartialMessages) {
        yield { type: "stream_event", event: { type: "content_block_delta", index: 0, delta: { type: "text_delta", text: t } } };
      }
    }
    if (d.throwMessage) {
      await step();
      throw new Error(d.throwMessage);
    }
    if (d.endEarly) return;
    if (d.assistantError) {
      await step();
      yield { type: "assistant", error: d.assistantError, message: { content: [{ type: "text", text: d.resultError?.result ?? "API Error" }] } };
    }
    await step();
    if (d.resultError && (!d.webOnce || (options?.tools?.length ?? 0) > 0)) {
      yield {
        type: "result",
        subtype: d.resultError.subtype,
        is_error: d.resultError.isError ?? true,
        errors: d.resultError.errors ?? [],
        result: d.resultError.result,
        total_cost_usd: d.webOnce ? 1.5 : 0,
      };
      return;
    }
    let result = d.result ?? streamed;
    if (d.echo === "options") {
      result = JSON.stringify({
        model: options?.model ?? null,
        effort: options?.effort ?? null,
        tools: options?.tools,
        allowedTools: options?.allowedTools,
        maxBudgetUsd: options?.maxBudgetUsd ?? null,
        maxTurns: options?.maxTurns,
        persistSession: options?.persistSession,
        settingSources: options?.settingSources,
        cwd: options?.cwd,
        outputFormat: options?.outputFormat?.type ?? null,
        includePartialMessages: Boolean(options?.includePartialMessages),
        systemPrompt: options?.systemPrompt,
        prompt,
      });
    } else if (d.echo === "env") {
      const env = options?.env ?? {};
      result = JSON.stringify({
        ANTHROPIC_API_KEY: "ANTHROPIC_API_KEY" in env && env.ANTHROPIC_API_KEY !== undefined,
        ANTHROPIC_AUTH_TOKEN: "ANTHROPIC_AUTH_TOKEN" in env && env.ANTHROPIC_AUTH_TOKEN !== undefined,
        CLAUDE_AGENT_SDK_CLIENT_APP: env.CLAUDE_AGENT_SDK_CLIENT_APP ?? null,
      });
    }
    yield { type: "result", subtype: "success", is_error: false, result, structured_output: d.structured, total_cost_usd: 0.001 };
  }

  const it = messages();
  // FAKE_STATUS steers accountInfo(): ok (default), fail, hang, or slow-models.
  const status = process.env.FAKE_STATUS ?? "ok";
  const never = new Promise<never>(() => {});
  return Object.assign(it, {
    accountInfo: async () => {
      if (status === "hang") return never;
      if (status === "fail") throw new Error("Not logged in · Please run /login");
      return { email: "writer@example.com", subscriptionType: "max", tokenSource: "claude.ai" };
    },
    supportedModels: async () => {
      if (status === "slow-models") return never;
      return [{ value: "claude-opus-5-5", displayName: "Opus 5.5" }];
    },
    close: () => {},
    interrupt: async () => undefined,
  });
}
