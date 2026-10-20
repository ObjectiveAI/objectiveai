// Over the events: the one thing of its kind.
//
// The daemon keeps exactly one, so a grant over it names nothing and
// reaches no narrower than the whole: on the wire a bare array of the
// `Action`s held, `{"events":["list"]}`, and holding the action is
// the whole of it. `Permission` is that array as a type.

export * from "./action.js";
export * from "./permission.js";
