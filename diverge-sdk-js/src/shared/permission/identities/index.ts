// Over the host's identities.
//
// The identities are the files under the root's `identities/`, each
// one subject at one issuer, signed in on the host: no request signs
// one in or removes one. A grant over them lists them, and nothing
// else: on the wire a bare array of the `Action`s held,
// `{"identities":["issuers","subjects"]}`.

export * from "./action.js";
export * from "./permission.js";
