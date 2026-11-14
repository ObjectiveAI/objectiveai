// Opening the scope, and reading whether the volume is served.

import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import { ExecuteHandle } from "../../../../../../provider/endpoints/volumes/fuse_serve/client/execute/execute_handle.js";
import * as request from "../request/index.js";
import * as response from "../../server/response/index.js";

/** Why a volumes fuse serve did not hand back a handle. */
export type FuseServeErrorKind =
  | "send"
  | "request"
  | "closed"
  | "frame"
  | "unanswered"
  | "misrouted"
  | "response"
  | "mounted"
  | "fuse_served"
  | "held"
  | "volume_mode"
  | "unsupported"
  | "forbidden"
  | "not_found"
  | "daemon";

/**
 * A served scope that did not come to be held: the machinery failing
 * as a one-answer exchange's can, or the daemon's answer that is not
 * `serving`. The last eight are what the daemon said — `volume_mode`
 * with the refusal as its `cause`, the name and the mode the volume
 * is in, and `daemon` with the failure's value as its `cause`; the
 * rest, the absence of an answer.
 */
export class FuseServeError extends Error {
  /** Which. */
  readonly kind: FuseServeErrorKind;

  constructor(kind: FuseServeErrorKind, cause?: unknown) {
    super(`the volumes fuse serve scope was not held: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "FuseServeError";
    this.kind = kind;
  }
}

/**
 * Open the daemon's FUSE serve of the volume `frame` names on
 * `handle` and read the daemon's answer: the `ExecuteHandle` when it
 * is serving, every ask on it answered on its own channel, else the
 * `FuseServeError` saying why not. There is no timeout: a daemon that
 * never answers is waited on until the connection ends.
 */
export async function execute(handle: Handle, frame: request.Frame): Promise<ExecuteHandle> {
  let payload: Uint8Array;
  try {
    payload = request.encode(frame);
  } catch (cause) {
    throw new FuseServeError("request", cause);
  }
  let scope: Scope;
  try {
    scope = await handle.sendRequest(payload);
  } catch (cause) {
    throw new FuseServeError("send", cause);
  }
  const bytes = await scope.responseReceiver.next();
  if (bytes === undefined) {
    throw new FuseServeError("closed");
  }
  let envelope;
  try {
    envelope = decodeServerFrame(bytes);
  } catch (cause) {
    throw new FuseServeError("frame", cause);
  }
  switch (envelope.kind) {
    case "response":
      break;
    case "response_finish":
      throw new FuseServeError("unanswered");
    default:
      throw new FuseServeError("misrouted");
  }
  let answer: response.Frame;
  try {
    answer = response.decode(envelope.payload);
  } catch (cause) {
    throw new FuseServeError("response", cause);
  }
  if (answer === "serving") {
    return new ExecuteHandle(handle, scope);
  }
  if (typeof answer === "string") {
    throw new FuseServeError(answer);
  }
  if ("volume_mode" in answer) {
    throw new FuseServeError("volume_mode", answer.volume_mode);
  }
  throw new FuseServeError("daemon", answer.error);
}
