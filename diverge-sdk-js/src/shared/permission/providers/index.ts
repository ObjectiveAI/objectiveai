// Over providers' records.
//
// In the shapes `permission` states: `Make`, the action that records one;
// `Over`, the actions over the records — reading, listing, asking them
// for images, changing, deleting, enabling and disabling one — reaching
// by tags; and `Permission`, one grant's worth
// of either, or of tagging. The daemon's own grant over providers may
// also name them by identity: see `daemon.grant.providers`.

export * from "./make.js";
export * from "./over.js";
export * from "./permission.js";
