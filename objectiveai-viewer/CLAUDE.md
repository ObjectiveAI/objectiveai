# objectiveai-viewer

The Tauri-shell desktop app that renders ObjectiveAI streams + hosts plugin iframes.

## Styling: Tailwind CSS + `classnames`

Styling uses [Tailwind v4](https://tailwindcss.com/) via the `@tailwindcss/vite` plugin (configured in `vite.config.ts`). Global setup lives in `src/styles.css`:

```css
@import "tailwindcss";
```

There is no `tailwind.config.js` — Tailwind v4 auto-detects content from the project.

### `cn()` convention — every class is a separate argument

**Every** `className` in this project must be built via the `classnames` package's `cn()` helper, with each individual utility class passed as its own argument. **Never** put more than one Tailwind class in a single string.

```tsx
import cn from "classnames";

// ✓ Correct — one class per argument.
<div className={cn("flex", "flex-col", "h-screen", "bg-neutral-100")}>

// ✓ Correct — nested cn() for conditional groups.
<button
  className={cn(
    "px-4",
    "py-2",
    "rounded",
    isActive
      ? cn("bg-blue-600", "text-white")
      : cn("bg-neutral-200", "text-neutral-700"),
  )}
>

// ✗ Wrong — multiple classes in one string.
<div className="flex flex-col h-screen bg-neutral-100">
<div className={cn("flex flex-col", "h-screen bg-neutral-100")}>
<div className={"flex flex-col h-screen bg-neutral-100"}>
```

### Why one-class-per-arg

- **Greppable.** Searching for `"bg-blue-600"` finds every site that uses it. A space-separated string `"bg-blue-600 hover:..."` is harder to locate.
- **Mechanically diffable.** Adding/removing a single class is one line of diff, not a re-flow of a string.
- **Composable.** `cn()` accepts arrays and conditionals at arg-level; one class per arg keeps each conditional small.
- **Lint-friendly.** Each string literal is a known fixed token — Prettier won't re-flow them, and editor autocomplete works on each.

### Where to put styles

- Inline `<style>` blocks: never.
- CSS-in-JS via the `style={...}` prop: never. The only `style={...}` left in the codebase should be for values that are computed at runtime and have no Tailwind equivalent (rare).
- External `.css` files: only `styles.css` (the Tailwind entry point). The pre-Tailwind `AgentCompletionView.css` etc. are being migrated to Tailwind incrementally — new components should be Tailwind-first.

### Dark mode

Tailwind v4 honors `prefers-color-scheme: dark` out of the box via the `dark:` variant. Use `dark:bg-neutral-900` / `dark:text-neutral-50` etc. alongside the light defaults. No JS-side theme switcher; the OS theme drives it.

## Tab bar

`TabBar.tsx` is the canonical reference for the `cn()` convention applied to a real component. Look there for an example of how to structure conditional active/inactive class sets.

## Plugin discovery

The viewer scans `~/.objectiveai/plugins/<name>.json` on startup via `list_plugins_with_viewer` (Tauri command in `src-tauri/src/plugins.rs`). Plugins with either `viewer_zip` or `viewer_url` set get a tab. See `objectiveai-sdk-rs/src/filesystem/plugins/manifest.rs::Manifest` for the manifest schema.

## Hot reload during plugin authoring

When developing a plugin with `viewer_url: "http://localhost:5173"`, run your dev server (Vite / Next / whatever) on the configured port. The viewer's iframe loads the URL directly, so the dev server's hot-reload propagates straight in — edit-save-see-update with no plugin reinstall.

## Testing

- `pnpm --filter objectiveai-viewer run build` — front-end TypeScript + Vite build.
- `pnpm tauri dev` (from `objectiveai-viewer/`) — Tauri dev shell with Vite HMR.
- `bash objectiveai-viewer/test.sh` — Rust + integration tests. Uses `--lib --tests` to skip the bin target (Tauri's deps can't link in `cargo test`'s bin pass).
- `bash objectiveai-viewer/build.sh [--release]` — production build via `tauri build` (embeds the frontend + icon), lands raw in `objectiveai-viewer/embed/objectiveai-viewer(.exe)`.
