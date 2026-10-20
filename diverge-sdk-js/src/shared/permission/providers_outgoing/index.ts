// Grants over outgoing providers that name none of them.
//
// What a dependency tool template may ask over outgoing providers, in the shapes
// `permission` states: `Make`, the actions that bring one into being; `Over`,
// the actions over those that exist, reaching by tags; and
// `Permission`, one grant's worth of either, or of tagging. The
// daemon's grants over outgoing providers are made of the same actions.

export * from "./make.js";
export * from "./over.js";
export * from "./permission.js";
