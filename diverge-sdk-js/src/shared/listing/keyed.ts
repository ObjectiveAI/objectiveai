// What tells one listed item from another.

/**
 * A key: a string or number, or an array or object of keys —
 * anything JSON can write, compared by value and not by identity, so
 * that a composite key such as a template and an index is written
 * as the pair it is.
 */
export type Key = string | number | boolean | null | readonly Key[] | { readonly [member: string]: Key };

/**
 * A listed record's key, as a function of the record: what tells it
 * from every other item of its list, as the record carries it on
 * the wire — the members that never change while the item is
 * listed, and not one an edit may change. Each record module
 * exports its own, beside its schema, and a `Listing` is made with
 * it.
 */
export type Keyed<T> = (item: T) => Key;

/** Whether two keys are the same key, member for member. */
export function sameKey(a: Key, b: Key): boolean {
  if (a === b) {
    return true;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((item, i) => sameKey(item, b[i] as Key));
  }
  if (typeof a === "object" && a !== null && !Array.isArray(a) && typeof b === "object" && b !== null && !Array.isArray(b)) {
    const left = a as { readonly [member: string]: Key };
    const right = b as { readonly [member: string]: Key };
    const members = Object.keys(left);
    return members.length === Object.keys(right).length && members.every((m) => m in right && sameKey(left[m] as Key, right[m] as Key));
  }
  return false;
}
