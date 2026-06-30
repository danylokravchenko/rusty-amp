# rusty-amp docs site

Source for the documentation site at <https://danylokravchenko.github.io/rusty-amp/>.
Built with [Eleventy](https://www.11ty.dev/): pages are **Markdown** (with a little
inline HTML for the bespoke terminal components), rendered into a shared layout.

## Layout

```text
site/
  _includes/
    base.njk        ← <head>, nav, footer, scripts (defined once)
    page.njk        ← inner-page chrome: page-head (eyebrow/heading/lead/toc) + prev/next pager
  assets/
    site.css        ← all styling (the terminal theme)
    site.js         ← mobile nav + active-link highlighting
    *.png / *.svg   ← screenshot & social images
  index.md          ← landing page (uses base.njk, hand-built hero)
  getting-started.md
  pedals.md
  amps-cabs.md
  presets.md
  plugins.md
  how-it-works.md
  README.md         ← this file (not published)
.eleventy.js        ← build config (repo root)
package.json        ← repo root
```

## Editing

- **Prose & tables** → write plain Markdown. Tables are auto-wrapped for horizontal
  scroll; fenced code blocks (```` ```bash ````) get a corner language label.
- **Section anchors** → add `{#id}` after a heading, e.g. `## Cabinet mics {#mics}`.
  Cross-page links rely on these ids, so keep them stable.
- **Per-page metadata** (title, description, `eyebrow`, `heading`, `lead`, `toc`,
  `prev`/`next`) lives in each file's YAML front matter.
- **The nav** is defined once in `_includes/base.njk` — add new pages there.
- **Bespoke components** (cards, pedal grid, panels, notes, knobs) are plain HTML
  classes from `site.css`; drop them into the Markdown where needed.

## Preview locally

```bash
npm install
npm run dev        # serves at http://localhost:8080 with live reload
# or one-off build into ./_site:
npm run build
```

## Deploy

Pushing to `main` with changes under `site/` (or `.eleventy.js` / `package.json`)
triggers `.github/workflows/pages.yml`, which runs the Eleventy build and publishes
`_site/` to GitHub Pages. The site **Source** must be set to *GitHub Actions* in the
repo's **Settings → Pages**.
