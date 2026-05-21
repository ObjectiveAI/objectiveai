import type {
  AgentCompletionsMessageMessage,
  AgentCompletionsRequestAgentCompletionCreateParams,
  FilesystemConfigFavorite,
} from "@objectiveai/sdk";

/**
 * Build an `AgentCompletionCreateParams` for the cli's
 * `objectiveai api agent completions post` endpoint.
 *
 * `FilesystemConfigFavorite` is `RemotePathCommitOptional & { name,
 * note }`; the agent field on the request accepts
 * `RemotePathCommitOptional` directly. We strip `name` / `note` for
 * hygiene (the server would ignore them either way).
 */
export function buildAgentCompletionRequest(
  favorite: FilesystemConfigFavorite,
  newMessages: AgentCompletionsMessageMessage[],
  continuation: string | null,
): AgentCompletionsRequestAgentCompletionCreateParams {
  const { name: _n, note: _nt, ...remote } = favorite as
    FilesystemConfigFavorite & { name: string; note: string };
  return {
    agent: remote as AgentCompletionsRequestAgentCompletionCreateParams["agent"],
    messages: newMessages,
    continuation: continuation ?? null,
    stream: true,
  };
}
