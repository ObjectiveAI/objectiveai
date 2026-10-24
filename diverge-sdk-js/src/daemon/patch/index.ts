// A JSON Patch: the one way every record of the daemon's is edited.
//
// Every `edit` names a record and carries a `Patch` — a list of
// `Operation`s, RFC 6902 — which the daemon applies, in order, to
// the record's editable members as one JSON document; the result
// must be that type again. One patch carries any number of
// operations and is applied whole or not at all.

export * from "./operation.js";
