import type { AgentCompletionsMessageRichContent } from "./richContent";
import type { AgentCompletionsMessageRichContentPart } from "./richContentPart";

export function agentCompletionsMessageRichContentMerged(
  a: AgentCompletionsMessageRichContent,
  b: AgentCompletionsMessageRichContent,
): [AgentCompletionsMessageRichContent, boolean] {
  const aIsString = typeof a === "string";
  const bIsString = typeof b === "string";

  if (aIsString && bIsString) {
    // Text + Text: concatenate
    if (b === "") return [a, false];
    return [a + b, true];
  }
  if (aIsString && !bIsString) {
    // Text + Parts: convert text to part, extend with other parts
    const parts: AgentCompletionsMessageRichContentPart[] = [
      { text: a as string, type: "text" as const },
      ...(b as AgentCompletionsMessageRichContentPart[]),
    ];
    return [parts, true];
  }
  if (!aIsString && bIsString) {
    // Parts + Text: append text as part
    if (b === "") return [a, false];
    return [
      [...(a as AgentCompletionsMessageRichContentPart[]), { text: b as string, type: "text" as const }],
      true,
    ];
  }
  // Parts + Parts: extend
  const bParts = b as AgentCompletionsMessageRichContentPart[];
  if (bParts.length === 0) return [a, false];
  return [
    [...(a as AgentCompletionsMessageRichContentPart[]), ...bParts],
    true,
  ];
}
