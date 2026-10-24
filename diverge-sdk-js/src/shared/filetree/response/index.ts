// Filetree response data.
//
// A filetree response is a STREAM: one `snapshot` carrying the whole
// tree, then one delta per change — inserted, modified, or removed —
// for as long as the caller watches. The rules governing that
// sequence are ordering properties over a stream, which no schema
// can express; they belong to the specification. What this module
// defines is the vocabulary — a node, a frame — in the tagged JSON
// form a consumer reads, with the postcard codec each rides the
// wire in, and `Root`, the tree a consumer keeps.

export * from "./node.js";
export * from "./frame.js";
export * from "./root.js";
