import { readFileSync } from "node:fs";
import { resolve } from "node:path";

// The specification revision IS the sdk version. The crate is the
// normative artifact — where prose and crate disagree, the crate is
// correct — so the revision is read out of the crate's own manifest at
// build time rather than written down a second time and left to drift.
//
// Resolved from the working directory rather than from this module's
// URL: the build bundles this file into a prerender entry that runs
// from `dist/`, so `import.meta.url` points nowhere useful, while the
// build always runs from the package directory.
const manifest = readFileSync(
  resolve(process.cwd(), "../diverge-sdk-rs/Cargo.toml"),
  "utf-8",
);

const version = manifest.match(/^version = "([^"]+)"/m);
if (!version) {
  throw new Error("diverge-sdk-rs/Cargo.toml has no version");
}

/** The current specification revision: the sdk crate's version. */
export const REVISION: string = version[1];

/** The site's absolute origin, for artifacts that need absolute URLs. */
export const ORIGIN = "https://provider.diverge.network";
