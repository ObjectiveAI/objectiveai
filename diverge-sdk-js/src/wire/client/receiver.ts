// An unbounded queue, read from one end.

/**
 * The receiving end of an unbounded queue: what a scope's responses
 * arrive on, and a channel's. `push` never blocks and never drops;
 * `next` resolves with the next item, or with `undefined` once the
 * queue is closed and drained. Iterating it is reading it to the
 * end. Closing it twice, or pushing after a close, is nothing.
 */
export class Receiver<T> implements AsyncIterable<T> {
  readonly #items: T[] = [];
  readonly #waiting: Array<(item: T | undefined) => void> = [];
  #closed = false;

  /** One more item, handed straight to a waiting reader if any. */
  push(item: T): void {
    if (this.#closed) {
      return;
    }
    const waiter = this.#waiting.shift();
    if (waiter === undefined) {
      this.#items.push(item);
    } else {
      waiter(item);
    }
  }

  /** No more items will come; readers get what is queued, then the end. */
  close(): void {
    if (this.#closed) {
      return;
    }
    this.#closed = true;
    for (const waiter of this.#waiting.splice(0)) {
      waiter(undefined);
    }
  }

  /** Whether `close` was called. */
  get closed(): boolean {
    return this.#closed;
  }

  /** The next item, or `undefined` at the end. */
  next(): Promise<T | undefined> {
    if (this.#items.length > 0) {
      return Promise.resolve(this.#items.shift());
    }
    if (this.#closed) {
      return Promise.resolve(undefined);
    }
    return new Promise((resolve) => this.#waiting.push(resolve));
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    for (;;) {
      const item = await this.next();
      if (item === undefined) {
        return;
      }
      yield item;
    }
  }
}
