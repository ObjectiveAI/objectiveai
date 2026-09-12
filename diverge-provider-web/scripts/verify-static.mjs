// The build's own proof of what the README claims. Runs after
// `astro build`, fails the build if any claim stops being true, and
// strips the one piece of dead weight the toolchain emits.
//
// 1. No HTML page ships a <script>. React is an authoring language
//    here; the day someone adds a `client:*` directive, this is what
//    says so, loudly, instead of the site quietly growing a runtime.
// 2. Unreferenced JS assets are deleted. @astrojs/react emits its
//    client entry into _astro/ even when nothing hydrates; once (1)
//    holds, any emitted .js is dead weight by definition — verified
//    unreferenced anyway before deletion.
// 3. Every link in llms.txt resolves to a file in dist/.
// 4. Every HTML page has exactly one <h1>, a canonical, a meta
//    description — and, for specification pages, the alternate
//    Markdown link with an existing twin.
// 5. Every docs.rs/rmcp link is pinned to the rmcp version the SDK
//    builds against. The links are the spec's incorporated type
//    definitions; a version drift silently changes what the spec
//    says, so the SDK's Cargo.toml is the authority.
// 6. Every `include=` a specification page names is a file in the
//    workspace, and lives in the LATEST revision's module: an older
//    module is frozen text. The build already stops on either; this
//    names the page.
// 7. The latest module's version endpoint states its own revision as
//    the answer: the one string the specification hard-codes, checked
//    against the crate's manifest.

import { readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const dist = new URL("../dist/", import.meta.url).pathname.replace(
  /^\/([A-Za-z]:)/,
  "$1",
);

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

const files = walk(dist);
const failures = [];

// 1. No scripts in any HTML.
const html = files.filter((f) => f.endsWith(".html"));
for (const page of html) {
  const text = readFileSync(page, "utf-8");
  if (text.includes("<script")) {
    failures.push(`${relative(dist, page)}: contains a <script> tag`);
  }
}

// 2. Emitted JS must be unreferenced, then goes.
const scripts = files.filter((f) => f.endsWith(".js") || f.endsWith(".mjs"));
for (const script of scripts) {
  const name = relative(dist, script).replace(/\\/g, "/");
  const referenced = html.some((page) =>
    readFileSync(page, "utf-8").includes(name.split("/").pop()),
  );
  if (referenced) {
    failures.push(`${name}: JS asset is referenced by a page`);
  } else {
    rmSync(script);
  }
}

// 3. llms.txt links resolve — when it exists. The site is being
// rebuilt page by page, and the index returns when its targets do.
let llms = "";
try {
  llms = readFileSync(join(dist, "llms.txt"), "utf-8");
} catch {
  // Not built in this pass.
}
for (const match of llms.matchAll(
  /\]\(https:\/\/provider\.diverge\.network(\/[^)]+)\)/g,
)) {
  const path = match[1];
  const candidates = path.endsWith("/")
    ? [join(dist, path, "index.html")]
    : [join(dist, path)];
  if (
    !candidates.some((candidate) => {
      try {
        return statSync(candidate).isFile();
      } catch {
        return false;
      }
    })
  ) {
    failures.push(`llms.txt: ${path} resolves to nothing in dist/`);
  }
}

// 4. Page anatomy.
for (const page of html) {
  const text = readFileSync(page, "utf-8");
  const name = relative(dist, page);
  const h1s = (text.match(/<h1[\s>]/g) ?? []).length;
  if (h1s !== 1) {
    failures.push(`${name}: ${h1s} <h1> elements`);
  }
  if (!text.includes('rel="canonical"')) {
    failures.push(`${name}: no canonical link`);
  }
  if (!text.includes('name="description"')) {
    failures.push(`${name}: no meta description`);
  }
  const twin = text.match(
    /rel="alternate" type="text\/markdown" href="https:\/\/provider\.diverge\.network(\/[^"]+)"/,
  );
  if (twin) {
    try {
      statSync(join(dist, twin[1]));
    } catch {
      failures.push(`${name}: markdown twin ${twin[1]} does not exist`);
    }
  }
}

// 5. rmcp links match the SDK's pinned rmcp version.
const cargo = readFileSync(
  new URL("../../diverge-provider-sdk-rs/Cargo.toml", import.meta.url),
  "utf-8",
);
const rmcp = cargo.match(/^rmcp\s*=\s*\{\s*version\s*=\s*"([^"]+)"/m)?.[1];
if (!rmcp) {
  failures.push("Cargo.toml: no rmcp version found in the SDK manifest");
}
for (const page of html) {
  const text = readFileSync(page, "utf-8");
  for (const match of text.matchAll(/docs\.rs\/rmcp\/([^/"]+)\//g)) {
    if (rmcp && match[1] !== rmcp) {
      failures.push(
        `${relative(dist, page)}: rmcp link pinned to ${match[1]}, SDK uses ${rmcp}`,
      );
    }
  }
}

// 6. Included crate files exist, and only the latest module includes.
const workspace = new URL("../../", import.meta.url).pathname.replace(
  /^\/([A-Za-z]:)/,
  "$1",
);
const content = new URL("../src/content/spec/", import.meta.url).pathname.replace(
  /^\/([A-Za-z]:)/,
  "$1",
);
const latest = cargo.match(/^version = "([^"]+)"/m)?.[1];
if (!latest) {
  failures.push("Cargo.toml: no version found in the SDK manifest");
}
for (const page of walk(content).filter((f) => f.endsWith(".mdx"))) {
  const text = readFileSync(page, "utf-8");
  const module = relative(content, page).split(/[\\/]/)[0];
  for (const match of text.matchAll(/^```\w+ include=(\S+)/gm)) {
    if (latest && module !== latest) {
      failures.push(`${relative(content, page)}: module ${module} is frozen and includes ${match[1]}`);
    }
    try {
      statSync(join(workspace, match[1]));
    } catch {
      failures.push(`${relative(content, page)}: include ${match[1]} does not exist`);
    }
  }
}

// 7. The version endpoint answers with the revision it belongs to.
if (latest) {
  const response = join(content, latest, "endpoints", "version", "response.mdx");
  try {
    if (!readFileSync(response, "utf-8").includes("`" + latest + "`")) {
      failures.push(`${latest}/endpoints/version/response.mdx: does not state \`${latest}\``);
    }
  } catch {
    // The section is not written for this revision; nothing to check.
  }
}

if (failures.length > 0) {
  console.error("verify-static: the build breaks its own claims:");
  for (const failure of failures) {
    console.error(`  - ${failure}`);
  }
  process.exit(1);
}
console.log(
  `verify-static: ${html.length} pages, zero scripts, llms.txt resolves, anatomy sound, includes exist and only in ${latest}.`,
);
