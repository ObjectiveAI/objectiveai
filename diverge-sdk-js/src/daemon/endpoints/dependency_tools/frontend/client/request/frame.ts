// One HTTP request to a dependency tool's frontend.

import { z } from "zod";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Relay } from "../../../../../../shared/relay.js";
import { Dependency } from "../../../../../../daemon/reference/dependency.js";
import { Head } from "../../../../../../shared/containers/frontend/request/head.js";

/**
 * One HTTP request to a dependency tool's frontend: the dependency, then the head's
 * own members — `method`, `path`, `headers` — as
 * `shared.containers.frontend.request.Head` states them.
 */
export const Frame = Head.extend({
  /** Whose frontend. */
  dependency: Dependency,
  /** Served by another daemon: see `Relay`. Absent, served here. */
  relay: Relay.optional(),
});
export type Frame = z.infer<typeof Frame>;

/** This frame's tag among the scope-opening requests: one byte at the front of the payload. */
const TAG = 122;

/** The payload: the tag, then the frame as JSON, as every request of the daemon's is. */
export function encode(frame: Frame): Uint8Array {
  return tagged(TAG, json(frame));
}
