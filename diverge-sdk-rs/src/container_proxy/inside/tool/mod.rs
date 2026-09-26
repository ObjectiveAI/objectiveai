//! The tool container's own server: what the proxy dials.
//!
//! A tool container's program is an HTTP server on the container's
//! loopback, at [`port()`](crate::container_proxy::inside::port), and the proxy beside it is
//! its only caller. This module is that server's contract — the two
//! calls every container answers, and the MCP server beside them —
//! stated once, for the program that answers them and the proxy that
//! makes them.
//!
//! | the proxy calls | with | the program answers |
//! |-----------------|------|---------------------|
//! | `POST /register` | the [`register::request::Request`](crate::container_proxy::inside::register::request::Request) JSON | `2xx` with the [`register::response::Response`](crate::container_proxy::inside::register::response::Response) JSON, the tools the program depends on, the arguments held for the container's life; or a non-`2xx` |
//! | `GET /schema` | nothing | `2xx` with the JSON Schema of the arguments; or a non-`2xx` |
//! | `/mcp` | MCP over Streamable HTTP | the server itself: every exchange the caller opens on the provider, made by the proxy's one client |
//!
//! # Registration comes first, and once
//!
//! The arguments are fixed for the container's life, and so are the
//! tools the program answers with. The proxy registers them exactly
//! once, before it dials the MCP server; the program refuses any
//! second `/register`, whatever it carries (`{"kind":"registered"}`,
//! `409`). What the program makes of the arguments — which tools it
//! serves, against what — is its own.
//!
//! # The MCP server is the program's
//!
//! The proxy is one MCP client to it, dialled on the first exchange
//! and kept while its transport lives, and every notification the
//! server sends is heard by that client and fanned out to whoever
//! asked. Nothing of the server's is otherwise kept, and the
//! provider's caller speaks to it in MCP's own vocabulary.
