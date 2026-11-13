//! Putting a structured decision to an agent.
//!
//! A client names an agent of its own and hands the daemon a prompt
//! and one or more typed questions, and the scope stays open until
//! the decision's fate is known: the daemon answers that the agent's
//! program has it, or that it failed, and the scope finishes. One
//! question, one answer: the client opens no channel on the scope, and
//! nothing takes a decision back once sent. The agent's answers to the
//! questions are not on this scope: the agent says them in the run the
//! decision starts, and they are read in the agent's log, numbered as
//! the decision is.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
