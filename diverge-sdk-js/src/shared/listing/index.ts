// A list stream, and the list a consumer keeps from it.
//
// Every list of the daemon's is a stream: an initial listing, each
// item as added and then the word that the listing is whole, and
// from then on each item come to be listed, listed still but
// differing, or listed no more, until the client cancels. What such
// a stream tells is one `Change` at a time, and what a consumer
// keeps from it is a `Listing`: the list as it stands, folded by
// `push` the way a filetree's root is folded by `update`. The fold
// tells one item from another by a `Keyed` key, which each listed
// record states for itself, beside its schema.

export * from "./change.js";
export * from "./keyed.js";
export * from "./listing.js";
