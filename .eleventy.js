const markdownIt = require("markdown-it");
const markdownItAttrs = require("markdown-it-attrs");

module.exports = function (eleventyConfig) {
  // Static assets (CSS, JS, images) and the Jekyll-disabling marker.
  eleventyConfig.addPassthroughCopy({ "site/assets": "assets" });
  eleventyConfig.addPassthroughCopy({ "site/.nojekyll": ".nojekyll" });

  // The site's own contributor README is not a page.
  eleventyConfig.ignores.add("site/README.md");
  eleventyConfig.ignores.add("site/assets/audio/README.md");

  // Markdown: allow raw HTML in pages, and `{#id}` / `{.class}` attribute syntax
  // on headings and fenced code blocks.
  eleventyConfig.setLibrary(
    "md",
    markdownIt({ html: true, linkify: false, typographer: false }).use(markdownItAttrs)
  );

  // Post-process the rendered HTML to keep the bespoke terminal styling:
  //  - wrap tables so they scroll horizontally on narrow screens
  //  - turn `language-xxx` code fences into the `data-lang` corner label
  eleventyConfig.addTransform("polish", function (content) {
    if (!(this.page.outputPath || "").endsWith(".html")) return content;
    return content
      .replace(/<table>/g, '<div class="tbl-wrap"><table>')
      .replace(/<\/table>/g, "</table></div>")
      .replace(/<pre><code class="language-(\w+)">/g, '<pre data-lang="$1"><code class="language-$1">');
  });

  return {
    dir: {
      input: "site",
      includes: "_includes",
      output: "_site",
    },
    // Pages are plain Markdown (+ raw HTML); layouts are Nunjucks.
    markdownTemplateEngine: false,
    htmlTemplateEngine: "njk",
  };
};
