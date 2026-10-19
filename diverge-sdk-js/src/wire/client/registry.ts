// Where frames should go: the registrations the router reads.

import { Receiver } from "./receiver.js";

/** One open scope: its two receivers, and the channels this end opened in it. */
export interface Entry {
  /** The scope's own answer. */
  readonly responseReceiver: Receiver<Uint8Array>;
  /** The server's channel requests inside it. */
  readonly requestReceiver: Receiver<Uint8Array>;
  /** The channels this end opened, by this end's number. */
  readonly channels: Map<number, Receiver<Uint8Array>>;
}

/**
 * Every scope open on one connection, by number, each with every
 * channel this end opened inside it. The handle registers before it
 * writes, and the router removes on a finish — a scope's finish
 * removing the scope and every channel inside it — which is also
 * what makes a number free to mint again. Shared between the two
 * halves directly: in one thread, a map is the channel the Rust
 * sends registrations and closures over.
 */
export class Registry {
  readonly #scopes = new Map<number, Entry>();

  /** Whether `scope` is open. */
  has(scope: number): boolean {
    return this.#scopes.has(scope);
  }

  /** The entry for `scope`, if open. */
  get(scope: number): Entry | undefined {
    return this.#scopes.get(scope);
  }

  /** Register a scope, its receivers fresh. */
  open(scope: number): Entry {
    const entry: Entry = {
      responseReceiver: new Receiver(),
      requestReceiver: new Receiver(),
      channels: new Map(),
    };
    this.#scopes.set(scope, entry);
    return entry;
  }

  /** Register a channel this end opened in `scope`, its receiver fresh. */
  openChannel(scope: number, channel: number): Receiver<Uint8Array> | undefined {
    const entry = this.#scopes.get(scope);
    if (entry === undefined) {
      return undefined;
    }
    const receiver = new Receiver<Uint8Array>();
    entry.channels.set(channel, receiver);
    return receiver;
  }

  /** A scope is over: close everything under it and forget it. */
  closeScope(scope: number): void {
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
  }

  /** A channel this end opened is over: close it and forget it. */
  closeChannel(scope: number, channel: number): void {
    const entry = this.#scopes.get(scope);
    const receiver = entry?.channels.get(channel);
    if (entry === undefined || receiver === undefined) {
      return;
    }
    entry.channels.delete(channel);
    receiver.close();
  }

  /** The connection is over: every scope is. */
  closeAll(): void {
    for (const scope of [...this.#scopes.keys()]) {
      this.closeScope(scope);
    }
  }
}
