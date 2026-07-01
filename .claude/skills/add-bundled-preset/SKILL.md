---
name: add-bundled-preset
description: Add a new pre-built (bundled) preset to rusty-amp. Use when asked to create, add, or contribute a bundled/artist/factory preset — a .toml tone shipped with the repo. Covers writing the preset file AND updating the docs site so the two never drift.
---

# Add a bundled preset to rusty-amp

A bundled preset is a read-only `.toml` tone shipped with the repo in `presets/`. Adding one is **two coordinated changes**: the preset file itself, and the documentation site. Both must land in the same change — a preset that isn't documented is treated as incomplete.

## Steps

### 1. Write the preset file

Create `presets/<name>.toml`. Use a short, lowercase, filesystem-safe file name (e.g. `pantera_floods.toml`); the file stem is what appears in the preset browser list.

- Study an existing preset in `presets/` (e.g. `presets/metallica.toml`) as a template — match its structure and its habit of a one-line `#` comment above each section explaining *why* the values are what they are.
- Only `name`, `description`, and the always-present sections are required; every other section is optional (see the schema below). Omitting a section leaves that effect's current state unchanged.
- All knob values are normalised `0.0`–`1.0`.
- `name` is the display name (can be long/descriptive); `description` is the one-line blurb shown in the browser.

See [full schema](../../../site/presets.md) for the TOML structure and optional sections.

> Confirm the exact set of required sections and any new fields (e.g. amp `presence`) against `src/preset.rs`, which is the source of truth for what deserializes. The site docs can lag behind the code.

### 2. Test it in the app

```bash
cargo test --package rusty-amp --lib -- preset::tests
```
Bundled presets can't be deleted by users, so only add tones that are genuinely useful and well-tuned.

### 3. Update the documentation site (required, same change)

The user-facing docs live in `site/`. The **bundled presets table** in [`site/presets.md`](../../../site/presets.md) lists every shipped preset and must include the new one. Add a row under the `## Bundled presets {#bundled}` table:

```
| `<name>.toml` | <Amp label> | <Cabinet label> | <one-line description> |
```

- Keep the columns consistent with existing rows: file name in backticks, human-readable amp and cabinet names (e.g. "Marshall JCM800", "Mesa V30"), and a short description.
- If the new preset would appear in the illustrative **preset browser mock-up** higher in `site/presets.md` (the `<div class="plist">` block), you generally leave that alone — it's a representative example, not an exhaustive list. Only touch it if explicitly asked.

Then verify the site builds:

```bash
cd site && npm run build   # or: npm run dev  to preview locally
```

Confirm the new row renders in the bundled-presets table.

### 4. Ship both together

The preset `.toml` and the `site/presets.md` table row belong in the **same PR**. See `CONTRIBUTING.md` → "Adding a bundled preset". Run the usual checks before opening the PR:

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
```

## Checklist

- [ ] `presets/<name>.toml` created, with per-section *why* comments
- [ ] Required sections present; values normalised `0.0`–`1.0`
- [ ] Tested via cargo test
- [ ] Row added to the bundled presets table in `site/presets.md`
- [ ] `npm run build` in `site/` succeeds and the row renders
- [ ] Preset + docs in the same PR
