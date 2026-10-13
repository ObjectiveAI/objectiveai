//! The nine requests over dependency templates and the tools running
//! from them, served: [`list`], [`get`], [`delete`], [`tag`] and
//! [`untag`] over the records, and [`filetree`], [`download`],
//! [`upload`] and [`transfer`] over a running dependency tool, named
//! by its template and whose it is. One handler per request, each the
//! shape [`serve`](crate::serve) states. What a reference names is
//! [`resolve`]d to the template's record and the run, or nothing: no
//! request here starts a tool, and none releases one. What a template
//! is attached to and what runs from it — the
//! [`Facts`](crate::judge::filter::dependencies::Facts) a grant and a
//! filter judge by — are [`agents_of`], [`runs_of`] and [`run_of`],
//! and whether the caller's grants [`reaches`] a template is one test.

mod facts;
mod resolve;

pub use facts::*;
pub use resolve::*;

pub mod delete;
pub mod download;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;
