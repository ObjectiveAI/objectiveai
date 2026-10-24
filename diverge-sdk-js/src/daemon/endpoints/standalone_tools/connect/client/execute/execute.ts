// Opening the scope and reading the one answer.

import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import * as request from "../request/index.js";
import * as response from "../../server/response/index.js";
import { ExecuteHandle } from "./execute_handle.js";

/** Why a connect did not hand back a handle. */
export type ConnectErrorKind = "send" | "request" | "closed" | "frame" | "unanswered" | "misrouted" | "response" | "not_found" | "forbidden" | "daemon";

/**
 * A connect that did not connect: the machinery failing as a
 * one-answer exchange's can, or the daemon's answer that is not
 * `connected` — no created tool is the one named, forbidden, or the
 * daemon's failure, whose value is the `cause`. The last three are
 * what the daemon said; the rest, the absence of an answer.
 */
export class ConnectError extends Error {
  /** Which. */
  readonly kind: ConnectErrorKind;

  constructor(kind: ConnectErrorKind, cause?: unknown) {
    super(`the connect did not connect: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "ConnectError";
    this.kind = kind;
  }
}

/**
 * Open a connect on `handle` and read the daemon's one answer: the
 * `ExecuteHandle` when the tool is connected — holding what the
 * daemon told of it, its image and its template — else the
 * `ConnectError` saying why not. The scope's request receiver is
 * left unread: the daemon opens no channel of its own on a connect.
 */
export async function execute(handle: Handle, frame: request.Frame): Promise<ExecuteHandle> {
  let payload: Uint8Array;
  try {
    payload = request.encode(frame);
  } catch (cause) {
    throw new ConnectError("request", cause);
  }
  let scope: Scope;
  try {
    scope = await handle.sendRequest(payload);
  } catch (cause) {
    throw new ConnectError("send", cause);
  }
  const bytes = await scope.responseReceiver.next();
  if (bytes === undefined) {
    throw new ConnectError("closed");
  }
  let envelope;
  try {
    envelope = decodeServerFrame(bytes);
  } catch (cause) {
    throw new ConnectError("frame", cause);
  }
  switch (envelope.kind) {
    case "response":
      break;
    case "response_finish":
      throw new ConnectError("unanswered");
    default:
      throw new ConnectError("misrouted");
  }
  let answer: response.Frame;
  try {
    answer = response.decode(envelope.payload);
  } catch (cause) {
    throw new ConnectError("response", cause);
  }
  if (typeof answer === "string") {
    throw new ConnectError(answer);
  }
  if ("error" in answer) {
    throw new ConnectError("daemon", answer.error);
  }
  return new ExecuteHandle(handle, scope, answer.connected);
}
