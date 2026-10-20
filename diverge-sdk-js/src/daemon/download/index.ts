// What a download sends: one piece of one file, with the file's path.
//
// A download — an agent's, a tool's, a volume's — is a stream of
// chunks, each a piece of one file and the path of that file
// relative to what was asked for, so that a reader writes every
// chunk at its destination joined with the chunk's path and needs no
// other state.

export * from "./chunk.js";
