// A queue, read from one end, that says when it is read.

/**
 * What a reader tells the end that feeds it (see the credit rules in
 * `window.ts`): `took`, each item its consumer takes off the queue;
 * `abandoned`, once, that it reads nothing more before its end.
 */
export interface Hooks<T> {
  readonly took?: (item: T) => void;
  readonly abandoned?: () => void;
}

/**
 * The receiving end of a queue: what a scope's responses arrive on,
 * and a channel's. `push` never blocks; `next` resolves with the next
 * item, or with `undefined` once the queue is closed and drained.
 *
 * The rules:
 *
 * 1. Every item a consumer takes — from the queue, or handed straight
 *    to a reader already waiting — is told to `took`. That is what
 *    gives the far end room for one more, so what the queue holds is
 *    one item ahead of its reader and no more (`depth`).
 * 2. `abandon` reads nothing more: the queue is emptied and closed, and
 *    `abandoned` is told — once, and only before the end.
 * 3. Iterating it is reading it to the end; leaving the loop early
 *    (`break`, `return`, a throw) abandons it.
 * 4. Closing it twice, or pushing after a close, is nothing.
 */
export class Receiver<T> implements AsyncIterable<T> {
  readonly #items: T[] = [];
  readonly #waiting: Array<(item: T | undefined) => void> = [];
  readonly #hooks: Hooks<T>;
  #closed = false;

  constructor(hooks: Hooks<T> = {}) {
    this.#hooks = hooks;
  }

  /** One more item, handed straight to a waiting reader if any. */
  push(item: T): void {
    if (this.#closed) {
      return;
    }
    const waiter = this.#waiting.shift();
    if (waiter === undefined) {
      this.#items.push(item);
    } else {
      this.#hooks.took?.(item);
      waiter(item);
    }
  }

  /** The end: every reader waiting is woken with `undefined`. */
  close(): void {
    if (this.#closed) {
      return;
    }
    this.#closed = true;
    for (const waiter of this.#waiting.splice(0)) {
      waiter(undefined);
    }
  }

  /** Rule 2. */
  abandon(): void {
    if (this.#closed) {
      return;
    }
    this.#items.length = 0;
    this.close();
    this.#hooks.abandoned?.();
  }

  get closed(): boolean {
    return this.#closed;
  }

  /** How many items wait to be taken. */
  get depth(): number {
    return this.#items.length;
  }

  /** The next item, or `undefined` at the end. Rule 1. */
  next(): Promise<T | undefined> {
    if (this.#items.length > 0) {
      const item = this.#items.shift() as T;
      this.#hooks.took?.(item);
      return Promise.resolve(item);
    }
    if (this.#closed) {
      return Promise.resolve(undefined);
    }
    return new Promise((resolve) => this.#waiting.push(resolve));
  }

  /** Rule 3. */
  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    try {
      for (;;) {
        const item = await this.next();
        if (item === undefined) {
          return;
        }
        yield item;
      }
    } finally {
      this.abandon();
    }
  }
}
