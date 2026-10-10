//! A template: what an agent, or a tool, is made from, held by its
//! hash.
//!
//! A template is everything about an agent or a tool that is not its
//! name, not the provider it runs on and not its mounts: the image,
//! the limits, the arguments, and — in words, for whoever makes one —
//! what the create has to bring that the template cannot name — so
//! that a template is shareable, the same template on any daemon
//! hashing the same. The shape is one, [`Template`], written once
//! here; which of the two it is for is its `type`, the first member,
//! `"agent"` for an [agent
//! template](crate::daemon::endpoints::agents::templates) and `"tool"`
//! for a [tool template](crate::daemon::endpoints::tools::templates),
//! so the two kinds never hash the same and a template's text says what
//! it is for. Each family makes, lists and deletes its own. The account
//! a container runs under is not a template's but a create's, and so
//! is every volume it mounts.
//!
//! # The id is the template's hash
//!
//! The lowercase hexadecimal SHA-256 of the template's
//! [`canonical`](crate::shared::canonical) bytes — its compact JSON,
//! absent members omitted, the image's `references` absent too, since
//! one image under different names is one template, no whitespace,
//! every object key sorted at every depth, the `arguments` and every
//! map inside them included; see [`Template::hashed`]. Sixty-four
//! characters. The daemon computes it on a create and
//! answers it; a caller may compute it the same way and need not.

mod template;

pub use template::*;
