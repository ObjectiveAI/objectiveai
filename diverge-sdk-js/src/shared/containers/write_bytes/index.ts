// Asking for a write's content, and streaming it: the half of a
// write that travels the other way. A server opens a channel asking
// for the content, and the client streams pieces back until it
// finishes the channel — so a read's body goes out as one piece, no
// acknowledgement per piece, no length either end has to know in
// advance, and a consumer holds one frame at a time however large
// the file is.

export * as response from "./response/index.js";
