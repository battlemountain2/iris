// SPDX-License-Identifier: GPL-3.0-or-later

//! HTML to Pango Markup Renderer & Sanitizer for Iris
//!
//! Converts raw HTML email bodies into secure, clean, and styled Pango markup
//! suitable for rendering in GTK4 Labels or TextBuffers without remote tracking exploits.

/// Renders an email body into valid Pango markup.
///
/// If `html` is provided, it is parsed and sanitized.
/// If only `plain_text` is provided, URLs are auto-linkified and quotes are styled.
pub fn render_body_markup(
    html: Option<&str>,
    plain_text: Option<&str>,
    allow_remote_images: bool,
) -> String {
    // Security limit: 2MB maximum processing size to prevent UI freeze
    const MAX_PARSE_BYTES: usize = 2 * 1024 * 1024;

    if let Some(raw_html) = html {
        let trimmed = raw_html.trim();
        if !trimmed.is_empty() {
            if trimmed.len() > MAX_PARSE_BYTES {
                return format_plain_text_to_pango("⚠️ Message body exceeds maximum size for HTML parsing.\n\n");
            }
            return convert_html_to_pango(trimmed, allow_remote_images);
        }
    }

    if let Some(raw_text) = plain_text {
        return format_plain_text_to_pango(raw_text);
    }

    "<i>(No message body)</i>".to_string()
}

/// Converts an HTML string into valid Pango markup.
pub fn convert_html_to_pango(html: &str, allow_remote_images: bool) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    let mut in_pre = false;
    let mut in_quote = false;
    let mut a_tag_stack: Vec<&'static str> = Vec::new();

    while let Some(c) = chars.next() {
        if c == '<' {
            // Read tag
            let mut tag_content = String::new();
            while let Some(&tc) = chars.peek() {
                chars.next();
                if tc == '>' {
                    break;
                }
                tag_content.push(tc);
            }

            let tag_trimmed = tag_content.trim();
            if tag_trimmed.is_empty() {
                continue;
            }

            let is_closing = tag_trimmed.starts_with('/');
            let tag_body = if is_closing {
                tag_trimmed[1..].trim()
            } else {
                tag_trimmed
            };

            let tag_name = tag_body
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_lowercase();

            match tag_name.as_str() {
                "b" | "strong" => {
                    if is_closing {
                        out.push_str("</b>");
                    } else {
                        out.push_str("<b>");
                    }
                }
                "i" | "em" => {
                    if is_closing {
                        out.push_str("</i>");
                    } else {
                        out.push_str("<i>");
                    }
                }
                "u" => {
                    if is_closing {
                        out.push_str("</u>");
                    } else {
                        out.push_str("<u>");
                    }
                }
                "s" | "strike" | "del" => {
                    if is_closing {
                        out.push_str("</s>");
                    } else {
                        out.push_str("<s>");
                    }
                }
                "h1" | "h2" | "h3" => {
                    if is_closing {
                        out.push_str("</b></span>\n\n");
                    } else {
                        out.push_str("\n\n<span size=\"large\" weight=\"bold\"><b>");
                    }
                }
                "h4" | "h5" | "h6" => {
                    if is_closing {
                        out.push_str("</b></span>\n");
                    } else {
                        out.push_str("\n<span weight=\"bold\"><b>");
                    }
                }
                "p" => {
                    if is_closing {
                        out.push_str("\n\n");
                    } else if !out.is_empty() && !out.ends_with("\n\n") {
                        out.push('\n');
                    }
                }
                "br" => {
                    out.push('\n');
                }
                "hr" => {
                    out.push_str("\n────────────────────────\n");
                }
                "a" => {
                    if is_closing {
                        let closing = a_tag_stack.pop().unwrap_or("a");
                        if closing == "a" {
                            out.push_str("</a>");
                        } else {
                            out.push_str("</span>");
                        }
                    } else {
                        let href = extract_attribute(tag_body, "href");
                        if let Some(url) = href.and_then(|u| sanitize_url(&u)) {
                            let escaped_url = escape_xml(&url);
                            out.push_str(&format!("<a href=\"{escaped_url}\">"));
                            a_tag_stack.push("a");
                        } else {
                            out.push_str("<span>");
                            a_tag_stack.push("span");
                        }
                    }
                }
                "blockquote" => {
                    if is_closing {
                        in_quote = false;
                        out.push_str("</span>\n\n");
                    } else {
                        in_quote = true;
                        out.push_str("\n<span foreground=\"#94a3b8\">  ▎ ");
                    }
                }
                "code" | "tt" => {
                    if is_closing {
                        if !in_pre {
                            out.push_str("</tt></span>");
                        }
                    } else if !in_pre {
                        out.push_str("<span background=\"#141822\"><tt>");
                    }
                }
                "pre" => {
                    if is_closing {
                        in_pre = false;
                        out.push_str("</tt></span>\n\n");
                    } else {
                        in_pre = true;
                        out.push_str("\n<span background=\"#141822\"><tt>");
                    }
                }
                "li" => {
                    if !is_closing {
                        out.push_str("\n  • ");
                    }
                }
                "ul" | "ol" => {
                    if is_closing {
                        out.push('\n');
                    }
                }
                "img" => {
                    let src_opt = extract_attribute(tag_body, "src");
                    let alt_opt = extract_attribute(tag_body, "alt");

                    if let Some(src) = src_opt {
                        let is_remote = src.starts_with("http://") || src.starts_with("https://");
                        if is_remote && !allow_remote_images {
                            out.push_str(
                                " <span foreground=\"#8fa368\">[🖼️ Remote image hidden]</span> ",
                            );
                        } else if let Some(safe_src) = sanitize_url(&src) {
                            let label = alt_opt.unwrap_or_else(|| "Image".to_string());
                            let escaped_src = escape_xml(&safe_src);
                            let escaped_label = escape_xml(&label);
                            out.push_str(&format!(
                                " <a href=\"{escaped_src}\"><span foreground=\"#60a5fa\">[🖼️ {escaped_label}]</span></a> "
                            ));
                        } else {
                            out.push_str(" <span foreground=\"#64748b\">[🖼️ Unsafe Image]</span> ");
                        }
                    }
                }
                _ => {}
            }
        } else if c == '&' {
            // Read HTML entity
            let mut entity = String::new();
            while let Some(&ec) = chars.peek() {
                chars.next();
                if ec == ';' {
                    break;
                }
                entity.push(ec);
                if entity.len() > 10 {
                    break;
                }
            }

            let decoded = decode_html_entity(&entity);
            out.push_str(&escape_xml(&decoded));
        } else {
            // Normal character: escape for Pango
            match c {
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '\n' if in_quote => out.push_str("\n  ▎ "),
                other => out.push(other),
            }
        }
    }

    clean_trailing_newlines(&out)
}

