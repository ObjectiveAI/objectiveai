// Rust included from the crate, never copied into the site.
//
// A specification page that shows a type shows the crate's own file,
// whole, read at build time: an empty fenced block whose meta names
// the file relative to the workspace root —
//
//     ```rust include=diverge-provider-sdk/src/container_proxy/requests/request/frame.rs
//     ```
//
// — is replaced by that file's contents. Two readers use one
// resolver: the remark plugin below fills the block for the rendered
// page, and `expandIncludes` fills it for the raw-Markdown twin, so
// both carry the same bytes. A file that does not exist fails the
// build, which is the point: a reference that went stale is a build
// that stops, not a page that quietly shows nothing.
//
// Resolved from the working directory for the reason `revision.ts`
// gives: the prerender bundle runs from `dist/`, and the build always
// runs from the package directory, one below the workspace root.

import { existsSync, readFileSync } from "node:fs";
import { resolve, sep } from "node:path";

const WORKSPACE = resolve(process.cwd(), "..");

// The revision, read from the crate's manifest the way `revision.ts`
// reads it, so a page that must STATE the revision — the version
// endpoint's answer — states the crate's and never a copy. Written
// as `%%REVISION%%` in a page; expanded in text, inline code and
// code blocks alike, for the page and for its Markdown twin.
const REVISION_TOKEN = "%%REVISION%%";
const REVISION = (() => {
  const manifest = readFileSync(resolve(WORKSPACE, "diverge-provider-sdk/Cargo.toml"), "utf-8");
  const version = manifest.match(/^version = "([^"]+)"/m);
  if (!version) {
    throw new Error("diverge-provider-sdk/Cargo.toml has no version");
  }
  return version[1];
})();

/** Every revision token in `text`, replaced by the crate's version. */
export function expandRevision(text) {
  return text.split(REVISION_TOKEN).join(REVISION);
}

/** The file named by an include, verbatim, without a trailing newline. */
export function readInclude(spec) {
  const path = resolve(WORKSPACE, spec);
  if (!path.startsWith(WORKSPACE + sep)) {
    throw new Error(`include escapes the workspace: ${spec}`);
  }
  if (!existsSync(path)) {
    throw new Error(`include does not exist: ${spec}`);
  }
  return readFileSync(path, "utf-8").replace(/\r\n/g, "\n").replace(/\n+$/, "");
}

const FENCE = /^```(\w+) include=(\S+)[ \t]*\n```[ \t]*$/gm;

/** Every empty include fence in `markdown`, filled. */
export function expandIncludes(markdown) {
  return expandRevision(
    markdown.replace(
      FENCE,
      (_, lang, spec) => "```" + lang + "\n" + readInclude(spec) + "\n```",
    ),
  );
}

/** The remark plugin: the same fill, on the syntax tree. */
export function remarkInclude() {
  return (tree) => {
    const visit = (node) => {
      if (node.type === "code" && typeof node.meta === "string") {
        const match = node.meta.match(/(?:^|\s)include=(\S+)/);
        if (match) {
          node.value = readInclude(match[1]);
          node.meta = null;
        }
      }
      if (
        (node.type === "text" || node.type === "inlineCode" || node.type === "code") &&
        typeof node.value === "string" &&
        node.value.includes(REVISION_TOKEN)
      ) {
        node.value = expandRevision(node.value);
      }
      if (Array.isArray(node.children)) {
        for (const child of node.children) {
          visit(child);
        }
      }
    };
    visit(tree);
  };
}
