// A template: what an agent, or a tool, is made from, held by its
// hash.
//
// A template is everything about an agent or a tool that is not its
// name, not the provider it runs on and not its mounts: the image,
// the limits, the arguments, the secrets it needs by name, and what
// the create has to bring that the template cannot name, in words.
// The shape is one, `template`, written once here; which of the two
// it is for is its `type`, `"agent"` or `"tool"`, flat among the
// members, so the two kinds never hash the same. The id is the
// template's hash: the SHA-256 of its canonical bytes over `hashed`.

export * from "./template.js";
