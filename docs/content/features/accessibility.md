---
title: "Accessibility Auditing"
---

# Accessibility Auditing

Mythic includes a built-in accessibility (a11y) auditor that checks your rendered HTML for common accessibility issues. These checks run as part of the `mythic check` command and help you catch problems before they reach production.

## Running the Audit

The accessibility audit is included when you run:

```bash
mythic check
```

This command runs all available checks, including accessibility, linting, and link validation. Output looks like:

```
  Checking accessibility...
  Error: content/blog/my-post.md: <img> missing alt attribute (line 42)
  Warning: content/about.md: heading level skipped (h2 -> h4) (line 18)
  Warning: templates/base.tera.html: <html> missing lang attribute
  Checked 24 pages, 1 error, 2 warnings
```

## Checks Performed

Mythic runs the following accessibility checks on every rendered page.

### Image Alt Text (Error)

Every `<img>` element must have an `alt` attribute. Decorative images should use an empty `alt=""` rather than omitting the attribute entirely.

```html
<!-- Error: missing alt -->
<img src="/images/photo.jpg">

<!-- OK: descriptive alt -->
<img src="/images/photo.jpg" alt="A sunset over the mountains">

<!-- OK: decorative image with empty alt -->
<img src="/images/divider.png" alt="">
```

**Fix:** Add an `alt` attribute to every image in your Markdown or templates. In Markdown, the alt text is the text inside the brackets: `![alt text here](/images/photo.jpg)`.

### HTML Lang Attribute (Error)

The `<html>` element must have a `lang` attribute specifying the document language. This helps screen readers select the correct pronunciation rules.

```html
<!-- Error -->
<html>

<!-- OK -->
<html lang="en">
```

**Fix:** Add `lang="{{ site.language }}"` to the `<html>` tag in your base template.

### Heading Order (Warning)

Headings must not skip levels. For example, an `<h2>` should not be followed directly by an `<h4>` without an intervening `<h3>`. Skipped heading levels confuse screen readers and assistive navigation tools.

```markdown
## Section Title
#### Subsection Title   <!-- Warning: skipped h3 -->
```

**Fix:** Use sequential heading levels. If you need smaller visual text, use CSS rather than skipping heading levels.

### Empty Links (Warning)

Links must have discernible text content. A link with no text or only whitespace is not navigable by screen reader users.

```html
<!-- Warning: empty link -->
<a href="/about"></a>

<!-- OK -->
<a href="/about">About Us</a>

<!-- OK: using aria-label -->
<a href="/about" aria-label="About Us"><svg>...</svg></a>
```

**Fix:** Add visible text, an `aria-label`, or an `aria-labelledby` attribute to every link.

### Form Labels (Warning)

Every form input (except `type="hidden"`) must have an associated `<label>` element or an `aria-label` attribute. Unlabeled inputs are difficult or impossible for screen reader users to interact with.

```html
<!-- Warning: no label -->
<input type="text" name="email">

<!-- OK: explicit label -->
<label for="email">Email</label>
<input type="text" id="email" name="email">

<!-- OK: aria-label -->
<input type="text" name="email" aria-label="Email address">
```

**Fix:** Add a `<label>` element with a matching `for` attribute, or add an `aria-label` to the input.

### Viewport Meta (Warning)

The page must include a `<meta name="viewport">` tag to ensure proper rendering on mobile devices. Without it, mobile browsers may scale the page unpredictably.

```html
<!-- OK -->
<meta name="viewport" content="width=device-width, initial-scale=1">
```

**Fix:** Add the viewport meta tag to the `<head>` section of your base template.

### Zoom Restriction (Warning)

The viewport meta tag should not disable user zooming. Setting `maximum-scale=1` or `user-scalable=no` prevents users with low vision from enlarging text.

```html
<!-- Warning: zoom disabled -->
<meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">

<!-- OK -->
<meta name="viewport" content="width=device-width, initial-scale=1">
```

**Fix:** Remove `user-scalable=no` and `maximum-scale=1` from your viewport meta tag.

## Severity Levels

| Severity | Meaning                                                              |
|----------|----------------------------------------------------------------------|
| Error    | A significant accessibility barrier. Should be fixed before deploy.  |
| Warning  | A best-practice violation. Recommended to fix but not blocking.      |

Errors indicate issues that will prevent some users from accessing your content. Warnings indicate areas where accessibility could be improved but the content is still technically usable.

## Summary of Rules

| Rule                | Severity | What it checks                                      |
|---------------------|----------|-----------------------------------------------------|
| Image alt text      | Error    | Every `<img>` has an `alt` attribute                |
| HTML lang           | Error    | `<html>` has a `lang` attribute                     |
| Heading order       | Warning  | Headings do not skip levels                         |
| Empty links         | Warning  | All links have discernible text                     |
| Form labels         | Warning  | Form inputs have associated labels                  |
| Viewport meta       | Warning  | Page includes viewport meta tag                     |
| Zoom restriction    | Warning  | Viewport does not disable user zoom                 |