/// Formats plain text into Pango markup with linkification and quote coloring.
pub fn format_plain_text_to_pango(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 100);

    for line in text.lines() {
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with('>') {
            out.push_str("<span foreground=\"#94a3b8\">  ▎ ");
            out.push_str(&linkify_and_escape(trimmed_start));
            out.push_str("</span>\n");
        } else {
            out.push_str(&linkify_and_escape(line));
            out.push('\n');
        }
    }

    clean_trailing_newlines(&out)
}

fn linkify_and_escape(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let words = line.split_whitespace();
    let mut first = true;

    // Preserving initial indentation
    let indent_len = line.len() - line.trim_start().len();
    if indent_len > 0 {
        result.push_str(&line[..indent_len]);
    }

    for word in words {
        if !first {
            result.push(' ');
        }
        first = false;

        if (word.starts_with("http://") || word.starts_with("https://")) && word.len() > 8 {
            let (url, trailing_punct) = split_trailing_punctuation(word);
            let escaped_url = escape_xml(url);
            let escaped_text = escape_xml(url);
            result.push_str(&format!("<a href=\"{escaped_url}\">{escaped_text}</a>"));
            result.push_str(&escape_xml(trailing_punct));
        } else {
            result.push_str(&escape_xml(word));
        }
    }

    result
}

fn split_trailing_punctuation(word: &str) -> (&str, &str) {
    let mut end = word.len();
    for (i, c) in word.char_indices().rev() {
        if c == '.' || c == ',' || c == ';' || c == ':' || c == ')' || c == ']' {
            end = i;
        } else {
            break;
        }
    }
    (&word[..end], &word[end..])
}

