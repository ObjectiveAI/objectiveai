# diverge-provider-web

The Diverge Provider Protocol specification site:
<https://provider.diverge.network>.

React-authored, rendered to static HTML at build time, and **zero
JavaScript shipped** — no component carries a `client:*` directive, and
the build is checked for the absence of `<script>` tags. Optimized for
crawlers: every page has a raw-Markdown twin at its own URL (`.md`),
`/llms.txt` indexes them, `/llms-full.txt` is the whole specification
in one file, and robots.txt welcomes AI crawlers by name.

The specification revision is the version of the normative
`diverge-provider-sdk` crate, read from its `Cargo.toml` at build time.
Where prose and crate disagree, the crate is correct.

```sh
pnpm --filter diverge-provider-web dev
pnpm --filter diverge-provider-web build
```
