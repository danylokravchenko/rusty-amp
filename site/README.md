# rusty-amp docs site

Source for the GitHub Pages site at <https://danylokravchenko.github.io/rusty-amp/>.

Hand-written static HTML sharing `assets/site.css` and `assets/site.js`. Edit the
`*.html` files directly; the nav block is repeated in each page, so keep it in
sync when adding a page.

## Preview locally

```bash
cd site && python3 -m http.server
# open http://localhost:8000
```

## Deploy

Pushed to GitHub Pages automatically by `.github/workflows/pages.yml` on any change
under `site/`.
