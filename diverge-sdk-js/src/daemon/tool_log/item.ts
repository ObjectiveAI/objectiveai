// What one entry of a tool's log holds.

import { z } from "zod";
import { CallToolResultSchema, ReadResourceResultSchema } from "@modelcontextprotocol/core";
import { Error } from "../../shared/error.js";
import { Caller } from "../../shared/mcp/attestation/caller.js";
import { Command } from "../command_log/command.js";
import * as lifecycle from "../lifecycle/index.js";

/**
 * One item of a tool's log, told apart by `type`.
 *
 * 1. An ask — `call` or `read` — is kept when the daemon routes it,
 *    before the tool is asked; its answer — `call_result` /
 *    `call_error`, `read_result` / `read_error` — when the tool
 *    answers, naming the ask by its `logs_index`.
 * 2. `active` is kept just before an ask while no other is in flight,
 *    `inactive` just after the answer that leaves none in flight;
 *    `deployed` and `undeployed` bracket every run, as `lifecycle`
 *    states. An ask the run went down under is answered
 *    `call_interrupted` / `read_interrupted` before the run's
 *    `inactive` and `undeployed`.
 * 3. A `notification` is kept once per notification the tool sends,
 *    naming the caller whose session it was sent on.
 * 4. An `error` is the daemon's own, met keeping the tool.
 */
export const Item = z.discriminatedUnion("type", [
  /** The tool's container came up on a provider. */
  z.object({ type: z.literal("deployed"), ...lifecycle.Deployed.shape }),
  /** An ask came in while none was in flight. */
  z.object({ type: z.literal("active") }),
  /** The last ask in flight was answered. */
  z.object({ type: z.literal("inactive") }),
  /** The tool's container went down, and why when unexpectedly. */
  z.object({ type: z.literal("undeployed"), ...lifecycle.Undeployed.shape }),
  /** A tool call: who asked, the tool's own name for what was called, and the arguments as they came. */
  z.object({ type: z.literal("call"), caller: Caller, name: z.string(), arguments: z.record(z.string(), z.json()).optional() }),
  /** A tool call's result, whole: `call` is the call's `logs_index`. */
  z.object({ type: z.literal("call_result"), call: z.number().int().nonnegative(), result: CallToolResultSchema }),
  /** A tool call the tool did not answer, in the daemon's words. */
  z.object({ type: z.literal("call_error"), call: z.number().int().nonnegative(), error: Error }),
  /** A tool call the run went down under: `call` is the call's `logs_index`. */
  z.object({ type: z.literal("call_interrupted"), call: z.number().int().nonnegative() }),
  /** A resource read: who asked, and which resource. */
  z.object({ type: z.literal("read"), caller: Caller, uri: z.string() }),
  /** A resource read's result, whole: `read` is the read's `logs_index`. */
  z.object({ type: z.literal("read_result"), read: z.number().int().nonnegative(), result: ReadResourceResultSchema }),
  /** A resource read the tool did not answer, in the daemon's words. */
  z.object({ type: z.literal("read_error"), read: z.number().int().nonnegative(), error: Error }),
  /** A resource read the run went down under: `read` is the read's `logs_index`. */
  z.object({ type: z.literal("read_interrupted"), read: z.number().int().nonnegative() }),
  /** A notification the tool sent, MCP's own JSON as it came, and the caller whose session it was sent on. */
  z.object({ type: z.literal("notification"), to: Caller, notification: z.json() }),
  /** An error the daemon met keeping the tool. */
  z.object({ type: z.literal("error"), error: Error }),
  /** A request the tool put to the daemon. */
  Command,
]);
export type Item = z.infer<typeof Item>;
