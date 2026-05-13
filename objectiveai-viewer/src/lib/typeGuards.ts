import type { AgentCompletionsResponseStreamingAssistantResponseChunk } from "objectiveai";

export function isAssistantMessage(
  msg: { role: string },
): msg is { role: "assistant" } & AgentCompletionsResponseStreamingAssistantResponseChunk {
  return msg.role === "assistant";
}

export function hasContent(value: unknown): value is { content: unknown } {
  return typeof value === "object" && value !== null && "content" in value;
}

export function hasImageUrl(part: object): part is { image_url: { url: string } } {
  return "image_url" in part &&
    typeof (part as Record<string, unknown>).image_url === "object" &&
    (part as Record<string, unknown>).image_url !== null &&
    "url" in ((part as Record<string, unknown>).image_url as object);
}

export function hasFile(part: object): part is { file: { filename?: string } } {
  return "file" in part &&
    typeof (part as Record<string, unknown>).file === "object" &&
    (part as Record<string, unknown>).file !== null;
}
