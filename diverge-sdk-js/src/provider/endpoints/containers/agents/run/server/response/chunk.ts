// One chunk of the agentic loop: one of fourteen kinds.

import { z } from "zod";
import { AssistantAudioContentChunk } from "./assistant_audio_content_chunk.js";
import { AssistantImageContentChunk } from "./assistant_image_content_chunk.js";
import { AssistantReasoningChunk } from "./assistant_reasoning_chunk.js";
import { AssistantRefusalChunk } from "./assistant_refusal_chunk.js";
import { AssistantTextContentChunk } from "./assistant_text_content_chunk.js";
import { AssistantToolCallChunk } from "./assistant_tool_call_chunk.js";
import { NotificationChunk } from "./notification_chunk.js";
import { ToolResponseChunk } from "./tool_response_chunk.js";
import { UsageChunk } from "./usage_chunk.js";
import { UserAudioContentChunk } from "./user_audio_content_chunk.js";
import { UserImageContentChunk } from "./user_image_content_chunk.js";
import { UserResourceChunk } from "./user_resource_chunk.js";
import { UserResourceLinkChunk } from "./user_resource_link_chunk.js";
import { UserTextContentChunk } from "./user_text_content_chunk.js";

/**
 * One chunk of a streaming agentic loop.
 *
 * Each chunk is ONE event, not a struct of mostly-absent optionals. A
 * consumer learns what happened by matching, rather than by inspecting
 * which fields happen to be set.
 *
 * **Untagged, discriminated by payload.** serde adds no tag of its own;
 * instead every variant's payload carries a `type` field whose value no
 * other variant can produce — each `type` is its own single-variant
 * enum, so a wrong value fails to deserialize instead of arriving as
 * data nobody checks. So the wire shape is the event itself rather than
 * a wrapper around one, and deserialization is still unambiguous — the
 * `type` constants do the work a tag would, without a level of nesting.
 *
 * That also means variant order here is not load-bearing. Untagged
 * deserialization takes the first variant that matches, and with
 * distinct `type` constants at most one ever can.
 */
export const AgenticLoopChunk = z.discriminatedUnion("type", [
  AssistantReasoningChunk,
  AssistantTextContentChunk,
  AssistantImageContentChunk,
  AssistantAudioContentChunk,
  AssistantToolCallChunk,
  AssistantRefusalChunk,
  ToolResponseChunk,
  UserTextContentChunk,
  UserImageContentChunk,
  UserAudioContentChunk,
  UserResourceChunk,
  UserResourceLinkChunk,
  UsageChunk,
  NotificationChunk,
]);
export type AgenticLoopChunk = z.infer<typeof AgenticLoopChunk>;
