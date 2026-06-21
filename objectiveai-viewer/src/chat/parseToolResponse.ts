import type {
  AgentCompletionsMessageMessage,
  AgentCompletionsMessageRichContent,
  AgentCompletionsMessageRichContentPart,
} from "@objectiveai/sdk";

/** Result of running `parseToolResponse` on a tool message's
 * `content`. The cleaned form drops any
 * `<system-reminder>The user sent a new message…</system-reminder>`
 * blocks the MCP proxy injected; those become standalone
 * `extractedUserMessages` for separate rendering. */
export interface ParsedToolResponse {
  cleanContent: AgentCompletionsMessageRichContent;
  extractedUserMessages: AgentCompletionsMessageMessage[];
}

// The MCP proxy wraps queued user messages in a
// `<system-reminder-{uuid}>…</system-reminder-{uuid}>` block (both tags carry
// the confirmation token). The token is optional here so already-stored
// conversations — which used a token-less `</system-reminder>` closing tag —
// still match. Group 1 captures the message body.
const NOTIFICATION_RE =
  /<system-reminder(?:-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})?>\nThe user sent a new message while you were working:\n([\s\S]*?)\n\n<\/system-reminder(?:-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})?>/g;

interface ExtractionResult {
  cleaned: string;
  users: AgentCompletionsMessageMessage[];
}

function extractFromString(text: string): ExtractionResult {
  const users: AgentCompletionsMessageMessage[] = [];
  const cleaned = text.replace(NOTIFICATION_RE, (_match, captured: string) => {
    users.push({
      role: "user",
      content: captured,
    } as AgentCompletionsMessageMessage);
    return "";
  });
  return { cleaned: cleaned.trim(), users };
}

export function parseToolResponse(
  content: AgentCompletionsMessageRichContent,
): ParsedToolResponse {
  if (typeof content === "string") {
    const { cleaned, users } = extractFromString(content);
    return {
      cleanContent: cleaned as AgentCompletionsMessageRichContent,
      extractedUserMessages: users,
    };
  }
  if (Array.isArray(content)) {
    const extractedUserMessages: AgentCompletionsMessageMessage[] = [];
    const cleanedParts: AgentCompletionsMessageRichContentPart[] = [];
    for (const part of content) {
      if ("text" in part && typeof part.text === "string") {
        const { cleaned, users } = extractFromString(part.text);
        extractedUserMessages.push(...users);
        if (cleaned.length > 0) {
          cleanedParts.push({ ...part, text: cleaned });
        }
      } else {
        cleanedParts.push(part);
      }
    }
    return {
      cleanContent: cleanedParts as AgentCompletionsMessageRichContent,
      extractedUserMessages,
    };
  }
  return {
    cleanContent: content,
    extractedUserMessages: [],
  };
}
