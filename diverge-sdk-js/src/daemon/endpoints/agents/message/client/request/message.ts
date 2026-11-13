// What is asked of the agent: a chat message, or a structured decision.

import { z } from "zod";
import { Question } from "../../../../../../shared/containers/structured_decisions/question.js";
import { ContentBlockSchema } from "@modelcontextprotocol/core";

/**
 * What is asked, by its `type`. The content of either is MCP's own
 * content blocks, relayed to the container verbatim. Before anything
 * is sent on, the daemon holds the request to what the agent's image
 * says: that it declares the kind — `chat`, or
 * `structured_decisions` — that a structured decision's questions
 * are valid, and that the content carries only what the image says
 * it takes. A request that breaks one reaches nothing.
 */
export const Message = z.discriminatedUnion("type", [
  /** A chat message, which joins the agent's conversation. */
  z.object({
    type: z.literal("chat"),
    /** The message: content blocks, as MCP writes them. */
    content: z.array(ContentBlockSchema),
  }),
  /**
   * A structured decision: a prompt, and one or more typed questions,
   * each answered under its own id. The agent's answers are no part
   * of an unwatched request's answer: they are read in its log.
   */
  z.object({
    type: z.literal("structured_decisions"),
    /** The prompt: content blocks, as MCP writes them. */
    content: z.array(ContentBlockSchema),
    /** The questions, in order. */
    questions: z.array(Question),
  }),
]);
export type Message = z.infer<typeof Message>;
