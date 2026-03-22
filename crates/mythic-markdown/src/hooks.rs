//! Markdown render hooks for customizing how elements are rendered.
//!
//! Render hooks allow overriding the HTML output for links, images,
//! and headings. If a hook is not set, the default rendering is used.

/// Type alias for render hook functions.
pub type HookFn = Box<dyn Fn(&str, &str, Option<&str>) -> String + Send + Sync>;

/// A set of render hooks that customize markdown element output.
#[derive(Default)]
pub struct RenderHooks {
    /// Custom link renderer. Receives (url, text, title) → HTML.
    pub link: Option<HookFn>,
    /// Custom image renderer. Receives (src, alt, title) → HTML.
    pub image: Option<HookFn>,
}

/// Apply render hooks to generated HTML by post-processing.
///
/// This transforms `<a>` and `<img>` tags using the provided hooks.
pub fn apply_hooks(html: &str, hooks: &RenderHooks) -> String {
    let mut result = html.to_string();

    if let Some(ref image_hook) = hooks.image {
        result = transform_images(&result, image_hook);
    }

    if let Some(ref link_hook) = hooks.link {
        result = transform_links(&result, link_hook);
    }

    result
}

fn transform_images(html: &str, hook: &dyn Fn(&str, &str, Option<&str>) -> String) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<img") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start..];

        if let Some(end) = after.find('>') {
            let tag = &after[..=end];
            let src = extract_attr(tag, "src").unwrap_or_default();
            let alt = extract_attr(tag, "alt").unwrap_or_default();
            let title = extract_attr(tag, "title");

            result.push_str(&hook(&src, &alt, title.as_deref()));
            remaining = &after[end + 1..];
        } else {
            result.push_str(&remaining[start..]);
            break;
        }
    }

    result.push_str(remaining);
    result
}

fn transform_links(html: &str, hook: &dyn Fn(&str, &str, Option<&str>) -> String) -> String {
    let mut result = String::with_capacity(html.len());
    let mut remaining = html;

    while let Some(start) = remaining.find("<a ") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start..];

        if let Some(close_tag) = after.find("</a>") {
            let full = &after[..close_tag + 4];

            if let Some(open_end) = after.find('>') {
                let open_tag = &after[..=open_end];
                let href = extract_attr(open_tag, "href").unwrap_or_default();
                let title = extract_attr(open_tag, "title");
                let text = &after[open_end + 1..close_tag];

                result.push_str(&hook(&href, text, title.as_deref()));
                remaining = &after[close_tag + 4..];
            } else {
                result.push_str(full);
                remaining = &after[close_tag + 4..];
            }
        } else {
            result.push_str(&remaining[start..]);
            break;
        }
    }

    result.push_str(remaining);
    result
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let pattern = format!("{name}=\"");
    if let Some(start) = tag.find(&pattern) {
        let after = &tag[start + pattern.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    // Try single quotes
    let pattern = format!("{name}='");
    if let Some(start) = tag.find(&pattern) {
        let after = &tag[start + pattern.len()..];
        if let Some(end) = after.find('\'') {
            return Some(after[..end].to_string());
        }
    }
    None
}

/// Create a responsive image hook that generates `<picture>` elements.
pub fn responsive_image_hook(base_url: &str) -> HookFn {
    let base = base_url.trim_end_matches('/').to_string();
    Box::new(move |src: &str, alt: &str, _title: Option<&str>| {
        // For external URLs, render as normal img
        if src.starts_with("http://") || src.starts_with("https://") {
            return format!("<img src=\"{src}\" alt=\"{alt}\" loading=\"lazy\">");
        }
        // For local images, wrap with picture element for WebP
        let webp_src = if let Some(dot) = src.rfind('.') {
            format!("{}.webp", &src[..dot])
        } else {
            format!("{src}.webp")
        };
        format!(
            "<picture>\
            <source srcset=\"{base}/{webp_src}\" type=\"image/webp\">\
            <img src=\"{base}/{src}\" alt=\"{alt}\" loading=\"lazy\" decoding=\"async\">\
            </picture>"
        )
    })
}

/// Create an external link hook that adds `target="_blank"` and `rel="noopener"`.
pub fn external_link_hook() -> HookFn {
    Box::new(|href: &str, text: &str, _title: Option<&str>| {
        if href.starts_with("http://") || href.starts_with("https://") {
            format!("<a href=\"{href}\" target=\"_blank\" rel=\"noopener noreferrer\">{text}</a>")
        } else {
            format!("<a href=\"{href}\">{text}</a>")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_hook_transforms_images() {
        let hooks = RenderHooks {
            image: Some(Box::new(|src, alt, _title| {
                format!("<picture><img src=\"{src}\" alt=\"{alt}\"></picture>")
            })),
            ..Default::default()
        };

        let html = "<p><img src=\"photo.jpg\" alt=\"A photo\"></p>";
        let result = apply_hooks(html, &hooks);
        assert!(result.contains("<picture>"));
        assert!(result.contains("photo.jpg"));
    }

    #[test]
    fn link_hook_transforms_links() {
        let hooks = RenderHooks {
            link: Some(external_link_hook()),
            ..Default::default()
        };

        let html =
            "<p><a href=\"https://example.com\">Example</a> and <a href=\"/about\">About</a></p>";
        let result = apply_hooks(html, &hooks);
        assert!(result.contains("target=\"_blank\""));
        assert!(result.contains("rel=\"noopener"));
        // Internal link should NOT have target="_blank"
        assert!(result.contains("<a href=\"/about\">About</a>"));
    }

    #[test]
    fn no_hooks_returns_unchanged() {
        let hooks = RenderHooks::default();
        let html = "<p><a href=\"/x\">Link</a></p>";
        assert_eq!(apply_hooks(html, &hooks), html);
    }

    #[test]
    fn responsive_image_hook_generates_picture() {
        let hook = responsive_image_hook("https://example.com");
        let result = hook("images/photo.jpg", "A photo", None);
        assert!(result.contains("<picture>"));
        assert!(result.contains("image/webp"));
        assert!(result.contains("photo.webp"));
        assert!(result.contains("loading=\"lazy\""));
    }

    #[test]
    fn responsive_image_hook_skips_external() {
        let hook = responsive_image_hook("https://example.com");
        let result = hook("https://cdn.example.com/photo.jpg", "External", None);
        assert!(!result.contains("<picture>"));
        assert!(result.contains("<img"));
        assert!(result.contains("loading=\"lazy\""));
    }
}
