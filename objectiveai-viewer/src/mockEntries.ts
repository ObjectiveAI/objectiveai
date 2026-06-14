import type { Entry } from "./types";

const now = Math.floor(Date.now() / 1000);

export const mockEntries: Entry[] = [
  {
    kind: "agent-completion",
    id: "ac-complete-001",
    receivedAt: Date.now() - 30000,
    request: {
      id: "ac-complete-001",
      messages: [
        { role: "developer", content: "You are a code analysis agent. Use the available tools to inspect files." },
        { role: "user", content: "Analyze the function at src/scoring.rs and suggest improvements." },
      ],
    } as any,
    chunk: {
      id: "ac-complete-001",
      object: "agent.completion.chunk",
      created: now - 30,
      model: "gpt-4.1",
      upstream: "openai",
      messages: [
        {
          role: "assistant",
          model: "gpt-4.1",
          upstream: "openai",
          content: null,
          tool_calls: [
            { id: "call_abc123", type: "function", function: { name: "read_file", arguments: JSON.stringify({ path: "src/scoring.rs", line_start: 1, line_end: 50 }) } },
          ],
          finish_reason: "tool_calls",
        },
        {
          role: "tool",
          tool_call_id: "call_abc123",
          content: "pub fn compute_scores(weights: &[f64], votes: &[usize], n_responses: usize) -> Vec<f64> {\n    let mut scores = vec![0.0; n_responses];\n    for (w, &v) in weights.iter().zip(votes.iter()) {\n        scores[v] += w;\n    }\n    scores\n}",
        },
        {
          role: "assistant",
          model: "gpt-4.1",
          upstream: "openai",
          content: "The `compute_scores` function doesn't normalize the output. When total weight is non-zero, divide each score by the sum to get a proper probability distribution.",
          finish_reason: "stop",
        },
      ],
      usage: { prompt_tokens: 1247, completion_tokens: 389, total_tokens: 1636, cost: 0.004218 },
    } as any,
    error: null,
  },
  {
    kind: "agent-completion",
    id: "ac-streaming-002",
    receivedAt: Date.now() - 5000,
    request: {
      id: "ac-streaming-002",
      messages: [
        { role: "user", content: "What are the trade-offs between Swiss System and round-robin tournaments for collective scoring?" },
      ],
    } as any,
    chunk: {
      id: "ac-streaming-002",
      object: "agent.completion.chunk",
      created: now - 2,
      model: "claude-sonnet-4-6",
      upstream: "anthropic",
      messages: [
        {
          role: "assistant",
          model: "claude-sonnet-4-6",
          upstream: "anthropic",
          content: "Swiss System tournaments are significantly more efficient than round-robin when you have many participants. In a round-robin with N players you need N×(N−1)/2 games. Swiss only needs ⌈log₂(N)⌉ rounds, so for 64 players you go from 2,016 games down to 6 rounds. The trade-off is that Swiss doesn't guarantee every pair meets, which can miss subtle preference orderings...",
        },
      ],
    } as any,
    error: null,
  },
  {
    kind: "execution",
    id: "fe-001",
    receivedAt: Date.now() - 20000,
    request: { id: "fe-001" } as any,
    chunk: {
      id: "fe-001",
      object: "vector.function.execution.chunk",
      created: now - 20,
      reasoning: {
        id: "fe-001-reasoning",
        object: "agent.completion.chunk",
        created: now - 20,
        model: "claude-sonnet-4-6",
        upstream: "anthropic",
        messages: [{ role: "assistant", model: "claude-sonnet-4-6", upstream: "anthropic", content: "Splitting 8 responses into 2 pools for Swiss rounds.", finish_reason: "stop" }],
        error: null,
      },
      output: null,
      tasks: [
        {
          object: "vector.completion.chunk",
          task_path: [0, 0],
          completions: [
            { id: "vc-001-c0", object: "agent.completion.chunk", created: now - 18, index: 0, model: "claude-sonnet-4-6", upstream: "anthropic", messages: [{ role: "assistant", model: "claude-sonnet-4-6", upstream: "anthropic", content: "A", finish_reason: "stop" }], error: null },
            { id: "vc-001-c1", object: "agent.completion.chunk", created: now - 17, index: 1, model: "gpt-4.1-mini", upstream: "openai", messages: [{ role: "assistant", model: "gpt-4.1-mini", upstream: "openai", content: "B", finish_reason: "stop" }], error: null },
          ],
        },
        {
          object: "scalar.function.execution.chunk",
          task_path: [1],
          split_index: 0,
          swiss_pool_index: 1,
          swiss_round: 2,
          reasoning: { id: "fe-nested-r", object: "agent.completion.chunk", created: now - 15, model: "claude-haiku-4-5", upstream: "anthropic", messages: [{ role: "assistant", model: "claude-haiku-4-5", upstream: "anthropic", content: "Pool 1 round 2: comparing remaining candidates.", finish_reason: "stop" }], error: null },
          tasks: [
            { object: "vector.completion.chunk", task_path: [1, 0], completions: [{ id: "vc-n-c0", object: "agent.completion.chunk", created: now - 14, index: 0, model: "claude-haiku-4-5", upstream: "anthropic", messages: [{ role: "assistant", model: "claude-haiku-4-5", upstream: "anthropic", content: "C", finish_reason: "stop" }], error: null }] },
          ],
        },
      ],
    } as any,
    error: null,
  },
  {
    kind: "agent-completion",
    id: "ac-error-003",
    receivedAt: Date.now() - 25000,
    request: { id: "ac-error-003", messages: [{ role: "user", content: "Run the full evaluation pipeline." }] } as any,
    chunk: null,
    error: { id: "ac-error-003", code: 429, message: "Rate limit exceeded. Retry after 12 seconds." } as any,
  },
];
