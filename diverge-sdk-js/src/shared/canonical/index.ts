// The one form a value is hashed in: compact JSON with every object
// key sorted, at every depth.
//
// Every id that is a hash — a template's, a dependency's — is the
// SHA-256 of the value's canonical bytes, and nothing else: the value
// as JSON with no whitespace, absent members omitted, and the keys of
// EVERY object sorted bytewise, the `arguments` and whatever maps lie
// inside them included, so that two spellings of one value hash the
// same. Array element order is data and is kept. `sortObjectKeys` is
// the sort; `bytes` is the whole rule, for whoever hashes.

export * from "./sort.js";
export * from "./bytes.js";
