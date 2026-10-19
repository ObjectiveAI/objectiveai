// The shapes more than one endpoint is made of.
//
// `error` is the one failure every endpoint answers with. `filetree`
// is a live filesystem view, and the fold that keeps one current.
// `listing` is a list stream's vocabulary and the list a consumer
// keeps from it, with its fold. They live here rather than beside
// whichever endpoint used them first because a shape defined twice
// is two shapes that agree until they do not, and the folds are
// exactly the kind of thing that would stop agreeing quietly.

export * as error from "./error.js";
export * as filetree from "./filetree/index.js";
export * as listing from "./listing/index.js";
