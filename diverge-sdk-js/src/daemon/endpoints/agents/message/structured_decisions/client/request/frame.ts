// Putting a structured decision to an agent.

import { z } from "zod";
import { json, tagged } from "../../../../../../../wire/encode.js";
import { Relay } from "../../../../../../../shared/relay.js";
import { Agent } from "../../../../../../../daemon/reference/agent.js";
import { Question } from "../../../../../../../shared/containers/structured_decisions/question.js";
import { ContentBlockSchema } from "@modelcontextprotocol/core";

/**
 * Ask the daemon to put a structured decision to an agent: the
 * prompt's content, MCP's own blocks, and one or more typed
 * questions, each answered under its own id. Before anything is sent
 * on, the daemon holds it to three things, and a request that breaks
 * one reaches nothing: the agent's image declares
 * `structured_decisions`; the questions are valid; and the content,
 * the instructions, the options' descriptions and the levels carry
 * only what the image says it takes. The one answer is the
 * decision's fate, which may be long after the ask: a decision waits
 * for everything the agent was handed before it to finish. The
 * agent's answers to the questions are not this request's answer:
 * they are read in the agent's log.
 */
export const Frame = z.object({
  /** The agent: by name, or by template and index. */
  agent: Agent,
  /** The prompt: content blocks, as MCP writes them. */
  content: z.array(ContentBlockSchema),
  /** The questions, in order. */
  questions: z.array(Question),
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 157;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
