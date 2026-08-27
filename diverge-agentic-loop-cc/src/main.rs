//! The `claude_code` agentic loop, as a container.
//!
//! The program an `agentic_loop::run` server deploys for an agent
//! whose `upstream` is `claude_code`, per the Container section of the
//! provider specification: the loop served over HTTP and SSE on port
//! 8080, MCP asked on port 8081, Postgres opened to port 8082.

// The harness that consumes them comes later; the allows leave with it.
#[allow(dead_code)]
mod continuation;
#[allow(dead_code)]
mod response;

fn main() {}