/// Validates that a URL uses safe, whitelisted schemes (http, https, mailto).
/// Blocks dangerous protocols such as javascript:, file:, data:, or control characters.
pub fn sanitize_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.chars().any(|c| c.is_control()) {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:") {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Safely extracts an attribute from an HTML tag body using character iteration.
/// Never panics on multi-byte UTF-8 boundaries or malformed tag attributes.
fn extract_attribute(tag_body: &str, attr_name: &str) -> Option<String> {
    let attr_lower = attr_name.to_ascii_lowercase();
    let chars: Vec<char> = tag_body.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Skip whitespace
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        // Read attribute key
        let key_start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '=' && chars[i] != '>' && chars[i] != '/' {
            i += 1;
        }
        let key: String = chars[key_start..i].iter().collect();

        // Skip whitespace before '='
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        if i < chars.len() && chars[i] == '=' {
            i += 1; // skip '='

            // Skip whitespace after '='
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i >= chars.len() {
                break;
            }

            let quote = chars[i];
            let val = if quote == '"' || quote == '\'' {
                i += 1; // skip opening quote
                let val_start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                let s: String = chars[val_start..i].iter().collect();
                if i < chars.len() && chars[i] == quote {
                    i += 1; // skip closing quote
                }
                s
            } else {
                let val_start = i;
                while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '>' {
                    i += 1;
                }
                chars[val_start..i].iter().collect()
            };

            if key.to_ascii_lowercase() == attr_lower {
                return Some(val);
            }
        }
    }

    None
}

fn escape_xml(raw: &str) -> String {
    let mut s = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\'' => s.push_str("&#39;"),
            other => s.push(other),
        }
    }
    s
}

fn decode_html_entity(entity: &str) -> String {
    match entity {
        "nbsp" => " ".to_string(),
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" | "#39" => "'".to_string(),
        "copy" => "©".to_string(),
        "reg" => "®".to_string(),
        "trade" => "™".to_string(),
        "mdash" => "—".to_string(),
        "ndash" => "–".to_string(),
        "bull" => "•".to_string(),
        other if other.starts_with('#') => {
            let num_str = &other[1..];
            if let Ok(code) = num_str.parse::<u32>() {
                if let Some(ch) = char::from_u32(code) {
                    return ch.to_string();
                }
            }
            format!("&{other};")
        }
        other => format!("&{other};"),
    }
}

fn clean_trailing_newlines(s: &str) -> String {
    let trimmed = s.trim_end_matches('\n');
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_bold_and_links() {
        let html = "<p>Hello <b>World</b>! Check <a href=\"https://hermes.dev\">Hermes</a>.</p>";
        let markup = convert_html_to_pango(html, false);
        assert!(markup.contains("<b>World</b>"));
        assert!(markup.contains("<a href=\"https://hermes.dev\">Hermes</a>"));
    }

    #[test]
    fn test_remote_image_privacy_protection() {
        let html = "<p>Track me: <img src=\"https://evil.com/pixel.png\" alt=\"Tracker\" /></p>";

        // Privacy protected: remote image blocked
        let blocked = convert_html_to_pango(html, false);
        assert!(blocked.contains("Remote image hidden"));
        assert!(!blocked.contains("evil.com"));

        // Privacy overridden: remote image unlocked
        let unlocked = convert_html_to_pango(html, true);
        assert!(unlocked.contains("https://evil.com/pixel.png"));
    }

    #[test]
    fn test_plain_text_linkify_and_quotes() {
        let plain = "> This was quoted.\nHere is https://omarchy.org for docs.";
        let markup = format_plain_text_to_pango(plain);
        assert!(markup.contains("▎"));
        assert!(markup.contains("<a href=\"https://omarchy.org\">https://omarchy.org</a>"));
    }

    #[test]
    fn test_html_italics_tag_and_emphasis() {
        let html = "<p><i>Italic text</i> and <em>emphasized text</em></p>";
        let markup = convert_html_to_pango(html, false);
        assert!(markup.contains("<i>Italic text</i>"), "Actual: {markup}");
        assert!(markup.contains("<i>emphasized text</i>"), "Actual: {markup}");
    }

    #[test]
    fn test_unicode_attribute_no_panic() {
        let html = "<a href=\"https://example.com/тест\" title=\"üñîçødé\" data-info=\"🎉\">Link</a>";
        let markup = convert_html_to_pango(html, false);
        assert!(markup.contains("<a href=\"https://example.com/тест\">Link</a>"));
    }

    #[test]
    fn test_unsafe_url_schemes_blocked() {
        let dangerous_html = "<a href=\"javascript:alert(1)\">Attack 1</a> and <a href=\"file:///etc/passwd\">Attack 2</a>";
        let markup = convert_html_to_pango(dangerous_html, false);
        assert!(!markup.contains("href=\"javascript:"));
        assert!(!markup.contains("href=\"file:"));
        assert!(markup.contains("<span>Attack 1</span>"));
        assert!(markup.contains("<span>Attack 2</span>"));
    }
}
