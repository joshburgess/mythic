---
title: "Asset Pipeline"
---

# Asset Pipeline

Mythic includes a built-in asset pipeline that handles images, CSS, JavaScript, and static files. Assets are processed, optimized, and fingerprinted automatically during builds.

## CSS and Sass/SCSS

### Source Files

Place your stylesheets in the `styles/` directory:

```
styles/
  main.scss           # Entry point
  _variables.scss     # Partial (not compiled standalone)
  _mixins.scss        # Partial
  components/
    _buttons.scss
    _nav.scss
    _typography.scss
```

Files prefixed with `_` are partials. They are included by other files but not compiled independently.

### Sass/SCSS Support

Mythic compiles Sass and SCSS out of the box. No additional tools needed:

```scss
// styles/_variables.scss
$primary: #2563eb;
$font-stack: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;

// styles/main.scss
@use "variables" as *;

body {
  font-family: $font-stack;
  color: #1a1a1a;
  line-height: 1.6;
}

a {
  color: $primary;
  text-decoration: none;
  &:hover {
    text-decoration: underline;
  }
}

.container {
  max-width: 48rem;
  margin: 0 auto;
  padding: 0 1rem;
}
```

### CSS Minification

In production builds, CSS is automatically minified. This is controlled by the config:

```toml
[assets]
minify_css = true
```

### Using in Templates

Reference the compiled CSS in your templates:

```html
<link rel="stylesheet" href="{{ assets.css_path }}">
```

In development, this resolves to `/css/main.css`. In production with hashing enabled, it becomes `/css/main.a1b2c3d4.css`.

## JavaScript

### Source Files

Place JavaScript files in the `scripts/` directory:

```
scripts/
  main.js
  utils.js
  components/
    modal.js
    dropdown.js
```

### Bundling

Mythic bundles JavaScript files together. The entry point is `scripts/main.js`:

```javascript
// scripts/main.js
import { initModal } from './components/modal.js';
import { initDropdown } from './components/dropdown.js';

document.addEventListener('DOMContentLoaded', () => {
    initModal();
    initDropdown();
});
```

### Minification

JavaScript is minified in production builds:

```toml
[assets]
minify_js = true
```

### Using in Templates

```html
<script src="{{ assets.js_path }}" defer></script>
```

## Image Pipeline

Mythic automatically processes images referenced in your content and templates.

### Responsive Images

Configure output widths for responsive image variants:

```toml
[assets]
image_widths = [640, 960, 1280, 1920]
image_quality = 85
image_format = "webp"
```

When you reference an image in Markdown:

```markdown
![A landscape photo](/images/landscape.jpg)
```

Mythic generates multiple sizes and outputs a responsive `<picture>` element:

```html
<picture>
  <source
    type="image/webp"
    srcset="/images/landscape.640.webp 640w,
            /images/landscape.960.webp 960w,
            /images/landscape.1280.webp 1280w,
            /images/landscape.1920.webp 1920w"
    sizes="(max-width: 640px) 640px, (max-width: 960px) 960px, 1280px">
  <img src="/images/landscape.1280.webp" alt="A landscape photo" loading="lazy">
</picture>
```

### Supported Formats

Input formats: JPEG, PNG, GIF, WebP, AVIF, SVG, BMP, TIFF.

Output formats (configurable):

- `"webp"` (default): good compression, wide support
- `"avif"`: better compression, growing support
- `"jpeg"`: maximum compatibility
- `"png"`: lossless, good for diagrams

SVGs are passed through without conversion.

### Quality Settings

```toml
[assets]
image_quality = 85     # 1-100, higher = better quality, larger files
```

### Disabling Image Processing

To skip image processing for specific images, place them in the `static/` directory instead of `content/`. Files in `static/` are copied as-is.

## Content Hashing

Content hashing appends a hash of the file's contents to its filename. This ensures browsers always load the latest version after deployments.

```toml
[assets]
hash_filenames = true
```

With hashing enabled:

```
/css/main.css       -> /css/main.a1b2c3d4.css
/js/main.js         -> /js/main.e5f6a7b8.js
/images/logo.png    -> /images/logo.9c8d7e6f.png
```

Template variables (`assets.css_path`, `assets.js_path`) automatically resolve to the hashed filenames. No manual updates needed.

Content hashing is disabled during development (`mythic serve`) for easier debugging.

## Static Files

Files in the `static/` directory are copied to the output directory without any processing:

```
static/
  favicon.ico          -> /favicon.ico
  robots.txt           -> /robots.txt
  images/
    og-image.png       -> /images/og-image.png
  fonts/
    inter.woff2        -> /fonts/inter.woff2
```

Use `static/` for files that should not be processed: favicons, web fonts, third-party libraries, verification files, and pre-optimized assets.

```toml
[assets]
copy_static = true    # default
```

## Build Output

After a production build, the output directory contains all processed assets:

```
public/
  index.html
  css/
    main.a1b2c3d4.css
  js/
    main.e5f6a7b8.js
  images/
    landscape.640.webp
    landscape.960.webp
    landscape.1280.webp
    landscape.1920.webp
    logo.9c8d7e6f.png
  favicon.ico
  robots.txt
```

## Performance

The asset pipeline runs in parallel with template rendering. On a typical site:

- Sass compilation: ~50ms
- JavaScript bundling: ~30ms
- Image processing: varies (cached between builds)

Incremental builds only reprocess assets that have changed, keeping rebuild times under 100ms for most edits.
