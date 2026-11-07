// Over providers' records.
//
// In the shapes `permission` states: `Make`, the action that records one;
// `Over`, the actions over the records — reading, listing, discovering,
// listening and messaging through them, changing, deleting, enabling and
// disabling one — reaching by tags; and `Permission`, one grant's worth
// of either, or of tagging. The daemon's grant over providers is this
// one, re-exported: their list filter names no tags.

export * from "./make.js";
export * from "./over.js";
export * from "./permission.js";
