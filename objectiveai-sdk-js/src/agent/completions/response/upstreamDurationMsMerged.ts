import type { AgentCompletionsResponseUpstreamDurationMs } from "./upstreamDurationMs";

/** Sum one optional per-upstream duration field — the TS mirror of the
 * Rust `push_option_u64` (both present ⇒ sum; one present ⇒ it; none ⇒
 * absent). */
function sumField(
  a: number | null | undefined,
  b: number | null | undefined,
): number | undefined {
  if (a != null && b != null) return a + b;
  return a ?? b ?? undefined;
}

/** The TS mirror of the Rust `UpstreamDurationMs::push`: per-field
 * sums across turns, fallbacks, and parallel agents. A field is
 * absent when that upstream was never used. */
export function agentCompletionsResponseUpstreamDurationMsMerged(
  a: AgentCompletionsResponseUpstreamDurationMs,
  b: AgentCompletionsResponseUpstreamDurationMs,
): [AgentCompletionsResponseUpstreamDurationMs, boolean] {
  const openrouter = sumField(a.openrouter, b.openrouter);
  const claude_agent_sdk = sumField(a.claude_agent_sdk, b.claude_agent_sdk);
  const codex_sdk = sumField(a.codex_sdk, b.codex_sdk);
  const changed =
    openrouter !== (a.openrouter ?? undefined) ||
    claude_agent_sdk !== (a.claude_agent_sdk ?? undefined) ||
    codex_sdk !== (a.codex_sdk ?? undefined);
  if (!changed) return [a, false];
  return [
    {
      ...(openrouter !== undefined ? { openrouter } : {}),
      ...(claude_agent_sdk !== undefined ? { claude_agent_sdk } : {}),
      ...(codex_sdk !== undefined ? { codex_sdk } : {}),
    },
    true,
  ];
}
