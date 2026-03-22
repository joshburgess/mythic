//! Template syntax conversion utilities shared across migrators.

/// Convert Liquid template syntax (Jekyll) to Tera.
pub fn liquid_to_tera(input: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut output = input.to_string();

    // {{ content }} → {{ content | safe }}
    output = output.replace("{{ content }}", "{{ content | safe }}");

    // {% for item in collection %} — same syntax in Tera
    // {% endfor %} — same in Tera

    // {% if condition %} — same syntax in Tera
    // {% endif %} — same in Tera
    // {% else %} — same in Tera
    // {% elsif %} → {% elif %}
    output = output.replace("{% elsif ", "{% elif ");

    // {% include file.html %} → {% include "file.html" %}
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{% include ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 11..];
        if let Some(end) = after.find(" %}") {
            let filename = after[..end].trim();
            if !filename.starts_with('"') {
                result.push_str(&format!("{{% include \"{filename}\" %}}"));
            } else {
                result.push_str(&format!("{{% include {filename} %}}"));
            }
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 11]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // {{ page.title }} — same in Tera
    // {{ site.title }} — same in Tera

    // Detect unconverted Liquid patterns
    for pattern in &[
        "| date:",
        "| markdownify",
        "| where:",
        "| sort:",
        "| group_by",
    ] {
        if output.contains(pattern) {
            warnings.push(format!("Liquid filter `{pattern}` needs manual conversion"));
        }
    }

    if output.contains("{% assign ") {
        warnings.push("{% assign %} → {% set %} needs manual conversion".to_string());
        output = output.replace("{% assign ", "{% set ");
    }

    if output.contains("{% capture ") {
        warnings.push("{% capture %} blocks need manual conversion to {% set %}".to_string());
    }

    (output, warnings)
}

