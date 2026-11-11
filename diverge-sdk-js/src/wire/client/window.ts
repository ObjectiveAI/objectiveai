// One frame ahead: how a reader keeps its sender from outrunning it.

import { SendError } from "./error.js";

/**
 * What a credit says, as the one byte of its payload.
 *
 * The rules, the same on every stream in either direction:
 *
 * 1. Every stream opens with room for exactly one frame.
 * 2. A sender sends a response, a channel request or a channel
 *    response only while it has room, and sending spends it; without
 *    room it waits (`Window.take`). Only that stream waits.
 * 3. A finish needs no room and never waits: a reader's queue holds
 *    at most one frame and the finish behind it (`DEPTH`).
 * 4. The reader gives room back (`RESPONSES`, `REQUESTS`) when its
 *    consumer TAKES a frame off the queue.
 * 5. A reader that stops before its stream's finish says so
 *    (`RESPONSES_ABANDONED`, `REQUESTS_ABANDONED`): its sender's sends
 *    then return at once and send nothing, and a frame already on its
 *    way is discarded on arrival.
 * 6. A frame that arrives on a stream whose queue is full broke rule 2,
 *    and ends the connection. A credit for a stream the sender does not
 *    know is nothing.
 * 7. A scope that finishes, or a connection that ends, ends every
 *    window under it, and a send waiting on one fails.
 */
export const RESPONSES = 0;
/** Room for one more channel request in the scope. */
export const REQUESTS = 1;
/** That response stream's reader reads nothing more. */
export const RESPONSES_ABANDONED = 2;
/** The scope's channel requests are read no more. */
export const REQUESTS_ABANDONED = 3;

/** How many frames a reader's queue holds: one frame, and its finish (rule 3). */
export const DEPTH = 2;

/** What a sender may do now: send, its room spent; or skip, the reader gone (rule 5). */
export type Go = "send" | "skip";

/** One outgoing stream's room. */
export class Window {
  #room = 1;
  #abandoned = false;
  #ended: "scope" | "connection" | null = null;
  readonly #waiting: Array<() => void> = [];

  /** Wait for room and spend it (rule 2); `skip` with no reader (rule 5); a `SendError` with no stream (rule 7). */
  async take(): Promise<Go> {
    for (;;) {
      if (this.#ended !== null) {
        throw new SendError(this.#ended);
      }
      if (this.#abandoned) {
        return "skip";
      }
      if (this.#room > 0) {
        this.#room -= 1;
        return "send";
      }
      await new Promise<void>((resolve) => this.#waiting.push(resolve));
    }
  }

  /** A credit arrived: what it says, applied. */
  apply(kind: number): void {
    if (kind === RESPONSES || kind === REQUESTS) {
      this.#room += 1;
    } else if (kind === RESPONSES_ABANDONED || kind === REQUESTS_ABANDONED) {
      this.#abandoned = true;
    } else {
      return;
    }
    this.#wake();
  }

  /** Rule 7. The first ending stands. */
  end(why: "scope" | "connection"): void {
    this.#ended ??= why;
    this.#wake();
  }

  #wake(): void {
    for (const resolve of this.#waiting.splice(0)) {
      resolve();
    }
  }
}
