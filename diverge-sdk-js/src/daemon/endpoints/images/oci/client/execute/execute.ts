// Opening the scope, and reading whether it is held.

import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import { ExecuteHandle } from "../../../../../../provider/endpoints/images/oci/client/execute/execute_handle.js";
import * as request from "../request/index.js";
import * as response from "../../server/response/index.js";

/** Why an images oci did not hand back a handle. */
export type OciErrorKind = "send" | "request" | "closed" | "frame" | "unanswered" | "misrouted" | "response" | "not_found" | "forbidden" | "daemon";

/**
 * An oci scope that did not come to be held: the machinery failing as
 * a one-answer exchange's can, or the daemon's answer that is not
 * `held` — no source it asked can supply the image, forbidden, or the
 * daemon's failure, whose value is the `cause`. The last three are
 * what the daemon said; the rest, the absence of an answer.
 */
export class OciError extends Error {
  /** Which. */
  readonly kind: OciErrorKind;

  constructor(kind: OciErrorKind, cause?: unknown) {
    super(`the images oci scope was not held: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "OciError";
    this.kind = kind;
  }
}

/**
 * Open the daemon's oci scope of the one image `frame` names on
 * `handle` and read the daemon's answer: the `ExecuteHandle` when it
 * is held, every ask on it answered on its own channel, else the
 * `OciError` saying why not. There is no timeout: a daemon that never
 * answers is waited on until the connection ends.
 */
export async function execute(handle: Handle, frame: request.Frame): Promise<ExecuteHandle> {
  let payload: Uint8Array;
  try {
    payload = request.encode(frame);
  } catch (cause) {
    throw new OciError("request", cause);
  }
  let scope: Scope;
  try {
    scope = await handle.sendRequest(payload);
  } catch (cause) {
    throw new OciError("send", cause);
  }
  const bytes = await scope.responseReceiver.next();
  if (bytes === undefined) {
    throw new OciError("closed");
  }
  let envelope;
  try {
    envelope = decodeServerFrame(bytes);
  } catch (cause) {
    throw new OciError("frame", cause);
  }
  switch (envelope.kind) {
    case "response":
      break;
    case "response_finish":
      throw new OciError("unanswered");
    default:
      throw new OciError("misrouted");
  }
  let answer: response.Frame;
  try {
    answer = response.decode(envelope.payload);
  } catch (cause) {
    throw new OciError("response", cause);
  }
  if (answer === "held") {
    return new ExecuteHandle(handle, scope);
  }
  if (typeof answer === "string") {
    throw new OciError(answer);
  }
  throw new OciError("daemon", answer.error);
}
