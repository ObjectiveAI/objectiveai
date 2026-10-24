// How a channel's answer is read.

/**
 * How one answer on a channel is read out of its payload: the item
 * the exchange hands back, or the server's refusal — an answer that
 * says no — and a throw where the payload is not what the exchange
 * reads at all.
 */
export type Answered<T, R> = (payload: Uint8Array) => { readonly item: T } | { readonly refusal: R };
