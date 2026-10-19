// The list a consumer keeps, and the fold that keeps it current.

import type { Change } from "./change.js";
import { type Keyed, sameKey } from "./keyed.js";

/**
 * A list as it stands: what a consumer of a list stream holds. It
 * is written as JSON as the bare list of its items, and it is the
 * shape a consumer materializes: applying what the stream tells
 * means editing this list, so what a consumer holds and what a
 * stream delivers are the same type.
 */
export class Listing<T> {
  /** What tells one item from another. */
  readonly key: Keyed<T>;
  /**
   * The items, in the order the stream listed them: the daemon's
   * lists oldest first, events by id. Empty is an empty list.
   */
  items: T[];

  constructor(key: Keyed<T>, items: T[] = []) {
    this.key = key;
    this.items = items;
  }

  /**
   * Fold one change into this list.
   *
   * This is THE fold. Every holder of a materialized listing applies
   * what a stream tells the same way, so that two consumers fed the
   * same stream cannot disagree about what it meant. The rules:
   *
   * 1. `added` and `changed` are one act: the item whose key matches
   *    is replaced in place, else the item is appended. The wire
   *    distinction is information for the consumer; the list that
   *    results is the same either way, which is what makes the fold
   *    indifferent to replay. An event whose count was raised is
   *    `changed` with the whole record, so replacing it IS the
   *    increment.
   * 2. `removed`: the item whose key matches is taken out; none
   *    matching is nothing.
   * 3. `listed`: nothing. The fold keeps no word of whether the
   *    listing is whole, since the type is the list and nothing
   *    else; a consumer that waits for the word reads it off the
   *    stream before folding.
   */
  push(change: Change<T>): void {
    switch (change.kind) {
      case "added":
      case "changed":
        this.#place(change.item);
        return;
      case "removed":
        this.#take(change.item);
        return;
      case "listed":
        return;
    }
  }

  /** The bare list, which is how a listing is written. */
  toJSON(): T[] {
    return this.items;
  }

  #place(item: T): void {
    const key = this.key(item);
    const at = this.items.findIndex((held) => sameKey(this.key(held), key));
    if (at === -1) {
      this.items.push(item);
    } else {
      this.items[at] = item;
    }
  }

  #take(item: T): void {
    const key = this.key(item);
    const at = this.items.findIndex((held) => sameKey(this.key(held), key));
    if (at !== -1) {
      this.items.splice(at, 1);
    }
  }
}
