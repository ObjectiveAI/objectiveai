//! The nine requests over dependency tools, served: [`list`], [`get`],
//! [`delete`], [`tag`] and [`untag`] over the rows — one per template
//! and owner, the delete by template over every row of it — and
//! [`filetree`], [`download`], [`upload`] and [`transfer`] over a
//! running one, named by its template and whose it is. One handler
//! per request, each the shape [`serve`](crate::serve) states. What a
//! reference names is [`resolve`]d to the row and its run, or nothing:
//! no request here starts a tool, and none releases one. What a tool
//! is attached to and whether it runs — the
//! [`Facts`](crate::judge::filter::dependency_tools::Facts) a grant
//! and a filter judge by — are [`load`]ed as a [`Loaded`], and whether
//! the caller's grants [`reaches`] a tool is one test.

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
