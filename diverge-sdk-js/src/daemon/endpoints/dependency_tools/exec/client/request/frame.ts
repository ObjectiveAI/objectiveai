// Running one command in a dependency tool's container.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";
import { Request } from "../../../../../../shared/containers/exec/request/request.js";

/**
 * Run one command in a dependency tool's container, its output streamed: the
 * dependency, then the command's own members — `argv`, `cwd`, `env` —
 * as `shared.containers.exec.request.Request` states them.
 */
export const Frame = Request.extend({
  /** Whose container. */
  dependency: Dependency,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 117;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