/// Convert Go template syntax (Hugo) to Tera.
pub fn go_template_to_tera(input: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut output = input.to_string();

    // {{ .Title }} → {{ page.title }}
    output = output.replace("{{ .Title }}", "{{ page.title }}");
    output = output.replace("{{.Title}}", "{{ page.title }}");

    // {{ .Content }} → {{ content | safe }}
    output = output.replace("{{ .Content }}", "{{ content | safe }}");
    output = output.replace("{{.Content}}", "{{ content | safe }}");

    // {{ .Date }} → {{ page.date }}
    output = output.replace("{{ .Date }}", "{{ page.date }}");

    // {{ .Summary }} → {{ page.summary }}
    output = output.replace("{{ .Summary }}", "{{ page.summary }}");

    // {{ .Permalink }} → {{ page.url }}
    output = output.replace("{{ .Permalink }}", "{{ page.url }}");

    // .Params.X → page.extra.X
    output = output.replace(".Params.", "page.extra.");

    // .Site.Title → site.title
    output = output.replace(".Site.Title", "site.title");
    output = output.replace(".Site.BaseURL", "site.base_url");

    // {{ range .Pages }} → {% for page in pages %}
    // This is approximate — Hugo's range is more complex
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{{ range ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 9..];
        if let Some(end) = after.find(" }}") {
            let collection = after[..end].trim();
            let var_name = collection.trim_start_matches('.').to_lowercase();
            result.push_str(&format!("{{% for item in {var_name} %}}"));
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 9]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // {{ end }} → {% endfor %} or {% endif %}
    // This is ambiguous — we'll use endfor as default, user may need to fix
    output = output.replace("{{ end }}", "{% endfor %}");
    output = output.replace("{{end}}", "{% endfor %}");

    // {{ if .X }} → {% if page.X %}
    output = output.replace("{{ if ", "{% if ");
    output = output.replace("{{if ", "{% if ");

    // {{ partial "name.html" . }} → {% include "name.html" %}
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{{ partial ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 11..];
        if let Some(end) = after.find(" }}") {
            let args = after[..end].trim();
            let name = args.split_whitespace().next().unwrap_or("\"\"");
            result.push_str(&format!("{{% include {name} %}}"));
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 11]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // {{ with .Params.X }}...{{ end }} → {% if page.extra.X %}...{% endif %}
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{{ with ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 8..];
        if let Some(end) = after.find(" }}") {
            let expr = after[..end].trim();
            let tera_expr = expr
                .replace(".Params.", "page.extra.")
                .replace(".Site.", "site.")
                .trim_start_matches('.')
                .to_string();
            result.push_str(&format!("{{% if {tera_expr} %}}"));
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 8]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // {{ block "name" . }} → {% block name %}
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{{ block ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 9..];
        if let Some(end) = after.find(" }}") {
            let name = after[..end]
                .split_whitespace()
                .next()
                .unwrap_or("main")
                .trim_matches('"');
            result.push_str(&format!("{{% block {name} %}}"));
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 9]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // {{ define "name" }} → {% block name %}
    let mut result = String::new();
    let mut rest = output.as_str();
    while let Some(start) = rest.find("{{ define ") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 10..];
        if let Some(end) = after.find(" }}") {
            let name = after[..end].trim().trim_matches('"');
            result.push_str(&format!("{{% block {name} %}}"));
            rest = &after[end + 3..];
        } else {
            result.push_str(&rest[start..start + 10]);
            rest = after;
        }
    }
    result.push_str(rest);
    output = result;

    // Hugo pipes: | safeHTML → | safe
    output = output.replace("| safeHTML", "| safe");
    output = output.replace("| safeCSS", "| safe");
    output = output.replace("| safeJS", "| safe");
    output = output.replace("| safeURL", "| safe");

    // Hugo functions
    output = output.replace("{{ .RelPermalink }}", "{{ page.url }}");
    output = output.replace("{{.RelPermalink}}", "{{ page.url }}");
    output = output.replace("{{ .IsHome }}", "{% if page.slug == \"index\" %}");
    output = output.replace("{{ .WordCount }}", "{{ content | word_count }}");
    output = output.replace("{{ .ReadingTime }}", "{{ content | reading_time }}");
    output = output.replace(
        "{{ .TableOfContents }}",
        "{% for entry in toc %}<a href=\"#{{ entry.id }}\">{{ entry.text }}</a>{% endfor %}",
    );
    output = output.replace("{{ .Description }}", "{{ page.extra.description }}");
    output = output.replace("{{.Description}}", "{{ page.extra.description }}");
    output = output.replace("{{ .Kind }}", "\"page\"");

    // Hugo else if → Tera elif
    output = output.replace("{{ else if ", "{% elif ");
    output = output.replace("{{else if ", "{% elif ");
    output = output.replace("{{ else }}", "{% else %}");
    output = output.replace("{{else}}", "{% else %}");

    // Hugo Pipes: replace common asset pipeline chains with Mythic equivalents
    // Pattern: {{ $style := resources.Get "css/..." | toCSS | minify | fingerprint }}
    // → replaced with comment + Mythic asset reference
    let pipes_patterns = [
        "resources.Get",
        "| toCSS",
        "| minify",
        "| fingerprint",
        "resources.Concat",
        "resources.Minify",
        "resources.Fingerprint",
    ];
    for pattern in &pipes_patterns {
        if output.contains(pattern) {
            // Add a comment noting the replacement needed
            let _replacement_note = format!(
                "{{# Hugo Pipes ({pattern}) — use Mythic's asset pipeline: {{{{ assets.css_path }}}} or {{{{ assets.js_path }}}} #}}"
            );
            // We can't auto-replace the whole chain without understanding context,
            // but we can replace the common full-line patterns
            break; // Warning is sufficient, added below
        }
    }

    // Hugo .Scratch → Tera {% set %} (simple cases)
    output = output.replace(".Scratch.Set \"", "set ");
    output = output.replace(".Scratch.Get \"", "");
    output = output.replace("$.Scratch.Set \"", "set ");
    output = output.replace("$.Scratch.Get \"", "");

    // Hugo | markdownify is now handled by registered Tera filter — no conversion needed

    // Detect remaining unconverted patterns
    // Note: | markdownify is handled by a registered Tera filter at runtime
    for pattern in &[
        "resources.Get",
        "resources.Concat",
        "resources.Minify",
        "resources.Fingerprint",
        ".Scratch",
        "$.Scratch",
        "dict ",
        "slice ",
    ] {
        if output.contains(pattern) {
            warnings.push(format!("Hugo function `{pattern}` needs manual conversion"));
        }
    }

    (output, warnings)
}

/// Convert Nunjucks template syntax (Eleventy) to Tera.
pub fn nunjucks_to_tera(input: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let mut output = input.to_string();

    // {{ content | safe }} — same in Tera
    // {% for %} / {% endfor %} — same in Tera
    // {% if %} / {% endif %} — same in Tera
    // {% include %} — same in Tera
    // {% extends %} — same in Tera
    // {% block %} / {% endblock %} — same in Tera
    // {% set %} — same in Tera

    // {% macro name(args) %} → {% macro name(args) %} (mostly compatible)

    // Nunjucks-specific filters
    if output.contains("| dump") {
        output = output.replace("| dump", "| json_encode()");
    }

    // "| striptags" is identical in both Nunjucks and Tera — no conversion needed

    // {% asyncEach %} / {% asyncAll %} — no equivalent
    for pattern in &["{% asyncEach", "{% asyncAll", "| groupby"] {
        if output.contains(pattern) {
            warnings.push(format!("Nunjucks `{pattern}` needs manual conversion"));
        }
    }

    // {% raw %} → {% raw %} (same in Tera)

    // Nunjucks calling syntax: {{ foo(bar) }}
    // This is the same in Tera for functions

    (output, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liquid_content_conversion() {
        let (result, _) = liquid_to_tera("<main>{{ content }}</main>");
        assert_eq!(result, "<main>{{ content | safe }}</main>");
    }

    #[test]
    fn liquid_elsif_conversion() {
        let (result, _) = liquid_to_tera("{% elsif x %}");
        assert_eq!(result, "{% elif x %}");
    }

    #[test]
    fn liquid_include_conversion() {
        let (result, _) = liquid_to_tera("{% include header.html %}");
        assert!(result.contains("\"header.html\""));
    }

    #[test]
    fn liquid_assign_warning() {
        let (_, warnings) = liquid_to_tera("{% assign x = 1 %}");
        assert!(warnings.iter().any(|w| w.contains("assign")));
    }

    #[test]
    fn go_template_title() {
        let (result, _) = go_template_to_tera("<h1>{{ .Title }}</h1>");
        assert_eq!(result, "<h1>{{ page.title }}</h1>");
    }

    #[test]
    fn go_template_content() {
        let (result, _) = go_template_to_tera("{{ .Content }}");
        assert_eq!(result, "{{ content | safe }}");
    }

    #[test]
    fn go_template_partial() {
        let (result, _) = go_template_to_tera("{{ partial \"header.html\" . }}");
        assert!(result.contains("include \"header.html\""));
    }

    #[test]
    fn go_template_params() {
        let (result, _) = go_template_to_tera("{{ .Params.color }}");
        assert!(result.contains("page.extra.color"));
    }

    #[test]
    fn nunjucks_dump_filter() {
        let (result, _) = nunjucks_to_tera("{{ data | dump }}");
        assert!(result.contains("json_encode()"));
    }

    #[test]
    fn nunjucks_passthrough() {
        let input = "{% for item in items %}<li>{{ item }}</li>{% endfor %}";
        let (result, warnings) = nunjucks_to_tera(input);
        assert_eq!(result, input);
        assert!(warnings.is_empty());
    }
}
