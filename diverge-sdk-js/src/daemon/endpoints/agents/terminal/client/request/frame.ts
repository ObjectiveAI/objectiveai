// Opening a terminal in an agent's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Agent } from "../../../../../../daemon/reference/agent.js";
import { Open } from "../../../../../../shared/containers/terminal/open/open.js";

/**
 * Open a terminal in an agent's container: the agent, then the size's
 * own members — `cols`, `rows` — as
 * `shared.containers.terminal.open.Open` states them.
 */
export const Frame = Open.extend({
  /** Whose container. */
  agent: Agent,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 118;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
