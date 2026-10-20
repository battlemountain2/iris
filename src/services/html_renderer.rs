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
    if let Some(raw_html) = html {
        let trimmed = raw_html.trim();
        if !trimmed.is_empty() {
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
                        out.push_str("</i>");
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
                        out.push_str("</a>");
                    } else {
                        let href = extract_attribute(tag_body, "href");
                        if let Some(url) = href {
                            let escaped_url = escape_xml(&url);
                            out.push_str(&format!("<a href=\"{escaped_url}\">"));
                        } else {
                            out.push_str("<a>");
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
                        } else {
                            let label = alt_opt.unwrap_or_else(|| "Image".to_string());
                            let escaped_src = escape_xml(&src);
                            let escaped_label = escape_xml(&label);
                            out.push_str(&format!(
                                " <a href=\"{escaped_src}\"><span foreground=\"#60a5fa\">[🖼️ {escaped_label}]</span></a> "
                            ));
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

fn extract_attribute(tag_body: &str, attr_name: &str) -> Option<String> {
    let needle = format!("{attr_name}=");
    let pos = tag_body.to_lowercase().find(&needle)?;
    let remainder = &tag_body[pos + needle.len()..].trim_start();

    let quote_char = remainder.chars().next()?;
    if quote_char == '"' || quote_char == '\'' {
        let after_quote = &remainder[1..];
        let end_pos = after_quote.find(quote_char)?;
        Some(after_quote[..end_pos].to_string())
    } else {
        let end_pos = remainder
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(remainder.len());
        Some(remainder[..end_pos].to_string())
    }
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
}
