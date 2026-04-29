type SetEntries = (updater: (prev: any[]) => any[]) => void;

const now = Math.floor(Date.now() / 1000);

function assistantMsg(opts: {
  model: string;
  agent?: string;
  content: string;
  reasoning?: string;
  finish_reason?: string;
  tool_calls?: { id: string; function: { name: string; arguments: string } }[];
  index?: number;
}) {
  return {
    role: "assistant" as const,
    model: opts.model,
    agent: opts.agent ?? null,
    finish_reason: opts.finish_reason ?? "stop",
    reasoning: opts.reasoning ?? null,
    content: opts.content,
    refusal: null,
    tool_calls: opts.tool_calls ?? null,
    logprobs: null,
    index: opts.index ?? 0,
  };
}

function usage(prompt: number, completion: number, cost: number) {
  return {
    prompt_tokens: prompt,
    completion_tokens: completion,
    total_tokens: prompt + completion,
    cost,
  };
}

function completion(
  id: string,
  index: number,
  upstream: string,
  msg: ReturnType<typeof assistantMsg>,
  u: ReturnType<typeof usage>,
) {
  return {
    id,
    created: now - 10 + index,
    object: "agent.completion.chunk" as const,
    upstream,
    index,
    messages: [msg],
    usage: u,
    error: null,
  };
}

function agentCompletionEntry() {
  return {
    kind: "agent-completion" as const,
    id: "ac-001",
    request: {
      id: "ac-001",
      messages: [
        {
          role: "system",
          content: "You are a senior software architect. Be direct and specific.",
        },
        {
          role: "user",
          content:
            "We're choosing between gRPC and REST for communication between our internal services. We have 12 services, mostly Python and Go. What do you recommend?",
        },
      ],
    },
    chunk: {
      id: "ac-001",
      created: now,
      object: "agent.completion.chunk",
      upstream: "openrouter",
      messages: [
        assistantMsg({
          model: "claude-sonnet-4-6",
          reasoning:
            "12 services, Python and Go — both have strong gRPC support. Internal services means they control both ends. But I should consider team familiarity, debugging complexity, and whether they actually need streaming or binary efficiency.",
          content:
            "gRPC for the hot paths, REST for everything else.\n\nWith 12 services in Python and Go, you have good gRPC support on both sides (grpcio, google.golang.org/grpc). Use gRPC where you need streaming, high throughput, or strict contracts between services that talk frequently. The generated clients eliminate a whole class of integration bugs.\n\nKeep REST for admin endpoints, webhooks, and anything external services consume. REST is easier to debug with curl, easier to document, and doesn't require protobuf compilation in CI.\n\nThe common mistake is going all-in on one. A service that handles 50k req/s between two Go services and also serves a settings page doesn't need the same protocol for both.",
        }),
      ],
      usage: usage(284, 167, 0.003012),
    },
    error: null,
  };
}

function executionEntry() {
  return {
    kind: "execution" as const,
    id: "exec-001",
    request: { id: "exec-001" },
    chunk: {
      id: "exec-001",
      created: now - 30,
      object: "vector.function.execution.chunk",
      function: null,
      profile: null,
      output: [0.71, 0.29],
      retry_token: null,
      error: null,
      reasoning: null,
      tasks_errors: null,
      usage: usage(5600, 420, 0.0384),
      tasks: [
        {
          object: "vector.completion.chunk",
          task_path: [0],
          completions: [
            completion(
              "vc-001-a",
              0,
              "openrouter",
              assistantMsg({
                model: "claude-sonnet-4-6",
                agent: "safety-reviewer",
                reasoning:
                  "The diff adds input validation on the /users endpoint. The regex pattern for email looks correct. No new SQL queries introduced, existing parameterized queries unchanged. The migration adds a NOT NULL column with a default — safe for concurrent reads.",
                content: "A",
              }),
              usage(1400, 89, 0.0095),
            ),
            completion(
              "vc-001-b",
              1,
              "openrouter",
              assistantMsg({
                model: "gpt-4o",
                agent: "edge-case-finder",
                reasoning:
                  "The email regex doesn't handle plus-addressing (user+tag@domain.com). The migration default is an empty string, but the application code assumes non-empty. If any row hits the validation before backfill completes, it'll reject valid users. This is a deploy-order dependency.",
                content: "B",
              }),
              usage(1400, 112, 0.0102),
            ),
            completion(
              "vc-001-c",
              2,
              "openrouter",
              assistantMsg({
                model: "claude-haiku-4-5",
                agent: "quick-scan",
                content: "A",
              }),
              usage(1400, 4, 0.0007),
            ),
            completion(
              "vc-001-d",
              3,
              "openrouter",
              assistantMsg({
                model: "gemini-2.5-pro",
                agent: "architecture-check",
                reasoning:
                  "Overall structure is fine. The validation layer is in the right place (handler, not model). One concern: the error message exposes internal regex pattern to the client, which is a minor info leak but not a security risk. Approve with note.",
                content: "A",
              }),
              usage(1400, 95, 0.0088),
            ),
          ],
          votes: [
            {
              agent: "safety-reviewer",
              vote: [1.0, 0.0],
              weight: 0.30,
              swarm_index: 0,
              flat_swarm_index: 0,
              prompt_id: "p-001",
              responses_ids: ["r-a", "r-b"],
            },
            {
              agent: "edge-case-finder",
              vote: [0.0, 1.0],
              weight: 0.29,
              swarm_index: 1,
              flat_swarm_index: 1,
              prompt_id: "p-001",
              responses_ids: ["r-a", "r-b"],
            },
            {
              agent: "quick-scan",
              vote: [1.0, 0.0],
              weight: 0.15,
              swarm_index: 2,
              flat_swarm_index: 2,
              prompt_id: "p-001",
              responses_ids: ["r-a", "r-b"],
            },
            {
              agent: "architecture-check",
              vote: [1.0, 0.0],
              weight: 0.26,
              swarm_index: 3,
              flat_swarm_index: 3,
              prompt_id: "p-001",
              responses_ids: ["r-a", "r-b"],
            },
          ],
          scores: [0.71, 0.29],
          weights: [0.71, 0.29],
          error: null,
        },
      ],
    },
    error: null,
  };
}

