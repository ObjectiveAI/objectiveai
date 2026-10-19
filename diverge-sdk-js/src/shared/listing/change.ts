// What a listing tells.

/**
 * One thing a listing tells: an item come to be listed, one listed
 * that differs from what was last sent, one that ceased to be
 * listed, or the word that the listing as it stood is sent. A list
 * frame turns itself into one by its `change`; the two answers that
 * end a scope — forbidden, and a failure — tell a listing nothing.
 */
export type Change<T> =
  | { readonly kind: "added"; readonly item: T }
  | { readonly kind: "changed"; readonly item: T }
  | { readonly kind: "removed"; readonly item: T }
  | { readonly kind: "listed" };
