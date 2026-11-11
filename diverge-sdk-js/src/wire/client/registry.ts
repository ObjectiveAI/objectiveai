// Where frames should go: the registrations the router reads.

import { CHANNEL_RESPONSE_FINISH, RESPONSE_FINISH } from "../frame/kind.js";
import { Receiver } from "./receiver.js";
import { REQUESTS, REQUESTS_ABANDONED, RESPONSES, RESPONSES_ABANDONED, Window } from "./window.js";

/**
 * One open scope: its two receivers, the channels this end opened in
 * it, and this end's room on what it sends there (see `window.ts`).
 */
export interface Entry {
  /** The scope's own answer. */
  readonly responseReceiver: Receiver<Uint8Array>;
  /** The server's channel requests inside it. */
  readonly requestReceiver: Receiver<Uint8Array>;
  /** The channels this end opened, by this end's number. */
  readonly channels: Map<number, Receiver<Uint8Array>>;
  /** This end's room for channel requests in the scope. */
  readonly asked: Window;
  /** This end's room on each channel the server opened, by the server's number. */
  readonly answers: Map<number, Window>;
}

/** How a credit is sent: the stream it names, and what it says. */
export type Post = (scope: number, channel: number, kind: number) => void;

/** Whether a whole frame is a finish, which takes no room. */
function finishes(frame: Uint8Array): boolean {
  return frame[0] === RESPONSE_FINISH || frame[0] === CHANNEL_RESPONSE_FINISH;
}

/** A receiver that credits the stream `(scope, channel)` — its channel requests if `requests` — through `post`. */
function crediting(post: Post, scope: number, channel: number, requests: boolean): Receiver<Uint8Array> {
  return new Receiver<Uint8Array>({
    took: (frame) => {
      if (!finishes(frame)) {
        post(scope, channel, requests ? REQUESTS : RESPONSES);
      }
    },
    abandoned: () => post(scope, channel, requests ? REQUESTS_ABANDONED : RESPONSES_ABANDONED),
  });
}

/**
 * Every scope open on one connection, by number, each with every
 * channel this end opened inside it. The handle registers before it
 * writes, and the router removes on a finish — a scope's finish
 * removing the scope and every channel inside it, and ending its
 * windows — which is also what makes a number free to mint again.
 * Shared between the two halves directly: in one thread, a map is the
 * channel the Rust sends registrations and closures over.
 */
export class Registry {
  readonly #scopes = new Map<number, Entry>();
  readonly #post: Post;

  constructor(post: Post) {
    this.#post = post;
  }

  /** Whether `scope` is open. */
  has(scope: number): boolean {
    return this.#scopes.has(scope);
  }

  /** The entry for `scope`, if open. */
  get(scope: number): Entry | undefined {
    return this.#scopes.get(scope);
  }

  /** Register a scope, its receivers and its room fresh. */
  open(scope: number): Entry {
    const entry: Entry = {
      responseReceiver: crediting(this.#post, scope, 0, false),
      requestReceiver: crediting(this.#post, scope, 0, true),
      channels: new Map(),
      asked: new Window(),
      answers: new Map(),
    };
    this.#scopes.set(scope, entry);
    return entry;
  }

  /** Register a channel in an open scope; `undefined` if the scope is not. */
  openChannel(scope: number, channel: number): Receiver<Uint8Array> | undefined {
    const entry = this.#scopes.get(scope);
    if (entry === undefined) {
      return undefined;
    }
    const receiver = crediting(this.#post, scope, channel, false);
    entry.channels.set(channel, receiver);
    return receiver;
  }

  /** Remove a scope: close its receivers and its channels', and end its windows. */
  closeScope(scope: number, why: "scope" | "connection" = "scope"): void {
    const entry = this.#scopes.get(scope);
    if (entry === undefined) {
      return;
    }
    this.#scopes.delete(scope);
    entry.responseReceiver.close();
    entry.requestReceiver.close();
    for (const receiver of entry.channels.values()) {
      receiver.close();
    }
    entry.asked.end(why);
    for (const window of entry.answers.values()) {
      window.end(why);
    }
  }

  /** Remove one channel and close its receiver; the scope stays. */
  closeChannel(scope: number, channel: number): void {
    const entry = this.#scopes.get(scope);
    const receiver = entry?.channels.get(channel);
    if (entry === undefined || receiver === undefined) {
      return;
    }
    entry.channels.delete(channel);
    receiver.close();
  }

  /** Remove every scope: the connection is gone. */
  closeAll(): void {
    for (const scope of [...this.#scopes.keys()]) {
      this.closeScope(scope, "connection");
    }
  }
}