function inventionEntry() {
  return {
    kind: "invention" as const,
    id: "inv-001",
    request: { id: "inv-001" },
    chunk: {
      id: "inv-001",
      created: now - 60,
      object: "function.invention.recursive.chunk",
      usage: usage(8200, 2400, 0.0612),
      error: null,
      inventions: [
        {
          index: 0,
          state: { type: "complete", name: "content-quality" },
          path: { remote: "filesystem", owner: "objectiveai", repository: "functions" },
          error: null,
          completions: [
            completion(
              "inv-comp-001",
              0,
              "openrouter",
              assistantMsg({
                model: "claude-sonnet-4-6",
                reasoning:
                  "The user wants a content quality scorer. This should be a branch scalar function — it needs sub-functions for readability, factual accuracy, and engagement. The root function averages their scores with learned weights. I'll define the input schema to accept text content and optional metadata.",
                content:
                  'I\'ll create a branch scalar function "content-quality" with three sub-tasks:\n\n1. **readability-scorer** — Evaluates clarity, sentence structure, and vocabulary appropriateness\n2. **factual-accuracy** — Cross-references claims against provided context\n3. **engagement-scorer** — Assesses whether the content holds attention\n\nThe root function combines these with profile weights. Input schema accepts `{ "text": string, "context"?: string, "audience"?: string }`.',
                tool_calls: [
                  {
                    id: "tc-001",
                    function: {
                      name: "write_function",
                      arguments: JSON.stringify(
                        {
                          name: "content-quality",
                          type: "scalar",
                          depth: 1,
                          input_schema: {
                            type: "object",
                            properties: {
                              text: { type: "string" },
                              context: { type: "string" },
                              audience: { type: "string", enum: ["technical", "general", "academic"] },
                            },
                            required: ["text"],
                          },
                        },
                        null,
                        2,
                      ),
                    },
                  },
                ],
                finish_reason: "tool_calls",
              }),
              usage(2800, 890, 0.0245),
            ),
          ],
        },
        {
          index: 1,
          state: { type: "complete", name: "readability-scorer" },
          path: { remote: "filesystem", owner: "objectiveai", repository: "functions" },
          error: null,
          completions: [
            completion(
              "inv-comp-002",
              0,
              "openrouter",
              assistantMsg({
                model: "claude-sonnet-4-6",
                reasoning:
                  "This is a leaf scalar function — it directly scores readability using a swarm of agents. The prompt should instruct agents to evaluate on a 0–1 scale. I'll use the input's text field and optional audience for calibration.",
                content:
                  'Creating leaf scalar function "readability-scorer". The swarm prompt instructs each agent to score text clarity from 0 (incomprehensible) to 1 (crystal clear), calibrated for the target audience. Agents evaluate sentence length variance, jargon density, and logical flow.',
                tool_calls: [
                  {
                    id: "tc-002",
                    function: {
                      name: "write_function",
                      arguments: JSON.stringify(
                        {
                          name: "readability-scorer",
                          type: "scalar",
                          depth: 0,
                          prompt:
                            "Score the readability of this text from 0 to 1. Consider: sentence structure, vocabulary appropriateness for the {{audience}} audience, logical flow, and information density.",
                        },
                        null,
                        2,
                      ),
                    },
                  },
                ],
                finish_reason: "tool_calls",
              }),
              usage(2200, 540, 0.0165),
            ),
          ],
        },
        {
          index: 2,
          state: { type: "in_progress", name: "factual-accuracy" },
          path: { remote: "filesystem", owner: "objectiveai", repository: "functions" },
          error: null,
          completions: [
            completion(
              "inv-comp-003",
              0,
              "openrouter",
              assistantMsg({
                model: "claude-sonnet-4-6",
                reasoning:
                  "Factual accuracy needs context to verify against. If no context is provided, agents should flag unverifiable claims rather than guessing. This is also a leaf scalar function.",
                content:
                  'Creating leaf scalar function "factual-accuracy". When context is provided, agents cross-reference each claim. When no context is available, agents score based on internal consistency and flag claims that would need verification.',
                finish_reason: "stop",
              }),
              usage(3200, 470, 0.0202),
            ),
          ],
        },
      ],
    },
    error: null,
  };
}

export function replayMockEvents(setEntries: SetEntries): () => void {
  const entries = [agentCompletionEntry(), executionEntry(), inventionEntry()];

  const timers: ReturnType<typeof setTimeout>[] = [];

  entries.forEach((entry, i) => {
    timers.push(
      setTimeout(() => {
        setEntries((prev) => [...prev, entry]);
      }, i * 600),
    );
  });

  return () => timers.forEach((t) => clearTimeout(t));
}
