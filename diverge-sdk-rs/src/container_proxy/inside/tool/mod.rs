//! The tool container's own server: what the proxy dials.
//!
//! A tool container's program is an HTTP server on the container's
//! loopback, at [`port()`](crate::container_proxy::inside::port), and the proxy beside it is
//! its only caller. This module is that server's contract — the
//! calls every container answers, and the MCP server beside them —
//! stated once, for the program that answers them and the proxy that
//! makes them. What the arguments may be is not among them: that is
//! the image's `/DIVERGE.md`, which the proxy reads from the root and
//! the program is never asked for.
//!
//! | the proxy calls | with | the program answers |
//! |-----------------|------|---------------------|
//! | `POST /register` | the [`register::request::Request`](crate::container_proxy::inside::register::request::Request) JSON | `2xx` with the [`register::response::Response`](crate::container_proxy::inside::register::response::Response) JSON, the secrets the program takes — its dependencies ignored — the arguments and the template id held for the container's life; or a non-`2xx` |
//! | `POST /secrets` | the [`secrets::feed::request::Request`](crate::container_proxy::inside::secrets::feed::request::Request) JSON | `2xx` once the values are held, its body unread; or a non-`2xx` |
//! | `POST /secrets/collect` | the [`secrets::collect::request::Request`](crate::container_proxy::inside::secrets::collect::request::Request) JSON | `2xx` with the [`secrets::collect::response::Response`](crate::container_proxy::inside::secrets::collect::response::Response) JSON, every name asked for with its value; or a non-`2xx` |
//! | `/mcp` | MCP over Streamable HTTP | the server itself: every exchange the caller opens on the provider, made by the proxy's one client |
//!
//! # Registration comes first, and once
//!
//! The arguments are fixed for the container's life, and so are the
//! modes the program declares its secrets in. The proxy registers
//! them exactly once, before it dials the MCP server; the program
//! refuses any second `/register`, whatever it carries
//! (`{"kind":"registered"}`, `409`). The secrets' values are not
//! fixed: they are fed on `POST /secrets` once the registration is
//! answered and again at every activation — an MCP exchange about to
//! be delivered when none is in flight — and the `rotating` ones are
//! collected on `POST /secrets/collect` at every inactive — the last
//! exchange in flight answered — so a value the program changed
//! while serving is the value it is fed next. What the program makes
//! of the arguments — which tools it serves, against what — is its
//! own.
//!
//! # The MCP server is the program's
//!
//! The proxy is one MCP client to it, dialled on the first exchange
//! and kept while its transport lives, and every notification the
//! server sends is heard by that client and fanned out to whoever
//! asked. Nothing of the server's is otherwise kept, and the
//! provider's caller speaks to it in MCP's own vocabulary.
