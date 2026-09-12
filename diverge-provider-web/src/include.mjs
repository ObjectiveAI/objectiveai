// What the build does to a module's text: Rust included from the
// crate, and links carried into the module's version.
//
// A specification page that shows a type shows the crate's own file,
// whole, read at build time: an empty fenced block whose meta names
// the file relative to the workspace root —
//
//     ```rust include=diverge-provider-sdk-rs/src/container_proxy/requests/request/frame.rs
//     ```
//
// — is replaced by that file's contents. Only the LATEST module may
// do so — the crate at HEAD is that revision's crate, and an older
// module is frozen text — so an include anywhere else fails the
// build. A file that does not exist fails it too: a reference that
// went stale is a build that stops, not a page that quietly shows
// nothing.
//
// A module's links are written version-relative — `/proxy/fuse/` —
// so a module is copied whole to begin the next revision; the build
// prefixes each with the module's version, `/2.3.0/proxy/fuse/`.
// The two site-wide indexes, `/llms.txt` and `/llms-full.txt`, are
// left as they are.
//
// Two readers use one resolver for each: the remark plugins below
// work the syntax tree for the rendered page, and `expandIncludes` /
// `expandLinks` work the raw text for the Markdown twin, so both
// carry the same bytes.
//
// Resolved from the working directory for the reason `revision.ts`
// gives: the prerender bundle runs from `dist/`, and the build always
// runs from the package directory, one below the workspace root.

import { existsSync, readFileSync } from "node:fs";
import { resolve, sep } from "node:path";

const WORKSPACE = resolve(process.cwd(), "..");

/** The latest revision: the crate's version. */
export const LATEST = (() => {
  const manifest = readFileSync(resolve(WORKSPACE, "diverge-provider-sdk-rs/Cargo.toml"), "utf-8");
  const version = manifest.match(/^version = "([^"]+)"/m);
  if (!version) {
    throw new Error("diverge-provider-sdk-rs/Cargo.toml has no version");
  }
  return version[1];
})();

/** The module a content file belongs to — its version — or `null`
 * for a file outside `src/content/spec/`. */
export function moduleOf(filePath) {
  const normalized = String(filePath ?? "").replace(/\\/g, "/");
  const match = normalized.match(/\/src\/content\/spec\/([^/]+)\//);
  return match ? match[1] : null;
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

/** Every empty include fence in `markdown`, filled. `version` is the
 * module the text belongs to; a frozen module may not include. */
export function expandIncludes(markdown, version) {
  return markdown.replace(FENCE, (_, lang, spec) => {
    frozen(version, spec);
    return "```" + lang + "\n" + readInclude(spec) + "\n```";
  });
}

/** Refuse an include outside the latest module. */
function frozen(version, spec) {
  if (version && version !== LATEST) {
    throw new Error(
      `module ${version} is frozen and cannot include ${spec}; only ${LATEST} includes from the crate`,
    );
  }
}

/** A site-wide path, which no module owns. */
function siteWide(url) {
  return url === "/llms.txt" || url === "/llms-full.txt";
}

/** Every root-relative link in `markdown`, carried into `version`. */
export function expandLinks(markdown, version) {
  return markdown.replace(/\]\((\/[^)\s]*)\)/g, (whole, url) =>
    siteWide(url) ? whole : `](/${version}${url})`,
  );
}

function walk(node, visit) {
  visit(node);
  if (Array.isArray(node.children)) {
    for (const child of node.children) {
      walk(child, visit);
    }
  }
}

/** The remark plugin: the include fill, on the syntax tree. */
export function remarkInclude() {
  return (tree, file) => {
    const version = moduleOf(file.path);
    walk(tree, (node) => {
      if (node.type === "code" && typeof node.meta === "string") {
        const match = node.meta.match(/(?:^|\s)include=(\S+)/);
        if (match) {
          frozen(version, match[1]);
          node.value = readInclude(match[1]);
          node.meta = null;
        }
      }
    });
  };
}

/** The remark plugin: root-relative links carried into the module's
 * version. */
export function remarkVersionLinks() {
  return (tree, file) => {
    const version = moduleOf(file.path);
    if (!version) {
      return;
    }
    walk(tree, (node) => {
      if (
        (node.type === "link" || node.type === "definition") &&
        typeof node.url === "string" &&
        node.url.startsWith("/") &&
        !siteWide(node.url)
      ) {
        node.url = `/${version}${node.url}`;
      }
    });
  };
}
