//! The requests over remote daemons' records, served: [`list`], kept
//! open; [`get`]; [`add`]; [`edit`]; [`delete`], refused while a
//! connected tool names the record; [`tag`] and [`untag`]. Every write
//! is told to the lists kept open, and a change or a delete drops the
//! record's connection, if one is held.

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
