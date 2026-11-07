//! The requests over remote daemons' records, served: [`list`], kept
//! open; [`get`]; [`add`], answering the record's ID; [`edit`], of the
//! description alone; [`delete`], refused while a connected tool names
//! the record; [`tag`] and [`untag`]. Every record is found by its ID.
//! Every write is told to the lists kept open, and a delete drops the
//! record's connection, if one is held.

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;
