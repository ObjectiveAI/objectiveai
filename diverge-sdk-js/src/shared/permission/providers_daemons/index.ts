// Through providers, over the daemons connected to them, naming no
// provider.
//
// In the shape `permission` states for actions over what exists:
// `Over`, the actions — listing the announced identities, listening for
// messages, sending one — reaching the providers by their tags, as a
// grant over `providers` does; and `Permission`, one grant's worth of
// them. Nothing is made and nothing is tagged here: the records are the
// providers', and the providers grant makes and tags them. The daemon's
// own grant through providers may also name them by identity: see
// `daemon.grant.providers_daemons`.

export * from "./over.js";
export * from "./permission.js";
