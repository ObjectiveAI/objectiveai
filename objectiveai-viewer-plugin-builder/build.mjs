// REFERENCE EXAMPLE — the build step of a viewer extension.
//
// Nothing in ObjectiveAI reads this file; it is here to be copied into
// a plugin repo and edited. See ./Containerfile for how it runs and
// what the host does with its output.
//
// What a viewer build owes the host: a DIRECTORY (`viewer_output` in
// the plugin's objectiveai.json) whose contents are the built assets —
// every tab `module` and the `icon` the manifest declares, at the
// paths it declares them, relative to that directory. No archive, no
// manifest rewriting: the host packs the directory and ships the
// repo's manifest verbatim, so the manifest must already describe the
// BUILT layout (`"module": "./home.js"`, not `"./home.tsx"`).
//
// THE ONE RULE: react and its subpath specifiers stay EXTERNAL. The
// host viewer serves them through an import map so every plugin shares
// its single React instance; a bundle carrying its own copy dies on
// the first hook. Both halves below enforce it — the package.json
// strip (so pnpm can't reinstall them) and the `--external:` flags.

import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

/** The plugin's repo, as COPY'd into the image. */
const SRC = process.env.OBJECTIVEAI_SRC ?? "/src";
/** Where the built assets land — the `viewer_output` the host copies. */
const OUT = process.env.OBJECTIVEAI_STAGING ?? "/dist";

/** The JS root inside the repo: the folder holding package.json. */
const VIEWER_ROOT = "viewer";

/** Entry points, relative to VIEWER_ROOT. Each becomes `<stem>.js` in
 * OUT — which is what the manifest's tab `module` paths must name. */
const ENTRIES = ["home.tsx", "greet.tsx"];

/** Host-provided packages: stripped from every dependency set (STRIP,
 * not move-to-peer — pnpm's auto-install-peers would reinstall them)
 * and left external by the bundle. */
const HOST_PACKAGES = ["react", "react-dom"];

/** Bare specifiers the bundle must leave external — a superset of
 * HOST_PACKAGES: the subpath imports too. */
const EXTERNALS = [
  "react",
  "react-dom",
  "react/jsx-runtime",
  "react-dom/client",
];

/** Run a tool, streaming nothing, failing loudly with its output tail. */
function run(program, args, cwd, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(program, args, {
      cwd,
      env: { ...process.env, ...env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let combined = "";
    child.stdout.on("data", (chunk) => (combined += chunk));
    child.stderr.on("data", (chunk) => (combined += chunk));
    child.on("error", (e) => reject(new Error(`spawn ${program}: ${e.message}`)));
    child.on("close", (code) =>
      code === 0
        ? resolve()
        : reject(
            new Error(
              `${program} exited with ${code}: ${combined.slice(-4096).trim()}`,
            ),
          ),
    );
  });
}

const viewerRoot = path.join(SRC, VIEWER_ROOT);

// 1. Strip the host-provided packages so pnpm never installs a second
//    React into the tree the bundler resolves against.
const packageJson = path.join(viewerRoot, "package.json");
const manifest = JSON.parse(await fs.readFile(packageJson, "utf8"));
for (const section of ["dependencies", "devDependencies", "peerDependencies"]) {
  const deps = manifest[section];
  if (deps && typeof deps === "object") {
    for (const pkg of HOST_PACKAGES) delete deps[pkg];
  }
}
await fs.writeFile(packageJson, `${JSON.stringify(manifest, null, 2)}\n`);

// 2. Install the plugin's own dependencies.
await run("pnpm", ["install", "--ignore-workspace", "--no-frozen-lockfile"], viewerRoot, {
  COREPACK_ENABLE_STRICT: "0",
});

// 3. Bundle. `--outbase=.` keeps each entry's relative path, so
//    `home.tsx` → `<OUT>/home.js`.
await fs.mkdir(OUT, { recursive: true });
await run(
  "esbuild",
  [
    ...ENTRIES,
    "--bundle",
    "--format=esm",
    "--platform=browser",
    `--outdir=${OUT}`,
    "--outbase=.",
    "--jsx=automatic",
    ...EXTERNALS.map((pkg) => `--external:${pkg}`),
  ],
  viewerRoot,
);

// 4. Anything else the manifest declares — the icon, static assets —
//    has to be in OUT too, at the path the manifest names.
for (const asset of ["icon.svg"]) {
  await fs
    .copyFile(path.join(viewerRoot, asset), path.join(OUT, asset))
    .catch(() => {});
}
