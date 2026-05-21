/** Stable per-instance keys for collapsible-dropdown override
 * tracking. Stored on `PanelTab.dropdownOverrides` so user clicks
 * persist across tab switching and panel toggling.
 *
 * `entryRequestId` is the per-completion uuid stashed on
 * `PanelTabCompletionEntry`. `msgIndex` is the position within
 * `chunk.messages`. `callIndex` is the tool-call's position within
 * an assistant message's `tool_calls`.
 */

export function reasoningKey(entryRequestId: string, msgIndex: number): string {
  return `reasoning:${entryRequestId}:${msgIndex}`;
}

export function toolCallKey(
  entryRequestId: string,
  msgIndex: number,
  callIndex: number,
): string {
  return `tool-call:${entryRequestId}:${msgIndex}:${callIndex}`;
}

export function toolResponseKey(
  entryRequestId: string,
  msgIndex: number,
): string {
  return `tool-response:${entryRequestId}:${msgIndex}`;
}
